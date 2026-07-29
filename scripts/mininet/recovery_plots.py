#!/usr/bin/env python3
"""Generate recovery comparison plots from the Epic 5.3 flat summary CSV."""

from __future__ import annotations

import argparse
import csv
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt


STRATEGIES = ("migrate", "reconnect")
REQUIRED_COLUMNS = {
    "trial_id",
    "strategy",
    "successful",
    "analysis_error_count",
    "largest_receive_gap_ms",
    "missing_frame_count",
    "recovery_action_duration_ms",
}


@dataclass(frozen=True)
class PlotTrial:
    trial_id: str
    strategy: str
    receive_gap_ms: float
    missing_frames: float
    action_duration_ms: float


@dataclass(frozen=True)
class InvalidPlotTrial:
    trial_id: str
    strategy: str
    reason: str


def _parse_bool(value: str, *, field: str) -> bool:
    normalized = value.strip().lower()
    if normalized in {"true", "1", "yes"}:
        return True
    if normalized in {"false", "0", "no"}:
        return False
    raise ValueError(f"{field} must be a boolean")


def _parse_nonnegative_float(value: str, *, field: str) -> float:
    try:
        parsed = float(value)
    except ValueError as error:
        raise ValueError(f"{field} must be numeric") from error
    if not math.isfinite(parsed):
        raise ValueError(f"{field} must be finite")
    if parsed < 0:
        raise ValueError(f"{field} must be non-negative")
    return parsed


def _parse_nonnegative_int(value: str, *, field: str) -> int:
    try:
        parsed = int(value)
    except ValueError as error:
        raise ValueError(f"{field} must be an integer") from error
    if parsed < 0:
        raise ValueError(f"{field} must be non-negative")
    return parsed


def read_plot_trials(
    input_path: str | Path,
) -> tuple[list[PlotTrial], list[InvalidPlotTrial]]:
    """Read successful, analyzable trials and report every excluded row."""

    valid: list[PlotTrial] = []
    invalid: list[InvalidPlotTrial] = []

    with Path(input_path).open(encoding="utf-8", newline="") as source:
        reader = csv.DictReader(source)
        missing = sorted(REQUIRED_COLUMNS - set(reader.fieldnames or ()))
        if missing:
            raise ValueError(
                "input CSV is missing required columns: " + ", ".join(missing)
            )

        for row_number, raw in enumerate(reader, start=2):
            trial_id = (raw.get("trial_id") or f"row-{row_number}").strip()
            strategy = (raw.get("strategy") or "unknown").strip()

            try:
                if strategy not in STRATEGIES:
                    raise ValueError(
                        f"strategy must be one of: {', '.join(STRATEGIES)}"
                    )
                successful = _parse_bool(
                    raw.get("successful", ""),
                    field="successful",
                )
                analysis_error_count = _parse_nonnegative_int(
                    raw.get("analysis_error_count", ""),
                    field="analysis_error_count",
                )
                if not successful:
                    raise ValueError("successful is false")
                if analysis_error_count != 0:
                    raise ValueError(
                        f"analysis_error_count={analysis_error_count}"
                    )

                valid.append(
                    PlotTrial(
                        trial_id=trial_id,
                        strategy=strategy,
                        receive_gap_ms=_parse_nonnegative_float(
                            raw.get("largest_receive_gap_ms", ""),
                            field="largest_receive_gap_ms",
                        ),
                        missing_frames=_parse_nonnegative_float(
                            raw.get("missing_frame_count", ""),
                            field="missing_frame_count",
                        ),
                        action_duration_ms=_parse_nonnegative_float(
                            raw.get("recovery_action_duration_ms", ""),
                            field="recovery_action_duration_ms",
                        ),
                    )
                )
            except ValueError as error:
                invalid.append(
                    InvalidPlotTrial(
                        trial_id=trial_id,
                        strategy=strategy,
                        reason=f"row {row_number}: {error}",
                    )
                )

    return valid, invalid


def _group_values(
    trials: Iterable[PlotTrial],
    attribute: str,
) -> list[list[float]]:
    grouped: list[list[float]] = []
    trials_list = list(trials)
    for strategy in STRATEGIES:
        grouped.append(
            [
                float(getattr(trial, attribute))
                for trial in trials_list
                if trial.strategy == strategy
            ]
        )
    return grouped


def _require_both_strategies(values: Sequence[Sequence[float]]) -> None:
    missing = [
        strategy
        for strategy, strategy_values in zip(STRATEGIES, values)
        if not strategy_values
    ]
    if missing:
        raise ValueError(
            "cannot plot without valid trials for: " + ", ".join(missing)
        )


