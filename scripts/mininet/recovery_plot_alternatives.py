#!/usr/bin/env python3
"""Generate alternative recovery-comparison plots.

Outputs:
- receive-gap-jittered.png
- missing-frames-frequency.png
- receive-gap-histogram.png
"""

from __future__ import annotations

import argparse
import csv
import math
from collections import Counter, defaultdict
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
}


@dataclass(frozen=True)
class Trial:
    trial_id: str
    strategy: str
    receive_gap_ms: float
    missing_frames: int


@dataclass(frozen=True)
class InvalidTrial:
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


def _parse_nonnegative_int(value: str, *, field: str) -> int:
    try:
        parsed = int(value)
    except ValueError as error:
        raise ValueError(f"{field} must be an integer") from error
    if parsed < 0:
        raise ValueError(f"{field} must be non-negative")
    return parsed


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


def read_trials(
    input_path: str | Path,
) -> tuple[list[Trial], list[InvalidTrial]]:
    """Read successful rows with zero analysis errors."""

    valid: list[Trial] = []
    invalid: list[InvalidTrial] = []

    with Path(input_path).open(encoding="utf-8", newline="") as source:
        reader = csv.DictReader(source)

        missing = sorted(REQUIRED_COLUMNS - set(reader.fieldnames or ()))
        if missing:
            raise ValueError(
                "input CSV is missing required columns: " + ", ".join(missing)
            )

        for row_number, row in enumerate(reader, start=2):
            trial_id = (row.get("trial_id") or f"row-{row_number}").strip()
            strategy = (row.get("strategy") or "unknown").strip()

            try:
                if strategy not in STRATEGIES:
                    raise ValueError(
                        f"strategy must be one of: {', '.join(STRATEGIES)}"
                    )

                successful = _parse_bool(
                    row.get("successful", ""),
                    field="successful",
                )
                analysis_error_count = _parse_nonnegative_int(
                    row.get("analysis_error_count", ""),
                    field="analysis_error_count",
                )

                if not successful:
                    raise ValueError("successful is false")
                if analysis_error_count != 0:
                    raise ValueError(
                        f"analysis_error_count={analysis_error_count}"
                    )

                valid.append(
                    Trial(
                        trial_id=trial_id,
                        strategy=strategy,
                        receive_gap_ms=_parse_nonnegative_float(
                            row.get("largest_receive_gap_ms", ""),
                            field="largest_receive_gap_ms",
                        ),
                        missing_frames=_parse_nonnegative_int(
                            row.get("missing_frame_count", ""),
                            field="missing_frame_count",
                        ),
                    )
                )

            except ValueError as error:
                invalid.append(
                    InvalidTrial(
                        trial_id=trial_id,
                        strategy=strategy,
                        reason=f"row {row_number}: {error}",
                    )
                )

    return valid, invalid


def _values(
    trials: Iterable[Trial],
    strategy: str,
    attribute: str,
) -> list[float]:
    return [
        float(getattr(trial, attribute))
        for trial in trials
        if trial.strategy == strategy
    ]


def _require_both_strategies(trials: Sequence[Trial]) -> None:
    missing = [
        strategy
        for strategy in STRATEGIES
        if not any(trial.strategy == strategy for trial in trials)
    ]
    if missing:
        raise ValueError(
            "cannot plot without valid trials for: " + ", ".join(missing)
        )


def _median(values: Sequence[float]) -> float:
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return ordered[middle]
    return (ordered[middle - 1] + ordered[middle]) / 2


def _deterministic_jitter(values: Sequence[float]) -> list[float]:
    """Spread repeated values horizontally without randomness."""

    grouped: dict[float, list[int]] = defaultdict(list)
    for index, value in enumerate(values):
        grouped[value].append(index)

    offsets = [0.0] * len(values)

    for indices in grouped.values():
        count = len(indices)
        if count == 1:
            spread = [0.0]
        else:
            step = 0.035
            centre = (count - 1) / 2
            spread = [(position - centre) * step for position in range(count)]

        for index, offset in zip(indices, spread):
            offsets[index] = offset

    return offsets


def plot_receive_gap_jittered(
    trials: Sequence[Trial],
    output_path: str | Path,
) -> Path:
    """Plot all receive-gap observations with deterministic horizontal jitter."""

    _require_both_strategies(trials)

    positions = {"migrate": 1.0, "reconnect": 2.0}

    figure, axis = plt.subplots(figsize=(7.2, 4.8))

    for strategy in STRATEGIES:
        values = _values(trials, strategy, "receive_gap_ms")
        offsets = _deterministic_jitter(values)
        x_values = [positions[strategy] + offset for offset in offsets]

        axis.scatter(x_values, values, label="Individual trials")
        axis.scatter(
            [positions[strategy]],
            [sum(values) / len(values)],
            marker="D",
            s=70,
            label="Mean",
        )
        axis.scatter(
            [positions[strategy]],
            [_median(values)],
            marker="_",
            s=280,
            label="Median",
        )

    handles, labels = axis.get_legend_handles_labels()
    unique: dict[str, object] = {}
    for handle, label in zip(handles, labels):
        unique.setdefault(label, handle)

    axis.set_xticks([1, 2], ["Migrate", "Reconnect"])
    axis.set_ylabel("Largest receiver-observed gap (ms)")
    axis.set_title("Receiver-visible interruption by recovery strategy")
    axis.grid(axis="y")
    axis.legend(unique.values(), unique.keys())
    figure.tight_layout()

    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(path, dpi=180)
    plt.close(figure)
    return path


