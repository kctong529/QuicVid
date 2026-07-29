#!/usr/bin/env python3
"""Parse QuicVid structured recovery logs."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
import math
import shlex
from typing import Iterable

Scalar = str | int | float | bool | None


@dataclass(frozen=True)
class LogEvent:
    name: str
    fields: dict[str, Scalar]
    line_number: int
    raw: str

    def get_str(self, key: str) -> str | None:
        value = self.fields.get(key)
        return value if isinstance(value, str) else None

    def get_int(self, key: str) -> int | None:
        value = self.fields.get(key)
        return value if isinstance(value, int) and not isinstance(value, bool) else None

    def get_float(self, key: str) -> float | None:
        value = self.fields.get(key)
        if isinstance(value, bool):
            return None
        return float(value) if isinstance(value, (int, float)) else None


@dataclass(frozen=True)
class MalformedEvent:
    line_number: int
    raw: str
    reason: str


@dataclass
class ParsedLog:
    events: list[LogEvent] = field(default_factory=list)
    malformed: list[MalformedEvent] = field(default_factory=list)

    def events_named(self, name: str) -> list[LogEvent]:
        return [event for event in self.events if event.name == name]

    def require_clean(self) -> None:
        if not self.malformed:
            return
        details = "\n".join(
            f"line {item.line_number}: {item.reason}: {item.raw}"
            for item in self.malformed
        )
        raise ValueError(f"malformed structured log events:\n{details}")


def _parse_scalar(raw: str) -> Scalar:
    lowered = raw.lower()
    if lowered == "true":
        return True
    if lowered == "false":
        return False
    if lowered in {"null", "none"}:
        return None

    signed = raw[1:] if raw.startswith(("+", "-")) else raw
    if signed.isdigit() and (signed == "0" or not signed.startswith("0")):
        try:
            return int(raw)
        except ValueError:
            pass

    try:
        value = float(raw)
        if math.isfinite(value) and any(char in raw for char in ".eE"):
            return value
    except ValueError:
        pass

    return raw


def parse_line(line: str, line_number: int) -> LogEvent | MalformedEvent | None:
    stripped = line.strip()
    if not stripped or "event=" not in stripped:
        return None

    try:
        tokens = shlex.split(stripped, comments=False, posix=True)
    except ValueError as exc:
        return MalformedEvent(line_number, stripped, f"tokenization failed ({exc})")

    event_index = next(
        (index for index, token in enumerate(tokens) if token.startswith("event=")),
        None,
    )
    if event_index is None:
        return None

    fields: dict[str, Scalar] = {}
    for token in tokens[event_index:]:
        if "=" not in token:
            return MalformedEvent(
                line_number, stripped, f"structured token has no '=': {token!r}"
            )

        key, raw_value = token.split("=", 1)
        if not key:
            return MalformedEvent(line_number, stripped, "empty field name")
        if key in fields:
            return MalformedEvent(line_number, stripped, f"duplicate field: {key}")
        fields[key] = _parse_scalar(raw_value)

    event_name = fields.pop("event", None)
    if not isinstance(event_name, str) or not event_name:
        return MalformedEvent(line_number, stripped, "event name is missing or empty")

    return LogEvent(event_name, fields, line_number, stripped)


def parse_lines(lines: Iterable[str]) -> ParsedLog:
    parsed = ParsedLog()
    for line_number, line in enumerate(lines, start=1):
        result = parse_line(line, line_number)
        if isinstance(result, LogEvent):
            parsed.events.append(result)
        elif isinstance(result, MalformedEvent):
            parsed.malformed.append(result)
    return parsed


def parse_text(text: str) -> ParsedLog:
    return parse_lines(text.splitlines())


def parse_file(path: str | Path) -> ParsedLog:
    with Path(path).open("r", encoding="utf-8", errors="replace") as handle:
        return parse_lines(handle)
