import csv
import tempfile
import unittest
from pathlib import Path

from scripts.mininet.recovery_plots import (
    generate_plots,
    read_plot_trials,
    write_plot_readme,
)


FIELDS = [
    "trial_id",
    "strategy",
    "successful",
    "analysis_error_count",
    "largest_receive_gap_ms",
    "missing_frame_count",
    "recovery_action_duration_ms",
]


def write_rows(path: Path, rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)


def row(
    trial_id: str,
    strategy: str,
    *,
    successful: str = "True",
    errors: str = "0",
    gap: str = "900",
    missing: str = "2",
    action: str = "150",
) -> dict[str, str]:
    return {
        "trial_id": trial_id,
        "strategy": strategy,
        "successful": successful,
        "analysis_error_count": errors,
        "largest_receive_gap_ms": gap,
        "missing_frame_count": missing,
        "recovery_action_duration_ms": action,
    }


class RecoveryPlotsTest(unittest.TestCase):
    def test_reads_valid_trials_and_reports_exclusions(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "summary.csv"
            write_rows(
                source,
                [
                    row("migrate-001", "migrate"),
                    row(
                        "reconnect-001",
                        "reconnect",
                        gap="800",
                        missing="7",
                        action="1",
                    ),
                    row(
                        "reconnect-failed",
                        "reconnect",
                        successful="False",
                    ),
                ],
            )
            trials, invalid = read_plot_trials(source)

        self.assertEqual(len(trials), 2)
        self.assertEqual(len(invalid), 1)
        self.assertEqual(invalid[0].trial_id, "reconnect-failed")
        self.assertIn("successful is false", invalid[0].reason)

    def test_rejects_missing_columns(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "summary.csv"
            source.write_text(
                "trial_id,strategy\nmigrate-001,migrate\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                ValueError,
                "missing required columns",
            ):
                read_plot_trials(source)

    def test_generates_three_nonempty_png_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "summary.csv"
            output = root / "plots"
            write_rows(
                source,
                [
                    row("migrate-001", "migrate", gap="900"),
                    row("migrate-002", "migrate", gap="1000"),
                    row(
                        "reconnect-001",
                        "reconnect",
                        gap="800",
                        missing="7",
                        action="1",
                    ),
                    row(
                        "reconnect-002",
                        "reconnect",
                        gap="850",
                        missing="8",
                        action="2",
                    ),
                ],
            )
            trials, invalid = read_plot_trials(source)
            paths = generate_plots(trials, output)
            readme = write_plot_readme(
                output,
                valid_trials=len(trials),
                invalid_trials=invalid,
            )

            self.assertEqual(len(paths), 3)
            for path in paths:
                self.assertTrue(path.exists())
                self.assertGreater(path.stat().st_size, 1000)
            self.assertIn(
                "primary cross-strategy interruption metric",
                readme.read_text(encoding="utf-8"),
            )

    def test_requires_both_strategies(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "summary.csv"
            write_rows(
                source,
                [
                    row("migrate-001", "migrate"),
                    row("migrate-002", "migrate"),
                ],
            )
            trials, _ = read_plot_trials(source)

            with self.assertRaisesRegex(
                ValueError,
                "without valid trials for: reconnect",
            ):
                generate_plots(trials, root / "plots")


if __name__ == "__main__":
    unittest.main()
