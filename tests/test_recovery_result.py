import json
import tempfile
import unittest
from pathlib import Path

from scripts.mininet.recovery_analysis import parse_text
from scripts.mininet.recovery_result import (
    RESULT_SCHEMA_VERSION,
    build_recovery_run_result,
    write_recovery_run_result,
)


class RecoveryRunResultTest(unittest.TestCase):
    def test_builds_complete_reconnect_result(self) -> None:
        client = parse_text(
            """
event=recovery_strategy_config strategy=reconnect
event=media_run_created media_run=R1 expected_frames=6
event=session_created session=S1
event=connected session=S1 connection=101 local=10.0.1.2:5000
event=hello_sent media_run=R1 session=S1
event=reconnect_started elapsed_ms=2000
event=session_created session=S2
event=connected session=S2 connection=202 local=10.0.2.2:6000
event=hello_sent media_run=R1 session=S2
event=reconnect_completed elapsed_ms=2003
event=media_resumed_after_reconnect last_frame_before_reconnect=2 first_frame_after_reconnect=3 skipped_frames=0
event=media_run_completed media_run=R1 final_frame_exclusive=6 sessions=2
"""
        )
        server = parse_text(
            """
event=client_hello media_run=R1 session=S1 connection=101
event=jpeg_frame_validated session=S1 frame=0 received_at_ms=1000
event=jpeg_frame_validated session=S1 frame=1 received_at_ms=1100
event=jpeg_frame_validated session=S1 frame=2 received_at_ms=1200
event=client_hello media_run=R1 session=S2 connection=202
event=jpeg_frame_validated session=S2 frame=3 received_at_ms=1500
event=jpeg_frame_validated session=S2 frame=4 received_at_ms=1600
event=jpeg_frame_validated session=S2 frame=5 received_at_ms=1700
event=jpeg_video_done media_run=R1 session=S2 final_frame_exclusive=6
"""
        )

        result = build_recovery_run_result(client, server)

        self.assertEqual(result.schema_version, RESULT_SCHEMA_VERSION)
        self.assertEqual(result.strategy, "reconnect")
        self.assertEqual(result.media_run_id, "R1")
        self.assertTrue(result.successful)
        self.assertEqual(result.analysis_errors, [])
        self.assertEqual(result.frames.received_unique_frames, 6)
        self.assertEqual(result.frames.missing_frame_ids, [])
        self.assertEqual(result.timing.recovery_action_duration_ms, 3.0)
        self.assertEqual(result.continuity.largest_receive_gap_ms, 300.0)

    def test_collects_component_errors_with_prefixes(self) -> None:
        client = parse_text(
            """
event=recovery_strategy_config strategy=reconnect
event=media_run_created media_run=R1 expected_frames=2
event=session_created session=S1
event=connected session=S1 connection=101 local=10.0.1.2:5000
"""
        )
        server = parse_text("")

        result = build_recovery_run_result(client, server)

        self.assertFalse(result.successful)
        self.assertTrue(
            any(error.startswith("identity:") for error in result.analysis_errors)
        )
        self.assertTrue(
            any(error.startswith("timing:") for error in result.analysis_errors)
        )
        self.assertTrue(
            any(error.startswith("continuity:") for error in result.analysis_errors)
        )

    def test_serializes_stable_json_shape(self) -> None:
        client = parse_text(
            """
event=recovery_strategy_config strategy=migrate
event=media_run_created media_run=R1 expected_frames=2
event=session_created session=S1
event=connected session=S1 connection=101 local=10.0.1.2:5000
event=hello_sent media_run=R1 session=S1
event=automatic_rebind_started elapsed_ms=1000
event=migration_confirmed elapsed_ms=1100
event=media_run_completed media_run=R1 final_frame_exclusive=2 sessions=1
"""
        )
        server = parse_text(
            """
event=client_hello media_run=R1 session=S1 connection=101
event=jpeg_frame_validated session=S1 frame=0 received_at_ms=1000
event=jpeg_frame_validated session=S1 frame=1 received_at_ms=1100
event=jpeg_video_done media_run=R1 session=S1 final_frame_exclusive=2
"""
        )
        result = build_recovery_run_result(client, server)

        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "result.json"
            write_recovery_run_result(result, output)
            data = json.loads(output.read_text(encoding="utf-8"))

        self.assertEqual(data["schema_version"], RESULT_SCHEMA_VERSION)
        self.assertEqual(data["strategy"], "migrate")
        self.assertEqual(data["media_run_id"], "R1")
        self.assertIn("identity", data)
        self.assertIn("frames", data)
        self.assertIn("timing", data)
        self.assertIn("continuity", data)
        self.assertIn("analysis_errors", data)

    def test_success_requires_completion_and_clean_analysis(self) -> None:
        client = parse_text(
            """
event=recovery_strategy_config strategy=migrate
event=media_run_created media_run=R1 expected_frames=1
event=session_created session=S1
event=connected session=S1 connection=101 local=10.0.1.2:5000
event=hello_sent media_run=R1 session=S1
event=automatic_rebind_started elapsed_ms=1000
event=migration_confirmed elapsed_ms=1100
"""
        )
        server = parse_text(
            """
event=client_hello media_run=R1 session=S1 connection=101
event=jpeg_frame_validated session=S1 frame=0 received_at_ms=1000
"""
        )

        result = build_recovery_run_result(client, server)

        self.assertFalse(result.successful)
        self.assertTrue(result.analysis_errors)


if __name__ == "__main__":
    unittest.main()
