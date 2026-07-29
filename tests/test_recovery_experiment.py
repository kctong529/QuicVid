import json
import sys
import tempfile
import unittest
from pathlib import Path

from scripts.mininet.recovery_experiment import (
    Trial,
    build_trials,
    parse_command_template,
    render_command,
    run_trial,
)


class RecoveryExperimentTest(unittest.TestCase):
    def test_builds_interleaved_trials(self) -> None:
        trials = build_trials("/tmp/results", 2)

        self.assertEqual(
            [(trial.strategy, trial.index, trial.trial_id) for trial in trials],
            [
                ("migrate", 1, "migrate-001"),
                ("reconnect", 1, "reconnect-001"),
                ("migrate", 2, "migrate-002"),
                ("reconnect", 2, "reconnect-002"),
            ],
        )

    def test_rejects_zero_repetitions(self) -> None:
        with self.assertRaisesRegex(ValueError, "at least 1"):
            build_trials("/tmp/results", 0)

    def test_parses_json_command_template(self) -> None:
        command = parse_command_template(
            '["python3", "launcher.py", "--strategy", "{strategy}"]'
        )

        self.assertEqual(command[0], "python3")
        self.assertEqual(command[-1], "{strategy}")

    def test_rejects_non_array_command_template(self) -> None:
        with self.assertRaisesRegex(ValueError, "JSON array"):
            parse_command_template('"python3 launcher.py"')

    def test_renders_command_without_shell(self) -> None:
        trial = Trial(
            strategy="reconnect",
            index=3,
            directory=Path("/tmp/reconnect-003"),
        )

        rendered = render_command(
            [
                "launcher",
                "--strategy",
                "{strategy}",
                "--output",
                "{trial_dir}",
                "--trial",
                "{trial_index}",
            ],
            trial,
        )

        self.assertEqual(
            rendered,
            [
                "launcher",
                "--strategy",
                "reconnect",
                "--output",
                "/tmp/reconnect-003",
                "--trial",
                "3",
            ],
        )

    def test_runs_scenario_and_writes_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            trial = Trial(
                strategy="migrate",
                index=1,
                directory=root / "migrate-001",
            )

            script = root / "fake_scenario.py"
            script.write_text(
                """
import sys
from pathlib import Path

trial_dir = Path(sys.argv[1])
trial_dir.mkdir(parents=True, exist_ok=True)

(trial_dir / "client.log").write_text(
    "\\n".join([
        "event=recovery_strategy_config strategy=migrate",
        "event=media_run_created media_run=R1 expected_frames=2",
        "event=session_created session=S1",
        "event=connected session=S1 connection=101 local=10.0.1.2:5000",
        "event=hello_sent media_run=R1 session=S1",
        "event=automatic_rebind_started elapsed_ms=1000",
        "event=migration_confirmed elapsed_ms=1100",
        "event=media_run_completed media_run=R1 final_frame_exclusive=2 sessions=1",
    ]) + "\\n",
    encoding="utf-8",
)

(trial_dir / "server.log").write_text(
    "\\n".join([
        "event=client_hello media_run=R1 session=S1 connection=101",
        "event=jpeg_frame_validated session=S1 frame=0 received_at_ms=1000",
        "event=jpeg_frame_validated session=S1 frame=1 received_at_ms=1100",
        "event=jpeg_video_done media_run=R1 session=S1 final_frame_exclusive=2",
    ]) + "\\n",
    encoding="utf-8",
)
""",
                encoding="utf-8",
            )

            outcome = run_trial(
                trial,
                [sys.executable, str(script), "{trial_dir}"],
                timeout_seconds=5,
            )

            self.assertEqual(outcome.returncode, 0)
            self.assertTrue(outcome.result_written)
            self.assertTrue(outcome.successful)
            self.assertTrue((trial.directory / "result.json").is_file())
            self.assertTrue((trial.directory / "command.json").is_file())

    def test_reports_missing_logs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            trial = Trial(
                strategy="migrate",
                index=1,
                directory=Path(directory) / "migrate-001",
            )

            outcome = run_trial(
                trial,
                [sys.executable, "-c", "pass"],
                timeout_seconds=5,
            )

            self.assertFalse(outcome.result_written)
            self.assertFalse(outcome.successful)
            self.assertIn("missing expected trial logs", outcome.error)

    def test_reports_scenario_timeout(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            trial = Trial(
                strategy="migrate",
                index=1,
                directory=Path(directory) / "migrate-001",
            )

            outcome = run_trial(
                trial,
                [sys.executable, "-c", "import time; time.sleep(1)"],
                timeout_seconds=0.01,
            )

            self.assertEqual(outcome.returncode, 124)
            self.assertFalse(outcome.successful)
            self.assertIn("timed out", outcome.error)


if __name__ == "__main__":
    unittest.main()
