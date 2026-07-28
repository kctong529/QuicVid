#!/usr/bin/env python3
"""Verify QuicVid migration or reconnect evidence from launcher logs."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


EVENT_RE = re.compile(r"(?:^|\s)event=([^\s]+)")
FIELD_RE = re.compile(r"(?:^|\s)([A-Za-z_][A-Za-z0-9_]*)=([^\s]+)")


@dataclass(frozen=True)
class Event:
    name: str
    fields: dict[str, str]
    raw: str


def parse_events(path: Path) -> list[Event]:
    events: list[Event] = []
    for raw_line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = EVENT_RE.search(raw_line)
        if not match:
            continue
        fields = {key: value for key, value in FIELD_RE.findall(raw_line)}
        events.append(Event(match.group(1), fields, raw_line))
    return events


def find_all(events: Iterable[Event], name: str) -> list[Event]:
    return [event for event in events if event.name == name]


def require(condition: bool, message: str, errors: list[str]) -> None:
    if not condition:
        errors.append(message)


def parse_int(event: Event, field: str, errors: list[str]) -> int | None:
    value = event.fields.get(field)
    if value is None:
        errors.append(f"{event.name} is missing {field}")
        return None
    try:
        return int(value)
    except ValueError:
        errors.append(f"{event.name} has invalid {field}={value!r}")
        return None


def verify_reconnect(client: list[Event], server: list[Event]) -> list[str]:
    errors: list[str] = []

    configs = find_all(client, "recovery_strategy_config")
    require(any(e.fields.get("strategy") == "reconnect" for e in configs),
            "client did not select strategy=reconnect", errors)

    created = find_all(client, "media_run_created")
    require(len(created) == 1, f"expected one media_run_created, got {len(created)}", errors)
    media_run = created[0].fields.get("media_run") if created else None

    reconnects = find_all(client, "reconnect_completed")
    require(len(reconnects) == 1, f"expected one reconnect_completed, got {len(reconnects)}", errors)
    reconnect = reconnects[0] if reconnects else None

    if reconnect is not None:
        old_session = reconnect.fields.get("old_session")
        new_session = reconnect.fields.get("new_session")
        old_connection = reconnect.fields.get("old_connection")
        new_connection = reconnect.fields.get("new_connection")
        old_local = reconnect.fields.get("old_local", "")
        new_local = reconnect.fields.get("new_local", "")

        require(old_session is not None and new_session is not None and old_session != new_session,
                "reconnect did not replace the session ID", errors)
        require(old_connection is not None and new_connection is not None and old_connection != new_connection,
                "reconnect did not replace the Quinn connection", errors)
        require(old_local.startswith("10.0.1.2:"),
                f"expected old Path A address, got {old_local!r}", errors)
        require(new_local.startswith("10.0.2.2:"),
                f"expected new Path B address, got {new_local!r}", errors)
        require(media_run is None or reconnect.fields.get("media_run") == media_run,
                "reconnect_completed changed the media_run ID", errors)

    resumes = find_all(client, "media_resumed_after_reconnect")
    require(len(resumes) == 1,
            f"expected one media_resumed_after_reconnect, got {len(resumes)}", errors)
    if resumes:
        resume = resumes[0]
        last_frame = parse_int(resume, "last_frame_before_reconnect", errors)
        first_frame = parse_int(resume, "first_frame_after_reconnect", errors)
        if last_frame is not None and first_frame is not None:
            require(first_frame > last_frame,
                    f"timeline did not advance: last={last_frame}, first={first_frame}", errors)
            require(first_frame != 0, "replacement session replayed frame 0", errors)
        require(media_run is None or resume.fields.get("media_run") == media_run,
                "media_resumed_after_reconnect changed the media_run ID", errors)

    completions = find_all(client, "media_run_completed")
    require(len(completions) == 1, f"expected one media_run_completed, got {len(completions)}", errors)
    if completions:
        sessions = parse_int(completions[0], "sessions", errors)
        require(sessions == 2, f"expected sessions=2, got {sessions}", errors)
        require(media_run is None or completions[0].fields.get("media_run") == media_run,
                "media_run_completed changed the media_run ID", errors)

    server_hellos = find_all(server, "client_hello")
    matching_hellos = [e for e in server_hellos if media_run is None or e.fields.get("media_run") == media_run]
    require(len(matching_hellos) == 2,
            f"expected two server HELLOs for the media run, got {len(matching_hellos)}", errors)
    if len(matching_hellos) >= 2:
        sessions = {e.fields.get("session") for e in matching_hellos}
        connections = {e.fields.get("connection") for e in matching_hellos}
        peers = {e.fields.get("peer", "") for e in matching_hellos}
        require(len(sessions) == 2, "server HELLOs did not use two session IDs", errors)
        require(len(connections) == 2, "server HELLOs did not use two connection IDs", errors)
        require(any(peer.startswith("10.0.1.2:") for peer in peers),
                "server did not observe Path A", errors)
        require(any(peer.startswith("10.0.2.2:") for peer in peers),
                "server did not observe Path B", errors)

    done = find_all(server, "jpeg_video_done")
    require(len(done) == 1, f"expected one server jpeg_video_done, got {len(done)}", errors)
    if done and reconnect is not None:
        require(done[0].fields.get("session") == reconnect.fields.get("new_session"),
                "DONE was not sent by the replacement session", errors)
        require(media_run is None or done[0].fields.get("media_run") == media_run,
                "server DONE changed the media_run ID", errors)

    require(not find_all(client, "endpoint_rebound"),
            "reconnect run unexpectedly rebound the old endpoint", errors)
    require(not find_all(client, "migration_confirmed"),
            "reconnect run unexpectedly reported migration_confirmed", errors)

    return errors


def verify_migrate(client: list[Event], server: list[Event]) -> list[str]:
    errors: list[str] = []

    configs = find_all(client, "recovery_strategy_config")
    require(any(e.fields.get("strategy") == "migrate" for e in configs),
            "client did not select strategy=migrate", errors)
    require(len(find_all(client, "migration_confirmed")) == 1,
            "expected one migration_confirmed event", errors)
    require(len(find_all(client, "endpoint_rebound")) == 1,
            "expected one endpoint_rebound event", errors)
    require(not find_all(client, "reconnect_completed"),
            "migration run unexpectedly reconnected", errors)
    require(not find_all(client, "media_resumed_after_reconnect"),
            "migration run unexpectedly logged reconnect resumption", errors)

    sessions = find_all(client, "session_created")
    hellos = find_all(client, "hello_sent")
    require(len(sessions) == 1, f"expected one client session, got {len(sessions)}", errors)
    require(len(hellos) == 1, f"expected one client HELLO, got {len(hellos)}", errors)

    completions = find_all(client, "media_run_completed")
    require(len(completions) == 1, f"expected one media_run_completed, got {len(completions)}", errors)
    if completions:
        count = parse_int(completions[0], "sessions", errors)
        require(count == 1, f"expected sessions=1, got {count}", errors)

    server_hellos = find_all(server, "client_hello")
    require(len(server_hellos) == 1, f"expected one server HELLO, got {len(server_hellos)}", errors)
    require(len(find_all(server, "jpeg_video_done")) == 1,
            "expected one server jpeg_video_done", errors)

    return errors


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strategy", choices=("migrate", "reconnect"), required=True)
    parser.add_argument("--client-log", type=Path, required=True)
    parser.add_argument("--server-log", type=Path, required=True)
    return parser


def main() -> int:
    args = build_parser().parse_args()

    for path in (args.client_log, args.server_log):
        if not path.is_file():
            print(f"error: log file does not exist: {path}", file=sys.stderr)
            return 2

    client = parse_events(args.client_log)
    server = parse_events(args.server_log)

    errors = (
        verify_reconnect(client, server)
        if args.strategy == "reconnect"
        else verify_migrate(client, server)
    )

    if errors:
        print(f"FAIL: {args.strategy} verification failed", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"PASS: {args.strategy} recovery evidence is consistent")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
