import csv
import tempfile
import unittest
from pathlib import Path

from scripts.mininet.recovery_compare import (
    METRICS,
    compare_trials,
    read_trials,
    render_summary_markdown,
    write_outputs,
)


FIELDS = [
    "trial_id",
    "strategy",
    "successful",
    "analysis_error_count",
    "duplicate_frame_count",
    "out_of_order_frames",
    *METRICS,
]


def row(
    trial_id: str,
    strategy: str,
    *,
    successful: str = "True",
    analysis_errors: str = "0",
    receive_gap: str = "900",
    missing: str = "2",
    received: str = "58",
    frame_gap: str = "2",
    action: str = "150",
    sessions: str = "1",
    connections: str = "1",
) -> dict[str, str]:
    return {
        "trial_id": trial_id,
        "strategy": strategy,
        "successful": successful,
        "analysis_error_count": analysis_errors,
        "duplicate_frame_count": "0",
        "out_of_order_frames": "0",
        "largest_receive_gap_ms": receive_gap,
        "missing_frame_count": missing,
        "received_unique_frames": received,
        "largest_frame_id_gap": frame_gap,
        "recovery_action_duration_ms": action,
        "session_count": sessions,
        "connection_count": connections,
    }


def write_input(path: Path, rows: list[dict[str, str]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=FIELDS)
        writer.writeheader()
        writer.writerows(rows)


class RecoveryCompareTest(unittest.TestCase):
    def test_calculates_sample_statistics_by_strategy(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "runs.csv"
            write_input(
                source,
                [
                    row("migrate-001", "migrate", receive_gap="900"),
                    row("migrate-002", "migrate", receive_gap="1100"),
                    row(
                        "reconnect-001",
                        "reconnect",
                        receive_gap="700",
                        missing="7",
                        received="53",
                        frame_gap="7",
                        action="1",
                        sessions="2",
                        connections="2",
                    ),
                    row(
                        "reconnect-002",
                        "reconnect",
                        receive_gap="900",
                        missing="9",
                        received="51",
                        frame_gap="9",
                        action="3",
                        sessions="2",
                        connections="2",
                    ),
                ],
            )
            trials, invalid = read_trials(source)

        comparison = compare_trials(trials, invalid)
        by_strategy = {
            result["strategy"]: result
            for result in comparison.summary_rows
        }

        migrate = by_strategy["migrate"]
        reconnect = by_strategy["reconnect"]

        self.assertEqual(migrate["valid_trials"], 2)
        self.assertEqual(migrate["largest_receive_gap_ms_mean"], 1000.0)
        self.assertEqual(migrate["largest_receive_gap_ms_median"], 1000.0)
        self.assertAlmostEqual(
            migrate["largest_receive_gap_ms_stddev"],
            141.4213562373095,
        )

        self.assertEqual(reconnect["missing_frame_count_mean"], 8.0)
        self.assertEqual(reconnect["session_count_mean"], 2.0)
        self.assertEqual(reconnect["connection_count_mean"], 2.0)

    def test_excludes_unsuccessful_and_analysis_error_rows_explicitly(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "runs.csv"
            write_input(
                source,
                [
                    row("migrate-good", "migrate"),
                    row(
                        "migrate-failed",
                        "migrate",
                        successful="False",
                    ),
                    row(
                        "reconnect-errors",
                        "reconnect",
                        analysis_errors="2",
                        sessions="2",
                        connections="2",
                    ),
                ],
            )
            trials, invalid = read_trials(source)

        comparison = compare_trials(trials, invalid)
        by_strategy = {
            result["strategy"]: result
            for result in comparison.summary_rows
        }

        self.assertEqual(by_strategy["migrate"]["valid_trials"], 1)
        self.assertEqual(by_strategy["migrate"]["invalid_trials"], 1)
        self.assertEqual(by_strategy["reconnect"]["valid_trials"], 0)
        self.assertEqual(len(comparison.invalid_trials), 2)
        self.assertIn(
            "successful is false",
            comparison.invalid_trials[0].reason,
        )
        self.assertIn(
            "analysis_error_count=2",
            comparison.invalid_trials[1].reason,
        )

    def test_reports_malformed_rows_instead_of_silently_dropping_them(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "runs.csv"
            write_input(
                source,
                [
                    row("migrate-good", "migrate"),
                    row(
                        "reconnect-bad",
                        "reconnect",
                        receive_gap="not-a-number",
                        sessions="2",
                        connections="2",
                    ),
                ],
            )
            trials, invalid = read_trials(source)

        self.assertEqual(len(trials), 1)
        self.assertEqual(len(invalid), 1)
        self.assertEqual(invalid[0].trial_id, "reconnect-bad")
        self.assertIn(
            "largest_receive_gap_ms must be numeric",
            invalid[0].reason,
        )

    def test_rejects_missing_required_columns(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "runs.csv"
            source.write_text(
                "trial_id,strategy\nmigrate-001,migrate\n",
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                ValueError,
                "missing required columns",
            ):
                read_trials(source)

    def test_writes_deterministic_csv_and_markdown(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "runs.csv"
            output = root / "analysis"
            write_input(
                source,
                [
                    row("migrate-001", "migrate"),
                    row(
                        "reconnect-001",
                        "reconnect",
                        receive_gap="800",
                        missing="7",
                        received="53",
                        frame_gap="7",
                        action="1",
                        sessions="2",
                        connections="2",
                    ),
                ],
            )
            trials, invalid = read_trials(source)
            comparison = compare_trials(trials, invalid)

            csv_path, markdown_path = write_outputs(comparison, output)
            first_csv = csv_path.read_text(encoding="utf-8")
            first_markdown = markdown_path.read_text(encoding="utf-8")

            write_outputs(comparison, output)

            self.assertEqual(
                csv_path.read_text(encoding="utf-8"),
                first_csv,
            )
            self.assertEqual(
                markdown_path.read_text(encoding="utf-8"),
                first_markdown,
            )
            self.assertIn(
                "primary cross-strategy interruption metric is `largest_receive_gap_ms`",
                first_markdown,
            )
            self.assertIn(
                "not treated as an equivalent end-to-end",
                first_markdown,
            )

    def test_markdown_contains_invalid_trial_table_when_needed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "runs.csv"
            write_input(
                source,
                [
                    row("migrate-001", "migrate"),
                    row(
                        "reconnect-failed",
                        "reconnect",
                        successful="False",
                        sessions="2",
                        connections="2",
                    ),
                ],
            )
            trials, invalid = read_trials(source)

        markdown = render_summary_markdown(
            compare_trials(trials, invalid)
        )

        self.assertIn("## Excluded trials", markdown)
        self.assertIn("reconnect-failed", markdown)
        self.assertIn("successful is false", markdown)


if __name__ == "__main__":
    unittest.main()
