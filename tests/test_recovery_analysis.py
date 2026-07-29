from pathlib import Path
import tempfile
import unittest

from scripts.mininet import recovery_analysis


class ParseStructuredRecoveryLogsTest(unittest.TestCase):
    def test_parses_event_with_prefix_and_scalar_values(self) -> None:
        parsed = recovery_analysis.parse_text(
            "INFO client event=reconnect_completed "
            "elapsed_seconds=2.702 sessions=2 completed=true\n"
        )
        event = parsed.events[0]
        self.assertEqual(event.name, "reconnect_completed")
        self.assertEqual(event.fields["elapsed_seconds"], 2.702)
        self.assertEqual(event.fields["sessions"], 2)
        self.assertIs(event.fields["completed"], True)
        self.assertEqual(parsed.malformed, [])

    def test_preserves_identifiers_and_addresses_as_strings(self) -> None:
        parsed = recovery_analysis.parse_text(
            "event=connected "
            "session=838eb93a-0123-4567-89ab-abcdefabcdef "
            "local=10.0.2.2:51000 connection=000123\n"
        )
        event = parsed.events[0]
        self.assertEqual(
            event.fields["session"],
            "838eb93a-0123-4567-89ab-abcdefabcdef",
        )
        self.assertEqual(event.fields["local"], "10.0.2.2:51000")
        self.assertEqual(event.fields["connection"], "000123")

    def test_parses_quoted_values(self) -> None:
        parsed = recovery_analysis.parse_text(
            'event=analysis_failed reason="missing migration_confirmed"\n'
        )
        self.assertEqual(
            parsed.events[0].fields["reason"], "missing migration_confirmed"
        )

    def test_ignores_non_event_output(self) -> None:
        parsed = recovery_analysis.parse_text("building\nframe 28 sent\n")
        self.assertEqual(parsed.events, [])
        self.assertEqual(parsed.malformed, [])

    def test_reports_unterminated_quote(self) -> None:
        parsed = recovery_analysis.parse_text(
            'event=analysis_failed reason="unfinished\n'
        )
        self.assertEqual(len(parsed.malformed), 1)
        self.assertIn("tokenization failed", parsed.malformed[0].reason)

    def test_reports_bare_structured_token(self) -> None:
        parsed = recovery_analysis.parse_text(
            "event=reconnect_completed unexpected-token\n"
        )
        self.assertEqual(len(parsed.malformed), 1)
        self.assertIn("has no '='", parsed.malformed[0].reason)

    def test_reports_duplicate_fields(self) -> None:
        parsed = recovery_analysis.parse_text(
            "event=connected session=S1 session=S2\n"
        )
        self.assertEqual(parsed.malformed[0].reason, "duplicate field: session")

    def test_filters_events_by_name(self) -> None:
        parsed = recovery_analysis.parse_text(
            "event=client_hello session=S1\n"
            "event=frame_received frame=0\n"
            "event=client_hello session=S2\n"
        )
        self.assertEqual(
            [event.fields["session"] for event in parsed.events_named("client_hello")],
            ["S1", "S2"],
        )

    def test_require_clean_includes_line_number(self) -> None:
        parsed = recovery_analysis.parse_text(
            "ordinary output\n"
            "event=connected invalid\n"
        )
        with self.assertRaisesRegex(ValueError, "line 2"):
            parsed.require_clean()

    def test_parse_file(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "client.log"
            path.write_text(
                "event=media_run_completed sessions=2 completed=true\n",
                encoding="utf-8",
            )
            parsed = recovery_analysis.parse_file(path)
        self.assertEqual(parsed.events[0].get_int("sessions"), 2)


if __name__ == "__main__":
    unittest.main()
