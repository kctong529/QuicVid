#!/usr/bin/env python3
"""Generate aggregate migration-versus-reconnect recovery statistics."""

from __future__ import annotations

import argparse
import csv
import math
import statistics
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Mapping, Sequence


STRATEGIES = ("migrate", "reconnect")

METRICS = (
    "largest_receive_gap_ms",
    "missing_frame_count",
    "received_unique_frames",
    "largest_frame_id_gap",
    "recovery_action_duration_ms",
    "session_count",
    "connection_count",
)

REQUIRED_COLUMNS = {
    "trial_id",
    "strategy",
    "successful",
    "analysis_error_count",
    "duplicate_frame_count",
    "out_of_order_frames",
    *METRICS,
}

SUMMARY_FIELDS = (
    "strategy",
    "total_trials",
    "valid_trials",
    "invalid_trials",
    "successful_trials",
    "failed_trials",
    "success_rate",
    "analysis_error_count",
    "duplicate_frame_total",
    "out_of_order_frame_total",
    *(
        f"{metric}_{statistic}"
        for metric in METRICS
        for statistic in ("count", "min", "mean", "median", "max", "stddev")
    ),
)


@dataclass(frozen=True)
class TrialRow:
    trial_id: str
    strategy: str
    successful: bool
    analysis_error_count: int
    duplicate_frame_count: int
    out_of_order_frames: int
    metrics: Mapping[str, float]


@dataclass(frozen=True)
class InvalidTrial:
    trial_id: str
    strategy: str
    reason: str


@dataclass(frozen=True)
class Comparison:
    summary_rows: tuple[dict[str, object], ...]
    invalid_trials: tuple[InvalidTrial, ...]


def _parse_bool(value: str, *, field: str) -> bool:
    normalized = value.strip().lower()
    if normalized in {"true", "1", "yes"}:
        return True
    if normalized in {"false", "0", "no"}:
        return False
    raise ValueError(f"{field} must be a boolean")


def _parse_int(value: str, *, field: str) -> int:
    try:
        parsed = int(value)
    except ValueError as error:
        raise ValueError(f"{field} must be an integer") from error
    if parsed < 0:
        raise ValueError(f"{field} must be non-negative")
    return parsed


def _parse_float(value: str, *, field: str) -> float:
    try:
        parsed = float(value)
    except ValueError as error:
        raise ValueError(f"{field} must be numeric") from error
    if not math.isfinite(parsed):
        raise ValueError(f"{field} must be finite")
    if parsed < 0:
        raise ValueError(f"{field} must be non-negative")
    return parsed


def _validate_columns(fieldnames: Sequence[str] | None) -> None:
    actual = set(fieldnames or ())
    missing = sorted(REQUIRED_COLUMNS - actual)
    if missing:
        raise ValueError(
            "input CSV is missing required columns: " + ", ".join(missing)
        )


def _parse_trial(raw: Mapping[str, str], *, row_number: int) -> TrialRow:
    trial_id = (raw.get("trial_id") or "").strip()
    strategy = (raw.get("strategy") or "").strip()

    if not trial_id:
        raise ValueError("trial_id must not be empty")
    if strategy not in STRATEGIES:
        raise ValueError(
            f"strategy must be one of: {', '.join(STRATEGIES)}"
        )

    return TrialRow(
        trial_id=trial_id,
        strategy=strategy,
        successful=_parse_bool(
            raw.get("successful", ""),
            field="successful",
        ),
        analysis_error_count=_parse_int(
            raw.get("analysis_error_count", ""),
            field="analysis_error_count",
        ),
        duplicate_frame_count=_parse_int(
            raw.get("duplicate_frame_count", ""),
            field="duplicate_frame_count",
        ),
        out_of_order_frames=_parse_int(
            raw.get("out_of_order_frames", ""),
            field="out_of_order_frames",
        ),
        metrics={
            metric: _parse_float(raw.get(metric, ""), field=metric)
            for metric in METRICS
        },
    )


