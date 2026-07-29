#!/usr/bin/env python3
"""Extract run-level identities from parsed QuicVid recovery logs."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Iterable

from scripts.mininet.recovery_analysis import LogEvent, ParsedLog


@dataclass(frozen=True)
class SessionIdentity:
    session_id: str
    connection_id: int | str | None = None
    local_address: str | None = None
    remote_address: str | None = None


@dataclass
class RecoveryRunIdentity:
    strategy: str | None = None
    media_run_id: str | None = None
    expected_frames: int | None = None
    final_frame_exclusive: int | None = None
    sessions: list[SessionIdentity] = field(default_factory=list)
    hello_sessions: list[str] = field(default_factory=list)
    initial_local_address: str | None = None
    recovered_local_address: str | None = None
    completed: bool = False
    reported_session_count: int | None = None
    analysis_errors: list[str] = field(default_factory=list)

    @property
    def session_ids(self) -> list[str]:
        return [session.session_id for session in self.sessions]

    @property
    def connection_ids(self) -> list[int | str]:
        return [
            session.connection_id
            for session in self.sessions
            if session.connection_id is not None
        ]

    @property
    def hello_count(self) -> int:
        return len(self.hello_sessions)

    @property
    def session_count(self) -> int:
        return len(self.sessions)

    @property
    def connection_count(self) -> int:
        return len(dict.fromkeys(self.connection_ids))


def _first_event(events: Iterable[LogEvent], name: str) -> LogEvent | None:
    return next((event for event in events if event.name == name), None)


def _last_event(events: Iterable[LogEvent], name: str) -> LogEvent | None:
    matching = [event for event in events if event.name == name]
    return matching[-1] if matching else None


def _field_str(event: LogEvent | None, *keys: str) -> str | None:
    if event is None:
        return None
    for key in keys:
        value = event.fields.get(key)
        if isinstance(value, str):
            return value
    return None


def _field_int(event: LogEvent | None, *keys: str) -> int | None:
    if event is None:
        return None
    for key in keys:
        value = event.fields.get(key)
        if isinstance(value, int) and not isinstance(value, bool):
            return value
    return None


def _field_identifier(event: LogEvent | None, *keys: str) -> int | str | None:
    if event is None:
        return None
    for key in keys:
        value = event.fields.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, (int, str)):
            return value
    return None


def _deduplicate_preserving_order(values: Iterable[str]) -> list[str]:
    return list(dict.fromkeys(values))


def extract_recovery_run_identity(
    client_log: ParsedLog,
    server_log: ParsedLog | None = None,
) -> RecoveryRunIdentity:
    """Extract stable run and transport identities from client/server events.

    The client log is authoritative for strategy, connection transitions, local
    addresses, and final client completion. The optional server log contributes
    observed HELLO sessions and final-frame metadata when available.
    """

    result = RecoveryRunIdentity()
    client_events = client_log.events
    server_events = server_log.events if server_log is not None else []

    strategy_event = _first_event(client_events, "recovery_strategy_config")
    result.strategy = _field_str(strategy_event, "strategy")

    media_event = _first_event(client_events, "media_run_created")
    result.media_run_id = _field_str(media_event, "media_run", "media_run_id")
    result.expected_frames = _field_int(media_event, "expected_frames")

    connected_events = [
        event for event in client_events if event.name == "connected"
    ]
    sessions: list[SessionIdentity] = []
    seen_sessions: set[str] = set()

    for event in connected_events:
        session_id = _field_str(event, "session", "session_id")
        if session_id is None:
            result.analysis_errors.append(
                f"connected event at line {event.line_number} has no session ID"
            )
            continue
        if session_id in seen_sessions:
            continue
        seen_sessions.add(session_id)
        sessions.append(
            SessionIdentity(
                session_id=session_id,
                connection_id=_field_identifier(
                    event, "connection", "connection_id", "stable_id"
                ),
                local_address=_field_str(event, "local", "local_address"),
                remote_address=_field_str(event, "remote", "remote_address"),
            )
        )

    # Fall back to session_created when a connection event is absent or incomplete.
    for event in client_events:
        if event.name != "session_created":
            continue
        session_id = _field_str(event, "session", "session_id")
        if session_id is None or session_id in seen_sessions:
            continue
        seen_sessions.add(session_id)
        sessions.append(SessionIdentity(session_id=session_id))

    result.sessions = sessions

    client_hello_sessions = [
        session
        for event in client_events
        if event.name == "hello_sent"
        for session in [_field_str(event, "session", "session_id")]
        if session is not None
    ]
    server_hello_sessions = [
        session
        for event in server_events
        if event.name in {"client_hello", "hello_received"}
        for session in [_field_str(event, "session", "session_id")]
        if session is not None
    ]
    result.hello_sessions = _deduplicate_preserving_order(
        server_hello_sessions or client_hello_sessions
    )

    endpoint_events = [
        event
        for event in client_events
        if event.name in {"client_endpoint_created", "endpoint_rebound"}
    ]
    if endpoint_events:
        result.initial_local_address = _field_str(
            endpoint_events[0], "local", "new_local", "bind"
        )

    if result.sessions and result.initial_local_address is None:
        result.initial_local_address = result.sessions[0].local_address

    reconnect_event = _last_event(client_events, "reconnect_completed")
    migration_event = _last_event(client_events, "endpoint_rebound")

    if reconnect_event is not None:
        result.recovered_local_address = _field_str(
            reconnect_event,
            "new_local",
            "local",
            "candidate_local",
        )
    elif migration_event is not None:
        result.recovered_local_address = _field_str(
            migration_event,
            "new_local",
            "local",
        )

    if result.recovered_local_address is None and len(result.sessions) >= 2:
        result.recovered_local_address = result.sessions[-1].local_address

    completion_event = _last_event(client_events, "media_run_completed")
    if completion_event is None:
        completion_event = _last_event(client_events, "client_stopped")

    if completion_event is not None:
        completed_field = completion_event.fields.get("completed")
        result.completed = completed_field is not False
        result.reported_session_count = _field_int(
            completion_event, "sessions", "session_count"
        )

    done_event = next(
        (
            event
            for event in reversed(server_events)
            if event.name
            in {
                "jpeg_video_done",
                "media_run_completed",
                "done_received",
            }
        ),
        None,
    )
    result.final_frame_exclusive = _field_int(
        done_event,
        "final_frame_exclusive",
        "expected_frames",
    )
    if result.final_frame_exclusive is None:
        result.final_frame_exclusive = _field_int(
            completion_event,
            "final_frame_exclusive",
            "expected_frames",
        )
    if result.final_frame_exclusive is None:
        result.final_frame_exclusive = result.expected_frames

    _validate_identity(result)
    return result


def _validate_identity(result: RecoveryRunIdentity) -> None:
    if result.strategy not in {"migrate", "reconnect"}:
        result.analysis_errors.append("missing or unsupported recovery strategy")

    if result.media_run_id is None:
        result.analysis_errors.append("missing media_run_id")

    if result.expected_frames is None:
        result.analysis_errors.append("missing expected_frames")

    if result.session_count == 0:
        result.analysis_errors.append("no transport sessions found")

    if result.hello_count == 0:
        result.analysis_errors.append("no HELLO sessions found")

    if result.strategy == "migrate":
        if result.session_count != 1:
            result.analysis_errors.append(
                f"migration expected 1 session, found {result.session_count}"
            )
        if result.connection_count != 1:
            result.analysis_errors.append(
                f"migration expected 1 connection, found {result.connection_count}"
            )
        if result.hello_count != 1:
            result.analysis_errors.append(
                f"migration expected 1 HELLO, found {result.hello_count}"
            )

    if result.strategy == "reconnect":
        if result.session_count < 2:
            result.analysis_errors.append(
                f"reconnect expected at least 2 sessions, found {result.session_count}"
            )
        if result.connection_count < 2:
            result.analysis_errors.append(
                "reconnect expected at least 2 distinct connections, "
                f"found {result.connection_count}"
            )
        if result.hello_count < 2:
            result.analysis_errors.append(
                f"reconnect expected at least 2 HELLOs, found {result.hello_count}"
            )
