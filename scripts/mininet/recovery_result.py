#!/usr/bin/env python3
"""Build and serialize one complete QuicVid recovery-run result."""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from scripts.mininet.recovery_analysis import ParsedLog, parse_file
from scripts.mininet.recovery_continuity import FrameContinuity, measure_frame_continuity
from scripts.mininet.recovery_frames import FrameAggregation, aggregate_recovery_frames
from scripts.mininet.recovery_identity import (
    RecoveryRunIdentity,
    extract_recovery_run_identity,
)
from scripts.mininet.recovery_timing import RecoveryTiming, extract_recovery_timing


RESULT_SCHEMA_VERSION = 1


@dataclass
class RecoveryRunResult:
    schema_version: int
    strategy: str | None
    media_run_id: str | None
    successful: bool
    identity: RecoveryRunIdentity
    frames: FrameAggregation
    timing: RecoveryTiming
    continuity: FrameContinuity
    analysis_errors: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


def _prefixed_errors(prefix: str, errors: list[str]) -> list[str]:
    return [f"{prefix}: {error}" for error in errors]


def build_recovery_run_result(
    client_log: ParsedLog,
    server_log: ParsedLog,
) -> RecoveryRunResult:
    """Combine all recovery analyses into one per-run result."""

    identity = extract_recovery_run_identity(client_log, server_log)
    strategy = identity.strategy

    frames = aggregate_recovery_frames(server_log, identity)

    timing_strategy = strategy or "unknown"
    timing = extract_recovery_timing(client_log, timing_strategy)

    continuity = measure_frame_continuity(
        client_log,
        server_log,
        identity,
    )

    analysis_errors: list[str] = []
    analysis_errors.extend(_prefixed_errors("identity", identity.analysis_errors))
    analysis_errors.extend(_prefixed_errors("frames", frames.analysis_errors))
    analysis_errors.extend(_prefixed_errors("timing", timing.analysis_errors))
    analysis_errors.extend(
        _prefixed_errors("continuity", continuity.analysis_errors)
    )

    successful = (
        identity.completed
        and frames.final_frame_exclusive is not None
        and not analysis_errors
    )

    return RecoveryRunResult(
        schema_version=RESULT_SCHEMA_VERSION,
        strategy=strategy,
        media_run_id=identity.media_run_id,
        successful=successful,
        identity=identity,
        frames=frames,
        timing=timing,
        continuity=continuity,
        analysis_errors=analysis_errors,
    )


def write_recovery_run_result(
    result: RecoveryRunResult,
    output_path: str | Path,
) -> None:
    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(result.to_dict(), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Build one structured QuicVid recovery-run result.",
    )
    parser.add_argument("--client-log", required=True)
    parser.add_argument("--server-log", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument(
        "--allow-analysis-errors",
        action="store_true",
        help="write the result and exit successfully even when analysis errors exist",
    )
    return parser


def main() -> int:
    args = _build_parser().parse_args()

    result = build_recovery_run_result(
        parse_file(args.client_log),
        parse_file(args.server_log),
    )
    write_recovery_run_result(result, args.output)

    print(
        "event=recovery_result_written "
        f"strategy={result.strategy} "
        f"media_run={result.media_run_id} "
        f"successful={str(result.successful).lower()} "
        f"analysis_errors={len(result.analysis_errors)} "
        f"output={args.output}"
    )

    if result.analysis_errors and not args.allow_analysis_errors:
        for error in result.analysis_errors:
            print(f"event=recovery_result_error error={json.dumps(error)}")
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
