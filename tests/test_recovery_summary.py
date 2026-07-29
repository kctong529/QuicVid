import csv
import tempfile
import unittest
from pathlib import Path

from scripts.mininet.recovery_summary import (
    CSV_FIELDS,
    flatten_recovery_result,
    write_summary_csv,
)


def sample_result(strategy: str = "migrate") -> dict:
    sessions = [
        {
            "session_id": "S1",
            "connection_id": 101,
        }
    ]
    if strategy == "reconnect":
        sessions.append(
            {
                "session_id": "S2",
                "connection_id": 202,
            }
        )

    return {
        "schema_version": 1,
        "strategy": strategy,
        "media_run_id": "R1",
        "successful": True,
        "analysis_errors": [],
        "identity": {
            "expected_frames": 60,
            "sessions": sessions,
        },
        "frames": {
            "received_unique_frames": 58,
            "missing_frame_ids": [21, 22],
            "duplicate_frames": 0,
        },
        "timing": {
            "recovery_action_duration_ms": 157.0,
        },
        "continuity": {
            "largest_frame_id_gap": 2,
            "out_of_order_frames": 0,
            "largest_receive_gap_ms": 941.5,
        },
    }


class RecoverySummaryTest(unittest.TestCase):
    def test_flattens_migration_result(self) -> None:
        row = flatten_recovery_result(
            sample_result("migrate"),
            trial_id="migration-001",
        )

        self.assertEqual(row["trial_id"], "migration-001")
        self.assertEqual(row["strategy"], "migrate")
        self.assertEqual(row["session_count"], 1)
        self.assertEqual(row["connection_count"], 1)
        self.assertEqual(row["missing_frame_count"], 2)
        self.assertEqual(row["largest_receive_gap_ms"], 941.5)
        self.assertEqual(row["recovery_action_duration_ms"], 157.0)

    def test_flattens_reconnect_result(self) -> None:
        result = sample_result("reconnect")
        result["frames"]["received_unique_frames"] = 52
        result["frames"]["missing_frame_ids"] = list(range(21, 29))
        result["continuity"]["largest_frame_id_gap"] = 8
        result["continuity"]["largest_receive_gap_ms"] = 900.9
        result["timing"]["recovery_action_duration_ms"] = 2.0

        row = flatten_recovery_result(
            result,
            trial_id="reconnect-001",
        )

        self.assertEqual(row["session_count"], 2)
        self.assertEqual(row["connection_count"], 2)
        self.assertEqual(row["missing_frame_count"], 8)
        self.assertEqual(row["largest_frame_id_gap"], 8)

    def test_counts_analysis_errors(self) -> None:
        result = sample_result()
        result["successful"] = False
        result["analysis_errors"] = ["timing: missing event", "frames: bad ID"]

        row = flatten_recovery_result(result, trial_id="broken")

        self.assertFalse(row["successful"])
        self.assertEqual(row["analysis_error_count"], 2)

    def test_deduplicates_connection_ids(self) -> None:
        result = sample_result("reconnect")
        result["identity"]["sessions"][1]["connection_id"] = 101

        row = flatten_recovery_result(result, trial_id="same-connection")

        self.assertEqual(row["session_count"], 2)
        self.assertEqual(row["connection_count"], 1)

    def test_writes_stable_csv_header_and_rows(self) -> None:
        rows = [
            flatten_recovery_result(
                sample_result("migrate"),
                trial_id="migration-001",
            ),
            flatten_recovery_result(
                sample_result("reconnect"),
                trial_id="reconnect-001",
            ),
        ]

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "summary.csv"
            write_summary_csv(rows, output)

            with output.open(encoding="utf-8", newline="") as source:
                reader = csv.DictReader(source)
                written_rows = list(reader)

        self.assertEqual(reader.fieldnames, CSV_FIELDS)
        self.assertEqual(len(written_rows), 2)
        self.assertEqual(written_rows[0]["trial_id"], "migration-001")
        self.assertEqual(written_rows[1]["trial_id"], "reconnect-001")

    def test_rejects_missing_analysis_error_array(self) -> None:
        result = sample_result()
        del result["analysis_errors"]

        with self.assertRaisesRegex(
            ValueError,
            "analysis_errors must be an array",
        ):
            flatten_recovery_result(result, trial_id="broken")


if __name__ == "__main__":
    unittest.main()
