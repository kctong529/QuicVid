#!/usr/bin/env python3
"""Aggregate received media frames across migration or reconnect sessions."""

from __future__ import annotations

from collections import Counter, defaultdict
from dataclasses import dataclass, field
from typing import Iterable

from scripts.mininet.recovery_analysis import LogEvent, ParsedLog
from scripts.mininet.recovery_identity import RecoveryRunIdentity


DEFAULT_FRAME_EVENT_NAMES = frozenset(
    {
        "jpeg_frame_validated",
    }
)

FRAME_ID_FIELDS = ("frame", "frame_id", "frame_index")
SESSION_FIELDS = ("session", "session_id")
MEDIA_RUN_FIELDS = ("media_run", "media_run_id")


@dataclass
class FrameAggregation:
    media_run_id: str | None
    final_frame_exclusive: int | None
    received_frame_ids: list[int] = field(default_factory=list)
    received_unique_frames: int = 0
    missing_frame_ids: list[int] = field(default_factory=list)
    duplicate_frame_ids: list[int] = field(default_factory=list)
    duplicate_frames: int = 0
    per_session_received: dict[str, int] = field(default_factory=dict)
    ignored_frame_events: int = 0
    analysis_errors: list[str] = field(default_factory=list)

    @property
    def expected_frames(self) -> int | None:
        return self.final_frame_exclusive

    @property
    def missing_frames(self) -> int | None:
        if self.final_frame_exclusive is None:
            return None
        return len(self.missing_frame_ids)


def _field_str(event: LogEvent, keys: Iterable[str]) -> str | None:
    for key in keys:
        value = event.fields.get(key)
        if isinstance(value, str):
            return value
    return None


def _field_int(event: LogEvent, keys: Iterable[str]) -> int | None:
    for key in keys:
        value = event.fields.get(key)
        if isinstance(value, int) and not isinstance(value, bool):
            return value
    return None


def aggregate_recovery_frames(
    server_log: ParsedLog,
    identity: RecoveryRunIdentity,
    *,
    frame_event_names: frozenset[str] = DEFAULT_FRAME_EVENT_NAMES,
) -> FrameAggregation:
    """Aggregate complete frame IDs across every session in one media run.

    Only events whose name appears in ``frame_event_names`` are considered.
    Events are accepted when either:

    * their media-run ID matches the target run; or
    * they have no media-run field but their session belongs to the run.

    The final session's local summary is deliberately ignored. Global missing
    frames are calculated from the union of all accepted frame IDs.
    """

    result = FrameAggregation(
        media_run_id=identity.media_run_id,
        final_frame_exclusive=identity.final_frame_exclusive,
    )

    if identity.media_run_id is None:
        result.analysis_errors.append("cannot aggregate frames without media_run_id")
        return result

    valid_sessions = set(identity.session_ids)
    occurrences: Counter[int] = Counter()
    per_session: dict[str, set[int]] = defaultdict(set)

    for event in server_log.events:
        if event.name not in frame_event_names:
            continue

        frame_id = _field_int(event, FRAME_ID_FIELDS)
        if frame_id is None:
            result.ignored_frame_events += 1
            result.analysis_errors.append(
                f"{event.name} at line {event.line_number} has no integer frame ID"
            )
            continue

        event_media_run = _field_str(event, MEDIA_RUN_FIELDS)
        event_session = _field_str(event, SESSION_FIELDS)

        belongs_to_run = event_media_run == identity.media_run_id
        belongs_to_session = (
            event_media_run is None
            and event_session is not None
            and event_session in valid_sessions
        )

        if not belongs_to_run and not belongs_to_session:
            result.ignored_frame_events += 1
            continue

        if frame_id < 0:
            result.ignored_frame_events += 1
            result.analysis_errors.append(
                f"{event.name} at line {event.line_number} has negative frame ID "
                f"{frame_id}"
            )
            continue

        occurrences[frame_id] += 1
        if event_session is not None:
            per_session[event_session].add(frame_id)

    result.received_frame_ids = sorted(occurrences)
    result.received_unique_frames = len(result.received_frame_ids)
    result.duplicate_frame_ids = sorted(
        frame_id for frame_id, count in occurrences.items() if count > 1
    )
    result.duplicate_frames = sum(count - 1 for count in occurrences.values())
    result.per_session_received = {
        session: len(frame_ids)
        for session, frame_ids in sorted(per_session.items())
    }

    if result.final_frame_exclusive is None:
        result.analysis_errors.append(
            "cannot calculate missing frames without final_frame_exclusive"
        )
        return result

    if result.final_frame_exclusive < 0:
        result.analysis_errors.append("final_frame_exclusive must not be negative")
        return result

    expected = set(range(result.final_frame_exclusive))
    received_in_range = {
        frame_id
        for frame_id in occurrences
        if frame_id < result.final_frame_exclusive
    }
    result.missing_frame_ids = sorted(expected - received_in_range)

    out_of_range = sorted(
        frame_id
        for frame_id in occurrences
        if frame_id >= result.final_frame_exclusive
    )
    if out_of_range:
        result.analysis_errors.append(
            "received frame IDs outside final range: "
            + ", ".join(str(frame_id) for frame_id in out_of_range)
        )

    return result
