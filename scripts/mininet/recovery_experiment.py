#!/usr/bin/env python3
"""Run repeated migration and reconnect recovery experiments."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

from scripts.mininet.recovery_analysis import parse_file
from scripts.mininet.recovery_result import (
    build_recovery_run_result,
    write_recovery_run_result,
)
from scripts.mininet.recovery_summary import (
    flatten_recovery_result,
    write_summary_csv,
)


STRATEGIES = ("migrate", "reconnect")


@dataclass(frozen=True)
class Trial:
    strategy: str
    index: int
    directory: Path

    @property
    def trial_id(self) -> str:
        return self.directory.name


@dataclass
class TrialOutcome:
    trial: Trial
    command: list[str]
    returncode: int
    result_written: bool
    successful: bool
    error: str | None = None


def build_trials(
    output_root: str | Path,
    repetitions: int,
    strategies: Iterable[str] = STRATEGIES,
) -> list[Trial]:
    if repetitions < 1:
        raise ValueError("repetitions must be at least 1")

    root = Path(output_root)
    trials: list[Trial] = []

    for index in range(1, repetitions + 1):
        for strategy in strategies:
            if strategy not in STRATEGIES:
                raise ValueError(f"unsupported strategy: {strategy}")
            directory = root / f"{strategy}-{index:03d}"
            trials.append(
                Trial(
                    strategy=strategy,
                    index=index,
                    directory=directory,
                )
            )

    return trials


def parse_command_template(value: str) -> list[str]:
    try:
        command = json.loads(value)
    except json.JSONDecodeError as error:
        raise ValueError(
            "--scenario-command must be a JSON array of strings"
        ) from error

    if (
        not isinstance(command, list)
        or not command
        or any(not isinstance(part, str) for part in command)
    ):
        raise ValueError(
            "--scenario-command must be a non-empty JSON array of strings"
        )

    return command


def render_command(
    template: Sequence[str],
    trial: Trial,
) -> list[str]:
    values = {
        "strategy": trial.strategy,
        "trial_index": trial.index,
        "trial_id": trial.trial_id,
        "trial_dir": str(trial.directory),
    }

    try:
        return [part.format(**values) for part in template]
    except KeyError as error:
        raise ValueError(
            f"unknown scenario-command placeholder: {error.args[0]}"
        ) from error


def analyze_trial(trial: Trial) -> bool:
    client_log = trial.directory / "client.log"
    server_log = trial.directory / "server.log"
    result_path = trial.directory / "result.json"

    missing = [
        str(path)
        for path in (client_log, server_log)
        if not path.is_file()
    ]
    if missing:
        raise FileNotFoundError(
            "missing expected trial logs: " + ", ".join(missing)
        )

    result = build_recovery_run_result(
        parse_file(client_log),
        parse_file(server_log),
    )
    write_recovery_run_result(result, result_path)
    return result.successful


def run_trial(
    trial: Trial,
    command_template: Sequence[str],
    *,
    timeout_seconds: float | None,
) -> TrialOutcome:
    trial.directory.mkdir(parents=True, exist_ok=True)
    command = render_command(command_template, trial)

    command_path = trial.directory / "command.json"
    command_path.write_text(
        json.dumps(command, indent=2) + "\n",
        encoding="utf-8",
    )

    stdout_path = trial.directory / "runner.stdout.log"
    stderr_path = trial.directory / "runner.stderr.log"

    try:
        with stdout_path.open("w", encoding="utf-8") as stdout, \
             stderr_path.open("w", encoding="utf-8") as stderr:
            completed = subprocess.run(
                command,
                stdout=stdout,
                stderr=stderr,
                text=True,
                timeout=timeout_seconds,
                check=False,
            )
    except subprocess.TimeoutExpired:
        return TrialOutcome(
            trial=trial,
            command=command,
            returncode=124,
            result_written=False,
            successful=False,
            error=f"scenario timed out after {timeout_seconds} seconds",
        )
    except OSError as error:
        return TrialOutcome(
            trial=trial,
            command=command,
            returncode=127,
            result_written=False,
            successful=False,
            error=str(error),
        )

    if completed.returncode != 0:
        return TrialOutcome(
            trial=trial,
            command=command,
            returncode=completed.returncode,
            result_written=False,
            successful=False,
            error="scenario command failed",
        )

    try:
        successful = analyze_trial(trial)
    except Exception as error:
        return TrialOutcome(
            trial=trial,
            command=command,
            returncode=completed.returncode,
            result_written=False,
            successful=False,
            error=str(error),
        )

    return TrialOutcome(
        trial=trial,
        command=command,
        returncode=completed.returncode,
        result_written=True,
        successful=successful,
        error=None if successful else "recovery analysis marked run unsuccessful",
    )


def write_experiment_summary(
    outcomes: Sequence[TrialOutcome],
    output_root: str | Path,
) -> None:
    rows = []

    for outcome in outcomes:
        result_path = outcome.trial.directory / "result.json"
        if not result_path.is_file():
            continue

        result = json.loads(result_path.read_text(encoding="utf-8"))
        rows.append(
            flatten_recovery_result(
                result,
                trial_id=outcome.trial.trial_id,
            )
        )

    write_summary_csv(
        rows,
        Path(output_root) / "summary.csv",
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run repeated migration and reconnect recovery trials.",
    )
    parser.add_argument("--output-root", required=True)
    parser.add_argument("--repetitions", type=int, default=10)
    parser.add_argument(
        "--strategy",
        action="append",
        choices=STRATEGIES,
        dest="strategies",
        help="strategy to run; repeat to select both explicitly",
    )
    parser.add_argument(
        "--scenario-command",
        required=True,
        help=(
            "JSON argv array supporting {strategy}, {trial_index}, "
            "{trial_id}, and {trial_dir} placeholders"
        ),
    )
    parser.add_argument("--timeout-seconds", type=float, default=120.0)
    parser.add_argument(
        "--continue-on-failure",
        action="store_true",
        help="continue after a scenario or analysis failure",
    )
    return parser


def main() -> int:
    args = _build_parser().parse_args()

    try:
        command_template = parse_command_template(args.scenario_command)
        trials = build_trials(
            args.output_root,
            args.repetitions,
            args.strategies or STRATEGIES,
        )
    except ValueError as error:
        print(f"event=experiment_configuration_error error={json.dumps(str(error))}")
        return 2

    outcomes: list[TrialOutcome] = []

    for trial in trials:
        print(
            "event=experiment_trial_started "
            f"trial={trial.trial_id} strategy={trial.strategy}"
        )

        outcome = run_trial(
            trial,
            command_template,
            timeout_seconds=args.timeout_seconds,
        )
        outcomes.append(outcome)

        print(
            "event=experiment_trial_finished "
            f"trial={trial.trial_id} "
            f"strategy={trial.strategy} "
            f"returncode={outcome.returncode} "
            f"result_written={str(outcome.result_written).lower()} "
            f"successful={str(outcome.successful).lower()} "
            f"error={json.dumps(outcome.error)}"
        )

        if not outcome.successful and not args.continue_on_failure:
            break

    write_experiment_summary(outcomes, args.output_root)

    failed = sum(not outcome.successful for outcome in outcomes)
    print(
        "event=experiment_finished "
        f"planned={len(trials)} "
        f"completed={len(outcomes)} "
        f"failed={failed} "
        f"summary={Path(args.output_root) / 'summary.csv'}"
    )

    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
