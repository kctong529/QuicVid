#!/usr/bin/env python3
"""Extract strategy-specific recovery-action timing from QuicVid client logs.

The extracted interval uses strategy-specific completion events:

* migration:
    ``automatic_rebind_started`` -> ``migration_confirmed``

* reconnect:
    ``reconnect_started`` -> ``reconnect_completed``

These intervals describe how long each recovery action takes according to its
own completion condition. They are useful diagnostic metrics, but they are not
equivalent end-to-end media disruption measurements across strategies.

Receiver-side metrics such as the largest validated-frame receive gap should be
used as the primary cross-strategy continuity comparison.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Iterable

from scripts.mininet.recovery_analysis import LogEvent, ParsedLog


@dataclass
class RecoveryTiming:
    strategy: str
    start_event: str
    completion_event: str
    recovery_action_started_ms: float | None = None
    recovery_action_completed_ms: float | None = None
    recovery_action_duration_ms: float | None = None
    analysis_errors: list[str] = field(default_factory=list)


def _events_named(events: Iterable[LogEvent], name: str) -> list[LogEvent]:
    return [event for event in events if event.name == name]


def _event_time_ms(event: LogEvent) -> float | None:
    """Read one client-relative event timestamp and normalize it to milliseconds.

    Supported fields:

    * ``elapsed_seconds`` and ``elapsed_s`` are interpreted as seconds;
    * ``elapsed_ms`` and ``timestamp_ms`` are interpreted as milliseconds.

    Wall-clock timestamps are not combined across hosts.
    """

    for key in ("elapsed_seconds", "elapsed_s"):
        value = event.fields.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, (int, float)):
            return float(value) * 1000.0

    for key in ("elapsed_ms", "timestamp_ms"):
        value = event.fields.get(key)
        if isinstance(value, bool):
            continue
        if isinstance(value, (int, float)):
            return float(value)

    return None


def extract_recovery_timing(
    client_log: ParsedLog,
    strategy: str,
) -> RecoveryTiming:
    """Extract one strategy-specific recovery-action interval."""

    if strategy == "migrate":
        start_name = "automatic_rebind_started"
        completion_name = "migration_confirmed"
    elif strategy == "reconnect":
        start_name = "reconnect_started"
        completion_name = "reconnect_completed"
    else:
        return RecoveryTiming(
            strategy=strategy,
            start_event="",
            completion_event="",
            analysis_errors=[f"unsupported recovery strategy: {strategy}"],
        )

    result = RecoveryTiming(
        strategy=strategy,
        start_event=start_name,
        completion_event=completion_name,
    )

    start_events = _events_named(client_log.events, start_name)
    completion_events = _events_named(client_log.events, completion_name)

    if not start_events:
        result.analysis_errors.append(f"missing {start_name}")
        return result
    if len(start_events) > 1:
        result.analysis_errors.append(
            f"expected one {start_name}, found {len(start_events)}"
        )

    if not completion_events:
        result.analysis_errors.append(f"missing {completion_name}")
        return result
    if len(completion_events) > 1:
        result.analysis_errors.append(
            f"expected one {completion_name}, found {len(completion_events)}"
        )

    start = start_events[0]
    completion = next(
        (
            event
            for event in completion_events
            if event.line_number > start.line_number
        ),
        completion_events[0],
    )

    start_ms = _event_time_ms(start)
    completion_ms = _event_time_ms(completion)

    if start_ms is None:
        result.analysis_errors.append(
            f"{start_name} at line {start.line_number} has no supported timestamp"
        )
    if completion_ms is None:
        result.analysis_errors.append(
            f"{completion_name} at line {completion.line_number} "
            "has no supported timestamp"
        )
    if start_ms is None or completion_ms is None:
        return result

    result.recovery_action_started_ms = start_ms
    result.recovery_action_completed_ms = completion_ms

    duration_ms = completion_ms - start_ms
    if duration_ms < 0:
        result.analysis_errors.append(
            f"{completion_name} occurs {abs(duration_ms):.3f} ms before {start_name}"
        )
        return result

    result.recovery_action_duration_ms = duration_ms
    return result
