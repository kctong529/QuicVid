import unittest

from scripts.mininet.recovery_analysis import parse_text
from scripts.mininet.recovery_timing import extract_recovery_timing


class RecoveryTimingTest(unittest.TestCase):
    def test_extracts_migration_action_timing_from_seconds(self) -> None:
        client = parse_text(
            """
event=automatic_rebind_started elapsed_seconds=2.502
event=migration_confirmed elapsed_seconds=2.731
"""
        )

        result = extract_recovery_timing(client, "migrate")

        self.assertEqual(result.start_event, "automatic_rebind_started")
        self.assertEqual(result.completion_event, "migration_confirmed")
        self.assertAlmostEqual(result.recovery_action_started_ms, 2502.0)
        self.assertAlmostEqual(result.recovery_action_completed_ms, 2731.0)
        self.assertAlmostEqual(result.recovery_action_duration_ms, 229.0)
        self.assertEqual(result.analysis_errors, [])

    def test_extracts_reconnect_action_timing_from_milliseconds(self) -> None:
        client = parse_text(
            """
event=reconnect_started elapsed_ms=2510
event=reconnect_completed elapsed_ms=2744
"""
        )

        result = extract_recovery_timing(client, "reconnect")

        self.assertEqual(result.recovery_action_started_ms, 2510.0)
        self.assertEqual(result.recovery_action_completed_ms, 2744.0)
        self.assertEqual(result.recovery_action_duration_ms, 234.0)
        self.assertEqual(result.analysis_errors, [])

    def test_accepts_timestamp_ms(self) -> None:
        client = parse_text(
            """
event=reconnect_started timestamp_ms=10000
event=reconnect_completed timestamp_ms=10125
"""
        )

        result = extract_recovery_timing(client, "reconnect")

        self.assertEqual(result.recovery_action_duration_ms, 125.0)

    def test_reports_missing_start(self) -> None:
        client = parse_text(
            "event=migration_confirmed elapsed_seconds=2.7\n"
        )

        result = extract_recovery_timing(client, "migrate")

        self.assertIsNone(result.recovery_action_duration_ms)
        self.assertEqual(
            result.analysis_errors,
            ["missing automatic_rebind_started"],
        )

    def test_reports_missing_completion(self) -> None:
        client = parse_text(
            "event=reconnect_started elapsed_seconds=2.5\n"
        )

        result = extract_recovery_timing(client, "reconnect")

        self.assertIsNone(result.recovery_action_duration_ms)
        self.assertEqual(
            result.analysis_errors,
            ["missing reconnect_completed"],
        )

    def test_reports_missing_timestamp(self) -> None:
        client = parse_text(
            """
event=reconnect_started media_run=R1
event=reconnect_completed media_run=R1
"""
        )

        result = extract_recovery_timing(client, "reconnect")

        self.assertIsNone(result.recovery_action_duration_ms)
        self.assertEqual(
            result.analysis_errors,
            [
                "reconnect_started at line 2 has no supported timestamp",
                "reconnect_completed at line 3 has no supported timestamp",
            ],
        )

    def test_rejects_negative_duration(self) -> None:
        client = parse_text(
            """
event=automatic_rebind_started elapsed_ms=3000
event=migration_confirmed elapsed_ms=2900
"""
        )

        result = extract_recovery_timing(client, "migrate")

        self.assertIsNone(result.recovery_action_duration_ms)
        self.assertIn("occurs 100.000 ms before", result.analysis_errors[0])

    def test_reports_duplicate_recovery_events(self) -> None:
        client = parse_text(
            """
event=reconnect_started elapsed_ms=2000
event=reconnect_started elapsed_ms=2010
event=reconnect_completed elapsed_ms=2200
event=reconnect_completed elapsed_ms=2210
"""
        )

        result = extract_recovery_timing(client, "reconnect")

        self.assertEqual(result.recovery_action_duration_ms, 200.0)
        self.assertIn(
            "expected one reconnect_started, found 2",
            result.analysis_errors,
        )
        self.assertIn(
            "expected one reconnect_completed, found 2",
            result.analysis_errors,
        )

    def test_rejects_unsupported_strategy(self) -> None:
        result = extract_recovery_timing(parse_text(""), "timeout")

        self.assertEqual(
            result.analysis_errors,
            ["unsupported recovery strategy: timeout"],
        )

    def test_field_names_describe_strategy_specific_action(self) -> None:
        result = extract_recovery_timing(
            parse_text(
                """
event=reconnect_started elapsed_ms=100
event=reconnect_completed elapsed_ms=103
"""
            ),
            "reconnect",
        )

        self.assertTrue(hasattr(result, "recovery_action_started_ms"))
        self.assertTrue(hasattr(result, "recovery_action_completed_ms"))
        self.assertTrue(hasattr(result, "recovery_action_duration_ms"))
        self.assertFalse(hasattr(result, "recovery_duration_ms"))


if __name__ == "__main__":
    unittest.main()