def _draw_observation_plot(
    values: Sequence[Sequence[float]],
    *,
    ylabel: str,
    title: str,
    output_path: str | Path,
) -> Path:
    _require_both_strategies(values)

    figure, axis = plt.subplots(figsize=(7.2, 4.8))

    positions = range(1, len(STRATEGIES) + 1)
    for position, strategy_values in zip(positions, values):
        x_values = [position] * len(strategy_values)
        axis.scatter(x_values, strategy_values, label="Individual trials")
        mean_value = sum(strategy_values) / len(strategy_values)
        sorted_values = sorted(strategy_values)
        middle = len(sorted_values) // 2
        if len(sorted_values) % 2:
            median_value = sorted_values[middle]
        else:
            median_value = (
                sorted_values[middle - 1] + sorted_values[middle]
            ) / 2
        axis.scatter([position], [mean_value], marker="D", s=70, label="Mean")
        axis.scatter([position], [median_value], marker="_", s=280, label="Median")

    handles, labels = axis.get_legend_handles_labels()
    unique: dict[str, object] = {}
    for handle, label in zip(handles, labels):
        unique.setdefault(label, handle)

    axis.set_xticks(list(positions), [strategy.title() for strategy in STRATEGIES])
    axis.set_ylabel(ylabel)
    axis.set_title(title)
    axis.grid(axis="y")
    axis.legend(unique.values(), unique.keys())
    figure.tight_layout()

    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(path, dpi=180)
    plt.close(figure)
    return path


def generate_plots(
    trials: Sequence[PlotTrial],
    output_dir: str | Path,
) -> tuple[Path, Path, Path]:
    """Generate one distinct figure per metric."""

    directory = Path(output_dir)
    receive_gap = _draw_observation_plot(
        _group_values(trials, "receive_gap_ms"),
        ylabel="Largest receiver-observed gap (ms)",
        title="Receiver-visible interruption by recovery strategy",
        output_path=directory / "receive-gap.png",
    )
    missing_frames = _draw_observation_plot(
        _group_values(trials, "missing_frames"),
        ylabel="Missing frames",
        title="Global missing frames by recovery strategy",
        output_path=directory / "missing-frames.png",
    )
    action_duration = _draw_observation_plot(
        _group_values(trials, "action_duration_ms"),
        ylabel="Strategy-specific action duration (ms)",
        title="Recovery-action timing by strategy",
        output_path=directory / "action-duration.png",
    )
    return receive_gap, missing_frames, action_duration


def write_plot_readme(
    output_dir: str | Path,
    *,
    valid_trials: int,
    invalid_trials: Sequence[InvalidPlotTrial],
) -> Path:
    path = Path(output_dir) / "README.md"
    lines = [
        "# Recovery comparison plots",
        "",
        "Generated from `../../summary.csv`.",
        "",
        "Plots:",
        "",
        "- `receive-gap.png` — primary cross-strategy interruption metric;",
        "- `missing-frames.png` — global media-run frame loss;",
        "- `action-duration.png` — strategy-specific diagnostic timing.",
        "",
        f"Valid plotted trials: {valid_trials}.",
        f"Excluded trials: {len(invalid_trials)}.",
        "",
        "Each plot shows individual observations together with mean and median "
        "markers. With ten trials per strategy, individual observations are "
        "shown rather than only aggregate bars.",
        "",
        "`action-duration.png` must not be interpreted as an equivalent "
        "end-to-end comparison because migration and reconnect use different "
        "completion events.",
        "",
    ]

    if invalid_trials:
        lines.extend(
            [
                "## Excluded trials",
                "",
                "| Trial | Strategy | Reason |",
                "|---|---|---|",
            ]
        )
        for trial in invalid_trials:
            lines.append(
                f"| {trial.trial_id} | {trial.strategy} | {trial.reason} |"
            )
        lines.append("")

    path.write_text("\n".join(lines), encoding="utf-8")
    return path


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Generate individual-observation recovery comparison plots from "
            "the Epic 5.3 flat summary CSV."
        )
    )
    parser.add_argument("--input", required=True, help="Epic 5.3 summary CSV")
    parser.add_argument(
        "--output-dir",
        required=True,
        help="directory for generated PNG files and README.md",
    )
    return parser


def main() -> int:
    args = _build_parser().parse_args()
    trials, invalid = read_plot_trials(args.input)
    paths = generate_plots(trials, args.output_dir)
    readme = write_plot_readme(
        args.output_dir,
        valid_trials=len(trials),
        invalid_trials=invalid,
    )

    print(
        "event=recovery_plots_written "
        f"valid_trials={len(trials)} "
        f"invalid_trials={len(invalid)} "
        f"plots={','.join(str(path) for path in paths)} "
        f"readme={readme}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
