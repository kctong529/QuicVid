#!/usr/bin/env python3
"""Export flat recovery experiment rows from per-run JSON results."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any, Iterable


CSV_FIELDS = [
    "schema_version",
    "trial_id",
    "strategy",
    "media_run_id",
    "successful",
    "analysis_error_count",
    "session_count",
    "connection_count",
    "expected_frames",
    "received_unique_frames",
    "missing_frame_count",
    "duplicate_frame_count",
    "largest_frame_id_gap",
    "out_of_order_frames",
    "largest_receive_gap_ms",
    "recovery_action_duration_ms",
]


def _require_mapping(value: Any, field: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{field} must be an object")
    return value


def flatten_recovery_result(
    result: dict[str, Any],
    *,
    trial_id: str,
) -> dict[str, Any]:
    """Convert one versioned recovery result into a stable CSV row."""

    identity = _require_mapping(result.get("identity"), "identity")
    frames = _require_mapping(result.get("frames"), "frames")
    timing = _require_mapping(result.get("timing"), "timing")
    continuity = _require_mapping(result.get("continuity"), "continuity")

    analysis_errors = result.get("analysis_errors")
    if not isinstance(analysis_errors, list):
        raise ValueError("analysis_errors must be an array")

    sessions = identity.get("sessions")
    if not isinstance(sessions, list):
        raise ValueError("identity.sessions must be an array")

    connection_ids = {
        session.get("connection_id")
        for session in sessions
        if isinstance(session, dict) and session.get("connection_id") is not None
    }

    missing_frame_ids = frames.get("missing_frame_ids")
    if not isinstance(missing_frame_ids, list):
        raise ValueError("frames.missing_frame_ids must be an array")

    return {
        "schema_version": result.get("schema_version"),
        "trial_id": trial_id,
        "strategy": result.get("strategy"),
        "media_run_id": result.get("media_run_id"),
        "successful": result.get("successful"),
        "analysis_error_count": len(analysis_errors),
        "session_count": len(sessions),
        "connection_count": len(connection_ids),
        "expected_frames": identity.get("expected_frames"),
        "received_unique_frames": frames.get("received_unique_frames"),
        "missing_frame_count": len(missing_frame_ids),
        "duplicate_frame_count": frames.get("duplicate_frames"),
        "largest_frame_id_gap": continuity.get("largest_frame_id_gap"),
        "out_of_order_frames": continuity.get("out_of_order_frames"),
        "largest_receive_gap_ms": continuity.get("largest_receive_gap_ms"),
        "recovery_action_duration_ms": timing.get(
            "recovery_action_duration_ms"
        ),
    }


def read_result(path: str | Path) -> dict[str, Any]:
    value = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: result JSON must contain an object")
    return value


def write_summary_csv(
    rows: Iterable[dict[str, Any]],
    output_path: str | Path,
) -> None:
    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)

    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=CSV_FIELDS)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Export recovery result JSON files to one flat CSV.",
    )
    parser.add_argument(
        "results",
        nargs="+",
        help="per-run result JSON files",
    )
    parser.add_argument("--output", required=True)
    return parser


def main() -> int:
    args = _build_parser().parse_args()

    rows: list[dict[str, Any]] = []
    for index, result_path in enumerate(args.results, start=1):
        path = Path(result_path)
        trial_id = path.parent.name or f"trial-{index}"
        rows.append(
            flatten_recovery_result(
                read_result(path),
                trial_id=trial_id,
            )
        )

    write_summary_csv(rows, args.output)

    print(
        "event=recovery_summary_written "
        f"runs={len(rows)} output={args.output}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