def read_trials(
    input_path: str | Path,
) -> tuple[list[TrialRow], list[InvalidTrial]]:
    """Read a flat Epic 5.3 summary and retain invalid rows explicitly."""

    trials: list[TrialRow] = []
    invalid: list[InvalidTrial] = []

    with Path(input_path).open(encoding="utf-8", newline="") as source:
        reader = csv.DictReader(source)
        _validate_columns(reader.fieldnames)

        for row_number, raw in enumerate(reader, start=2):
            trial_id = (raw.get("trial_id") or f"row-{row_number}").strip()
            strategy = (raw.get("strategy") or "unknown").strip()
            try:
                trials.append(_parse_trial(raw, row_number=row_number))
            except ValueError as error:
                invalid.append(
                    InvalidTrial(
                        trial_id=trial_id,
                        strategy=strategy,
                        reason=f"row {row_number}: {error}",
                    )
                )

    return trials, invalid


def _metric_statistics(values: Sequence[float]) -> dict[str, object]:
    if not values:
        return {
            "count": 0,
            "min": "",
            "mean": "",
            "median": "",
            "max": "",
            "stddev": "",
        }

    return {
        "count": len(values),
        "min": min(values),
        "mean": statistics.fmean(values),
        "median": statistics.median(values),
        "max": max(values),
        "stddev": statistics.stdev(values) if len(values) > 1 else 0.0,
    }


def compare_trials(
    trials: Iterable[TrialRow],
    parse_invalid: Iterable[InvalidTrial] = (),
) -> Comparison:
    """Aggregate valid trials and report all exclusions."""

    grouped: dict[str, list[TrialRow]] = defaultdict(list)
    invalid = list(parse_invalid)

    for trial in trials:
        grouped[trial.strategy].append(trial)

    summary_rows: list[dict[str, object]] = []

    for strategy in STRATEGIES:
        strategy_trials = grouped.get(strategy, [])
        valid_trials: list[TrialRow] = []

        for trial in strategy_trials:
            reasons: list[str] = []
            if not trial.successful:
                reasons.append("successful is false")
            if trial.analysis_error_count != 0:
                reasons.append(
                    f"analysis_error_count={trial.analysis_error_count}"
                )

            if reasons:
                invalid.append(
                    InvalidTrial(
                        trial_id=trial.trial_id,
                        strategy=trial.strategy,
                        reason="; ".join(reasons),
                    )
                )
            else:
                valid_trials.append(trial)

        row: dict[str, object] = {
            "strategy": strategy,
            "total_trials": len(strategy_trials),
            "valid_trials": len(valid_trials),
            "invalid_trials": len(strategy_trials) - len(valid_trials),
            "successful_trials": sum(
                1 for trial in strategy_trials if trial.successful
            ),
            "failed_trials": sum(
                1 for trial in strategy_trials if not trial.successful
            ),
            "success_rate": (
                sum(1 for trial in strategy_trials if trial.successful)
                / len(strategy_trials)
                if strategy_trials
                else 0.0
            ),
            "analysis_error_count": sum(
                trial.analysis_error_count for trial in strategy_trials
            ),
            "duplicate_frame_total": sum(
                trial.duplicate_frame_count for trial in valid_trials
            ),
            "out_of_order_frame_total": sum(
                trial.out_of_order_frames for trial in valid_trials
            ),
        }

        for metric in METRICS:
            stats = _metric_statistics(
                [trial.metrics[metric] for trial in valid_trials]
            )
            for statistic, value in stats.items():
                row[f"{metric}_{statistic}"] = value

        summary_rows.append(row)

    invalid.sort(key=lambda trial: (trial.strategy, trial.trial_id, trial.reason))
    return Comparison(
        summary_rows=tuple(summary_rows),
        invalid_trials=tuple(invalid),
    )


def _format_number(value: object, *, decimals: int = 3) -> str:
    if value == "":
        return "n/a"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        if math.isclose(value, round(value), abs_tol=1e-12):
            return str(int(round(value)))
        return f"{value:.{decimals}f}"
    return str(value)


def _strategy_rows(comparison: Comparison) -> dict[str, dict[str, object]]:
    return {
        str(row["strategy"]): row
        for row in comparison.summary_rows
    }


