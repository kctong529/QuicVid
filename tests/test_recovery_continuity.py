import unittest

from scripts.mininet.recovery_analysis import parse_text
from scripts.mininet.recovery_continuity import measure_frame_continuity
from scripts.mininet.recovery_identity import RecoveryRunIdentity, SessionIdentity


def identity(strategy: str = "reconnect") -> RecoveryRunIdentity:
    sessions = [SessionIdentity("S1", 101)]
    hello_sessions = ["S1"]
    if strategy == "reconnect":
        sessions.append(SessionIdentity("S2", 202))
        hello_sessions.append("S2")

    return RecoveryRunIdentity(
        strategy=strategy,
        media_run_id="R1",
        expected_frames=10,
        final_frame_exclusive=10,
        sessions=sessions,
        hello_sessions=hello_sessions,
        completed=True,
        reported_session_count=len(sessions),
    )


class RecoveryContinuityTest(unittest.TestCase):
    def test_measures_reconnect_boundary(self) -> None:
        client = parse_text(
            """
event=media_resumed_after_reconnect \
last_frame_before_reconnect=19 \
first_frame_after_reconnect=27 \
skipped_frames=7
"""
        )
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=19 received_at_ms=1900
event=jpeg_frame_validated session=S2 frame=27 received_at_ms=2700
"""
        )

        result = measure_frame_continuity(
            client,
            server,
            identity("reconnect"),
        )

        self.assertEqual(result.last_frame_before_recovery, 19)
        self.assertEqual(result.first_frame_after_recovery, 27)
        self.assertEqual(result.skipped_frames, 7)
        self.assertEqual(result.largest_frame_id_gap, 7)
        self.assertEqual(result.largest_frame_id_gap_start, 20)
        self.assertEqual(result.largest_frame_id_gap_end, 26)
        self.assertEqual(result.largest_receive_gap_ms, 800.0)
        self.assertEqual(result.analysis_errors, [])

    def test_derives_skipped_frames_when_not_logged(self) -> None:
        client = parse_text(
            """
event=media_resumed_after_reconnect \
last_frame_before_reconnect=10 \
first_frame_after_reconnect=13
"""
        )
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=10 received_at_ms=1000
event=jpeg_frame_validated session=S2 frame=13 received_at_ms=1300
"""
        )

        result = measure_frame_continuity(
            client,
            server,
            identity("reconnect"),
        )

        self.assertEqual(result.skipped_frames, 2)
        self.assertEqual(result.analysis_errors, [])

    def test_reports_inconsistent_skipped_frames(self) -> None:
        client = parse_text(
            """
event=media_resumed_after_reconnect \
last_frame_before_reconnect=10 \
first_frame_after_reconnect=13 \
skipped_frames=1
"""
        )
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=10 received_at_ms=1000
event=jpeg_frame_validated session=S2 frame=13 received_at_ms=1300
"""
        )

        result = measure_frame_continuity(
            client,
            server,
            identity("reconnect"),
        )

        self.assertIn(
            "reported skipped_frames does not match frame boundary",
            result.analysis_errors[0],
        )

    def test_counts_out_of_order_frames(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=0 received_at_ms=0
event=jpeg_frame_validated session=S1 frame=2 received_at_ms=100
event=jpeg_frame_validated session=S1 frame=1 received_at_ms=200
event=jpeg_frame_validated session=S1 frame=3 received_at_ms=300
"""
        )

        result = measure_frame_continuity(
            parse_text(""),
            server,
            identity("migrate"),
        )

        self.assertEqual(result.out_of_order_frames, 1)
        self.assertEqual(result.largest_frame_id_gap, 1)

    def test_ignores_other_sessions(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=0 received_at_ms=0
event=jpeg_frame_validated session=OTHER frame=9 received_at_ms=50
event=jpeg_frame_validated session=S1 frame=1 received_at_ms=100
"""
        )

        result = measure_frame_continuity(
            parse_text(""),
            server,
            identity("migrate"),
        )

        self.assertEqual(result.largest_frame_id_gap, 0)
        self.assertEqual(result.out_of_order_frames, 0)
        self.assertEqual(result.largest_receive_gap_ms, 100.0)

    def test_reports_receive_gap_unavailable_without_receiver_timestamps(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=0
event=jpeg_frame_validated session=S1 frame=1
"""
        )

        result = measure_frame_continuity(
            parse_text(""),
            server,
            identity("migrate"),
        )

        self.assertIsNone(result.largest_receive_gap_ms)
        self.assertEqual(result.receive_gap_sample_count, 0)
        self.assertIn(
            "largest_receive_gap_ms unavailable",
            result.analysis_errors[0],
        )

    def test_does_not_use_sent_at_ms_as_receive_time(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=0 sent_at_ms=1000
event=jpeg_frame_validated session=S1 frame=1 sent_at_ms=1100
"""
        )

        result = measure_frame_continuity(
            parse_text(""),
            server,
            identity("migrate"),
        )

        self.assertIsNone(result.largest_receive_gap_ms)
        self.assertIn(
            "largest_receive_gap_ms unavailable",
            result.analysis_errors[0],
        )

    def test_reports_missing_reconnect_boundary(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=0 received_at_ms=0
event=jpeg_frame_validated session=S2 frame=1 received_at_ms=100
"""
        )

        result = measure_frame_continuity(
            parse_text(""),
            server,
            identity("reconnect"),
        )

        self.assertIn(
            "missing media_resumed_after_reconnect",
            result.analysis_errors,
        )

    def test_rejects_timeline_restart(self) -> None:
        client = parse_text(
            """
event=media_resumed_after_reconnect \
last_frame_before_reconnect=19 \
first_frame_after_reconnect=0 \
skipped_frames=0
"""
        )
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=19 received_at_ms=1900
event=jpeg_frame_validated session=S2 frame=0 received_at_ms=2000
"""
        )

        result = measure_frame_continuity(
            client,
            server,
            identity("reconnect"),
        )

        self.assertIn(
            "first post-reconnect frame does not advance the media timeline",
            result.analysis_errors,
        )


if __name__ == "__main__":
    unittest.main()
