#!/usr/bin/env python3
"""Measure frame continuity around migration or reconnect recovery."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Iterable

from scripts.mininet.recovery_analysis import LogEvent, ParsedLog
from scripts.mininet.recovery_identity import RecoveryRunIdentity


FRAME_EVENT_NAME = "jpeg_frame_validated"
FRAME_ID_FIELDS = ("frame", "frame_id", "frame_index")
SESSION_FIELDS = ("session", "session_id")
MEDIA_RUN_FIELDS = ("media_run", "media_run_id")
RECEIVE_TIME_FIELDS = ("received_at_ms", "elapsed_ms", "timestamp_ms")


@dataclass
class FrameContinuity:
    strategy: str
    media_run_id: str | None
    last_frame_before_recovery: int | None = None
    first_frame_after_recovery: int | None = None
    skipped_frames: int | None = None
    largest_frame_id_gap: int = 0
    largest_frame_id_gap_start: int | None = None
    largest_frame_id_gap_end: int | None = None
    out_of_order_frames: int = 0
    largest_receive_gap_ms: float | None = None
    receive_gap_sample_count: int = 0
    analysis_errors: list[str] = field(default_factory=list)


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


def _field_number(event: LogEvent, keys: Iterable[str]) -> float | None:
    for key in keys:
        value = event.fields.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, (int, float)):
            return float(value)
    return None


def _belongs_to_run(
    event: LogEvent,
    identity: RecoveryRunIdentity,
) -> bool:
    event_media_run = _field_str(event, MEDIA_RUN_FIELDS)
    event_session = _field_str(event, SESSION_FIELDS)

    if event_media_run is not None:
        return event_media_run == identity.media_run_id

    return event_session is not None and event_session in set(identity.session_ids)


def _validated_frame_events(
    server_log: ParsedLog,
    identity: RecoveryRunIdentity,
) -> list[LogEvent]:
    return [
        event
        for event in server_log.events
        if event.name == FRAME_EVENT_NAME and _belongs_to_run(event, identity)
    ]


def _measure_frame_id_continuity(
    events: list[LogEvent],
    result: FrameContinuity,
) -> None:
    ordered_frame_ids: list[int] = []

    for event in events:
        frame_id = _field_int(event, FRAME_ID_FIELDS)
        if frame_id is None:
            result.analysis_errors.append(
                f"{FRAME_EVENT_NAME} at line {event.line_number} "
                "has no integer frame ID"
            )
            continue
        ordered_frame_ids.append(frame_id)

    if not ordered_frame_ids:
        result.analysis_errors.append("no validated frames found for media run")
        return

    previous = ordered_frame_ids[0]
    for current in ordered_frame_ids[1:]:
        if current < previous:
            result.out_of_order_frames += 1
        elif current > previous + 1:
            missing_between = current - previous - 1
            if missing_between > result.largest_frame_id_gap:
                result.largest_frame_id_gap = missing_between
                result.largest_frame_id_gap_start = previous + 1
                result.largest_frame_id_gap_end = current - 1
        previous = current


def _measure_receive_gap(
    events: list[LogEvent],
    result: FrameContinuity,
) -> None:
    receive_times: list[float] = []

    for event in events:
        receive_time = _field_number(event, RECEIVE_TIME_FIELDS)
        if receive_time is not None:
            receive_times.append(receive_time)

    result.receive_gap_sample_count = len(receive_times)

    if len(receive_times) < 2:
        result.analysis_errors.append(
            "largest_receive_gap_ms unavailable: validated frame events "
            "do not contain at least two receiver-side timestamps"
        )
        return

    largest_gap = 0.0
    for previous, current in zip(receive_times, receive_times[1:]):
        gap = current - previous
        if gap < 0:
            # Receive order is still measured separately through frame IDs.
            continue
        largest_gap = max(largest_gap, gap)

    result.largest_receive_gap_ms = largest_gap


def _extract_reconnect_boundary(
    client_log: ParsedLog,
    result: FrameContinuity,
) -> None:
    events = client_log.events_named("media_resumed_after_reconnect")
    if not events:
        result.analysis_errors.append("missing media_resumed_after_reconnect")
        return

    if len(events) > 1:
        result.analysis_errors.append(
            "expected one media_resumed_after_reconnect, "
            f"found {len(events)}"
        )

    event = events[0]
    result.last_frame_before_recovery = _field_int(
        event,
        (
            "last_frame_before_reconnect",
            "last_frame_before_recovery",
            "last_frame",
        ),
    )
    result.first_frame_after_recovery = _field_int(
        event,
        (
            "first_frame_after_reconnect",
            "first_frame_after_recovery",
            "first_frame",
        ),
    )
    result.skipped_frames = _field_int(
        event,
        ("skipped_frames", "frames_skipped"),
    )

    if result.last_frame_before_recovery is None:
        result.analysis_errors.append(
            "media_resumed_after_reconnect has no last pre-reconnect frame"
        )
    if result.first_frame_after_recovery is None:
        result.analysis_errors.append(
            "media_resumed_after_reconnect has no first post-reconnect frame"
        )

    if (
        result.last_frame_before_recovery is not None
        and result.first_frame_after_recovery is not None
    ):
        if result.first_frame_after_recovery <= result.last_frame_before_recovery:
            result.analysis_errors.append(
                "first post-reconnect frame does not advance the media timeline"
            )

        derived_skipped = max(
            0,
            result.first_frame_after_recovery
            - result.last_frame_before_recovery
            - 1,
        )
        if result.skipped_frames is None:
            result.skipped_frames = derived_skipped
        elif result.skipped_frames != derived_skipped:
            result.analysis_errors.append(
                "reported skipped_frames does not match frame boundary: "
                f"reported={result.skipped_frames} derived={derived_skipped}"
            )


def measure_frame_continuity(
    client_log: ParsedLog,
    server_log: ParsedLog,
    identity: RecoveryRunIdentity,
) -> FrameContinuity:
    """Measure strategy-neutral and reconnect-specific frame continuity."""

    result = FrameContinuity(
        strategy=identity.strategy or "unknown",
        media_run_id=identity.media_run_id,
    )

    events = _validated_frame_events(server_log, identity)
    _measure_frame_id_continuity(events, result)
    _measure_receive_gap(events, result)

    if identity.strategy == "reconnect":
        _extract_reconnect_boundary(client_log, result)

    return result