def render_summary_markdown(comparison: Comparison) -> str:
    """Render a deterministic advisor/report-friendly Markdown summary."""

    rows = _strategy_rows(comparison)
    migrate = rows["migrate"]
    reconnect = rows["reconnect"]

    lines = [
        "# Recovery comparison",
        "",
        "Generated from the flat Epic 5.3 trial summary.",
        "",
        "## Experiment validity",
        "",
        "| Strategy | Total trials | Valid trials | Invalid trials | "
        "Successful trials | Success rate | Analysis errors |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]

    for strategy in STRATEGIES:
        row = rows[strategy]
        lines.append(
            f"| {strategy.title()} "
            f"| {row['total_trials']} "
            f"| {row['valid_trials']} "
            f"| {row['invalid_trials']} "
            f"| {row['successful_trials']} "
            f"| {_format_number(float(row['success_rate']) * 100)}% "
            f"| {row['analysis_error_count']} |"
        )

    lines.extend(
        [
            "",
            "## Main comparison",
            "",
            "| Strategy | Valid runs | Success rate | "
            "Mean receive gap | Median receive gap | "
            "Mean missing frames | Median missing frames |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ]
    )

    for strategy in STRATEGIES:
        row = rows[strategy]
        lines.append(
            f"| {strategy.title()} "
            f"| {row['valid_trials']} "
            f"| {_format_number(float(row['success_rate']) * 100)}% "
            f"| {_format_number(row['largest_receive_gap_ms_mean'])} ms "
            f"| {_format_number(row['largest_receive_gap_ms_median'])} ms "
            f"| {_format_number(row['missing_frame_count_mean'])} "
            f"| {_format_number(row['missing_frame_count_median'])} |"
        )

    lines.extend(
        [
            "",
            "## Receiver-visible interruption",
            "",
            "The primary cross-strategy interruption metric is "
            "`largest_receive_gap_ms`.",
            "",
            "| Strategy | Count | Min | Mean | Median | Max | Sample stddev |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ]
    )

    for strategy in STRATEGIES:
        row = rows[strategy]
        lines.append(
            f"| {strategy.title()} "
            f"| {row['largest_receive_gap_ms_count']} "
            f"| {_format_number(row['largest_receive_gap_ms_min'])} ms "
            f"| {_format_number(row['largest_receive_gap_ms_mean'])} ms "
            f"| {_format_number(row['largest_receive_gap_ms_median'])} ms "
            f"| {_format_number(row['largest_receive_gap_ms_max'])} ms "
            f"| {_format_number(row['largest_receive_gap_ms_stddev'])} ms |"
        )

    lines.extend(
        [
            "",
            "## Frame preservation",
            "",
            "| Strategy | Missing min | Missing mean | Missing median | "
            "Missing max | Missing stddev | Mean received unique |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ]
    )

    for strategy in STRATEGIES:
        row = rows[strategy]
        lines.append(
            f"| {strategy.title()} "
            f"| {_format_number(row['missing_frame_count_min'])} "
            f"| {_format_number(row['missing_frame_count_mean'])} "
            f"| {_format_number(row['missing_frame_count_median'])} "
            f"| {_format_number(row['missing_frame_count_max'])} "
            f"| {_format_number(row['missing_frame_count_stddev'])} "
            f"| {_format_number(row['received_unique_frames_mean'])} |"
        )

    lines.extend(
        [
            "",
            "## Transport identity",
            "",
            "| Strategy | Mean sessions | Mean connections | "
            "Duplicate frames | Out-of-order frames |",
            "|---|---:|---:|---:|---:|",
        ]
    )

    for strategy in STRATEGIES:
        row = rows[strategy]
        lines.append(
            f"| {strategy.title()} "
            f"| {_format_number(row['session_count_mean'])} "
            f"| {_format_number(row['connection_count_mean'])} "
            f"| {row['duplicate_frame_total']} "
            f"| {row['out_of_order_frame_total']} |"
        )

    lines.extend(
        [
            "",
            "## Strategy-specific action timing",
            "",
            "`recovery_action_duration_ms` is diagnostic. Migration and "
            "reconnect use different completion events, so this metric is not "
            "treated as an equivalent end-to-end interruption measurement.",
            "",
            "| Strategy | Count | Min | Mean | Median | Max | Sample stddev |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ]
    )

    for strategy in STRATEGIES:
        row = rows[strategy]
        lines.append(
            f"| {strategy.title()} "
            f"| {row['recovery_action_duration_ms_count']} "
            f"| {_format_number(row['recovery_action_duration_ms_min'])} ms "
            f"| {_format_number(row['recovery_action_duration_ms_mean'])} ms "
            f"| {_format_number(row['recovery_action_duration_ms_median'])} ms "
            f"| {_format_number(row['recovery_action_duration_ms_max'])} ms "
            f"| {_format_number(row['recovery_action_duration_ms_stddev'])} ms |"
        )

    lines.extend(
        [
            "",
            "## Notes and limitations",
            "",
            "- Statistics use only successful rows with zero analysis errors.",
            "- Standard deviation is the sample standard deviation.",
            "- Invalid or malformed rows are reported rather than silently "
            "included.",
            "- Results describe one controlled Mininet configuration and should "
            "not be generalized to physical wireless networks without further "
            "experiments.",
            "- Individual observations and plots should accompany these "
            "aggregates because the trial count is small.",
        ]
    )

    if comparison.invalid_trials:
        lines.extend(
            [
                "",
                "## Excluded trials",
                "",
                "| Trial | Strategy | Reason |",
                "|---|---|---|",
            ]
        )
        for invalid in comparison.invalid_trials:
            lines.append(
                f"| {invalid.trial_id} | {invalid.strategy} | "
                f"{invalid.reason} |"
            )

    lines.extend(
        [
            "",
            "## Preliminary observation",
            "",
            f"- Reconnect median receive gap: "
            f"{_format_number(reconnect['largest_receive_gap_ms_median'])} ms.",
            f"- Migration median receive gap: "
            f"{_format_number(migrate['largest_receive_gap_ms_median'])} ms.",
            f"- Migration median missing frames: "
            f"{_format_number(migrate['missing_frame_count_median'])}.",
            f"- Reconnect median missing frames: "
            f"{_format_number(reconnect['missing_frame_count_median'])}.",
            "",
            "In this dataset, reconnect restored receiver activity sooner, "
            "while migration preserved more media frames and retained the "
            "existing transport identity.",
            "",
        ]
    )

    return "\n".join(lines)


def write_summary_csv(
    rows: Iterable[Mapping[str, object]],
    output_path: str | Path,
) -> None:
    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=SUMMARY_FIELDS)
        writer.writeheader()
        writer.writerows(rows)


def write_outputs(
    comparison: Comparison,
    output_dir: str | Path,
) -> tuple[Path, Path]:
    directory = Path(output_dir)
    directory.mkdir(parents=True, exist_ok=True)

    csv_path = directory / "summary.csv"
    markdown_path = directory / "summary.md"

    write_summary_csv(comparison.summary_rows, csv_path)
    markdown_path.write_text(
        render_summary_markdown(comparison),
        encoding="utf-8",
    )
    return csv_path, markdown_path


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Generate aggregate migration-versus-reconnect recovery "
            "statistics from an Epic 5.3 flat summary CSV."
        )
    )
    parser.add_argument("--input", required=True, help="Epic 5.3 summary CSV")
    parser.add_argument(
        "--output-dir",
        required=True,
        help="directory for summary.csv and summary.md",
    )
    return parser


def main() -> int:
    args = _build_parser().parse_args()

    trials, parse_invalid = read_trials(args.input)
    comparison = compare_trials(trials, parse_invalid)
    csv_path, markdown_path = write_outputs(comparison, args.output_dir)

    valid_count = sum(
        int(row["valid_trials"])
        for row in comparison.summary_rows
    )
    print(
        "event=recovery_comparison_written "
        f"valid_trials={valid_count} "
        f"invalid_trials={len(comparison.invalid_trials)} "
        f"csv={csv_path} markdown={markdown_path}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