def plot_missing_frame_frequency(
    trials: Sequence[Trial],
    output_path: str | Path,
) -> Path:
    """Plot exact missing-frame counts as discrete trial frequencies."""

    _require_both_strategies(trials)

    observed_values = sorted({trial.missing_frames for trial in trials})
    width = 0.34

    figure, axis = plt.subplots(figsize=(7.2, 4.8))

    for index, strategy in enumerate(STRATEGIES):
        counts = Counter(
            trial.missing_frames
            for trial in trials
            if trial.strategy == strategy
        )
        x_values = [
            value + (index - 0.5) * width
            for value in observed_values
        ]
        heights = [counts.get(value, 0) for value in observed_values]

        axis.bar(
            x_values,
            heights,
            width=width,
            label=strategy.title(),
        )

    axis.set_xticks(observed_values)
    axis.set_xlabel("Missing frames")
    axis.set_ylabel("Number of trials")
    axis.set_title("Frequency of global missing-frame counts")
    axis.set_ylim(bottom=0)
    axis.grid(axis="y")
    axis.legend()
    figure.tight_layout()

    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(path, dpi=180)
    plt.close(figure)
    return path


def plot_receive_gap_histogram(
    trials: Sequence[Trial],
    output_path: str | Path,
    *,
    bin_width_ms: int = 50,
) -> Path:
    """Plot both receive-gap distributions using identical fixed-width bins."""

    _require_both_strategies(trials)

    if bin_width_ms <= 0:
        raise ValueError("bin_width_ms must be positive")

    all_values = [trial.receive_gap_ms for trial in trials]
    start = math.floor(min(all_values) / bin_width_ms) * bin_width_ms
    end = math.ceil(max(all_values) / bin_width_ms) * bin_width_ms
    bins = list(
        range(
            int(start),
            int(end + bin_width_ms) + 1,
            bin_width_ms,
        )
    )

    figure, axis = plt.subplots(figsize=(7.2, 4.8))

    for strategy in STRATEGIES:
        values = _values(trials, strategy, "receive_gap_ms")
        axis.hist(
            values,
            bins=bins,
            alpha=0.55,
            label=strategy.title(),
        )

    axis.set_xlabel("Largest receiver-observed gap (ms)")
    axis.set_ylabel("Number of trials")
    axis.set_title("Distribution of receiver-visible interruption")
    axis.grid(axis="y")
    axis.legend()
    figure.tight_layout()

    path = Path(output_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(path, dpi=180)
    plt.close(figure)
    return path


def write_readme(
    output_dir: str | Path,
    *,
    valid_count: int,
    invalid_trials: Sequence[InvalidTrial],
    bin_width_ms: int,
) -> Path:
    path = Path(output_dir) / "README.md"

    lines = [
        "# Recovery plot alternatives",
        "",
        f"Valid plotted trials: {valid_count}.",
        f"Excluded trials: {len(invalid_trials)}.",
        "",
        "- `receive-gap-jittered.png`: recommended primary interruption plot;",
        "- `missing-frames-frequency.png`: recommended frame-loss plot;",
        f"- `receive-gap-histogram.png`: supporting histogram using "
        f"{bin_width_ms} ms bins.",
        "",
        "The jitter is deterministic and is used only to reveal overlapping "
        "observations. It does not modify the measured y-values.",
        "",
        "The missing-frame plot uses exact discrete counts because missing "
        "frames are integer-valued.",
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


def generate_all(
    input_path: str | Path,
    output_dir: str | Path,
    *,
    bin_width_ms: int = 50,
) -> tuple[Path, Path, Path, Path]:
    trials, invalid = read_trials(input_path)

    directory = Path(output_dir)
    directory.mkdir(parents=True, exist_ok=True)

    jittered = plot_receive_gap_jittered(
        trials,
        directory / "receive-gap-jittered.png",
    )
    frequency = plot_missing_frame_frequency(
        trials,
        directory / "missing-frames-frequency.png",
    )
    histogram = plot_receive_gap_histogram(
        trials,
        directory / "receive-gap-histogram.png",
        bin_width_ms=bin_width_ms,
    )
    readme = write_readme(
        directory,
        valid_count=len(trials),
        invalid_trials=invalid,
        bin_width_ms=bin_width_ms,
    )

    return jittered, frequency, histogram, readme


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Generate jittered, discrete-frequency, and histogram recovery "
            "comparison plots."
        )
    )
    parser.add_argument(
        "--input",
        required=True,
        help="Epic 5.3 flat summary CSV",
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        help="directory for generated plots",
    )
    parser.add_argument(
        "--histogram-bin-width-ms",
        type=int,
        default=50,
        help="shared receive-gap histogram bin width in milliseconds",
    )
    return parser


def main() -> int:
    args = _build_parser().parse_args()

    paths = generate_all(
        args.input,
        args.output_dir,
        bin_width_ms=args.histogram_bin_width_ms,
    )

    print(
        "event=recovery_plot_alternatives_written "
        f"outputs={','.join(str(path) for path in paths)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
