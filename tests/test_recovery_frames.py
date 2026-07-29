import unittest

from scripts.mininet.recovery_analysis import parse_text
from scripts.mininet.recovery_frames import aggregate_recovery_frames
from scripts.mininet.recovery_identity import (
    RecoveryRunIdentity,
    SessionIdentity,
)


def identity(
    *,
    media_run_id: str = "R1",
    final_frame_exclusive: int | None = 8,
) -> RecoveryRunIdentity:
    return RecoveryRunIdentity(
        strategy="reconnect",
        media_run_id=media_run_id,
        expected_frames=final_frame_exclusive,
        final_frame_exclusive=final_frame_exclusive,
        sessions=[
            SessionIdentity("S1", 101, "10.0.1.2:5000"),
            SessionIdentity("S2", 202, "10.0.2.2:6000"),
        ],
        hello_sessions=["S1", "S2"],
        completed=True,
        reported_session_count=2,
    )


class RecoveryFrameAggregationTest(unittest.TestCase):
    def test_unions_frames_across_sessions(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated media_run=R1 session=S1 frame=0
event=jpeg_frame_validated media_run=R1 session=S1 frame=1
event=jpeg_frame_validated media_run=R1 session=S1 frame=2
event=jpeg_frame_validated media_run=R1 session=S1 frame=3
event=jpeg_frame_validated media_run=R1 session=S2 frame=5
event=jpeg_frame_validated media_run=R1 session=S2 frame=6
event=jpeg_frame_validated media_run=R1 session=S2 frame=7
"""
        )

        result = aggregate_recovery_frames(server, identity())

        self.assertEqual(result.received_frame_ids, [0, 1, 2, 3, 5, 6, 7])
        self.assertEqual(result.received_unique_frames, 7)
        self.assertEqual(result.missing_frame_ids, [4])
        self.assertEqual(result.missing_frames, 1)
        self.assertEqual(result.duplicate_frames, 0)
        self.assertEqual(result.per_session_received, {"S1": 4, "S2": 3})
        self.assertEqual(result.analysis_errors, [])

    def test_counts_duplicates_across_sessions(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated media_run=R1 session=S1 frame_id=0
event=jpeg_frame_validated media_run=R1 session=S1 frame_id=1
event=jpeg_frame_validated media_run=R1 session=S2 frame_id=1
event=jpeg_frame_validated media_run=R1 session=S2 frame_id=2
"""
        )

        result = aggregate_recovery_frames(
            server,
            identity(final_frame_exclusive=3),
        )

        self.assertEqual(result.received_unique_frames, 3)
        self.assertEqual(result.duplicate_frame_ids, [1])
        self.assertEqual(result.duplicate_frames, 1)
        self.assertEqual(result.missing_frames, 0)

    def test_accepts_session_scoped_events_without_media_run(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated session=S1 frame=0
event=jpeg_frame_validated session=S2 frame=1
event=jpeg_frame_validated session=OTHER frame=2
"""
        )

        result = aggregate_recovery_frames(
            server,
            identity(final_frame_exclusive=3),
        )

        self.assertEqual(result.received_frame_ids, [0, 1])
        self.assertEqual(result.missing_frame_ids, [2])
        self.assertEqual(result.ignored_frame_events, 1)

    def test_ignores_other_media_runs(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated media_run=R1 session=S1 frame=0
event=jpeg_frame_validated media_run=R2 session=S9 frame=1
"""
        )

        result = aggregate_recovery_frames(
            server,
            identity(final_frame_exclusive=2),
        )

        self.assertEqual(result.received_frame_ids, [0])
        self.assertEqual(result.missing_frame_ids, [1])
        self.assertEqual(result.ignored_frame_events, 1)

    def test_does_not_use_session_local_summary(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated media_run=R1 session=S1 frame=0
event=jpeg_frame_validated media_run=R1 session=S1 frame=1
event=jpeg_frame_validated media_run=R1 session=S2 frame=2
event=media_run_completed media_run=R1 session=S2 expected=3 received=1 missing=2
"""
        )

        result = aggregate_recovery_frames(
            server,
            identity(final_frame_exclusive=3),
        )

        self.assertEqual(result.received_unique_frames, 3)
        self.assertEqual(result.missing_frames, 0)

    def test_reports_frame_event_without_integer_frame_id(self) -> None:
        server = parse_text(
            "event=jpeg_frame_validated media_run=R1 session=S1 frame=bad\n"
        )

        result = aggregate_recovery_frames(
            server,
            identity(final_frame_exclusive=1),
        )

        self.assertEqual(result.ignored_frame_events, 1)
        self.assertIn("has no integer frame ID", result.analysis_errors[0])
        self.assertEqual(result.missing_frame_ids, [0])

    def test_reports_out_of_range_frames(self) -> None:
        server = parse_text(
            """
event=jpeg_frame_validated media_run=R1 session=S1 frame=0
event=jpeg_frame_validated media_run=R1 session=S2 frame=3
"""
        )

        result = aggregate_recovery_frames(
            server,
            identity(final_frame_exclusive=3),
        )

        self.assertEqual(result.missing_frame_ids, [1, 2])
        self.assertIn(
            "received frame IDs outside final range: 3",
            result.analysis_errors,
        )

    def test_requires_media_run_identity(self) -> None:
        run = identity()
        run.media_run_id = None

        result = aggregate_recovery_frames(parse_text(""), run)

        self.assertEqual(
            result.analysis_errors,
            ["cannot aggregate frames without media_run_id"],
        )

    def test_requires_final_frame_boundary_for_missing_count(self) -> None:
        server = parse_text(
            "event=jpeg_frame_validated media_run=R1 session=S1 frame=0\n"
        )

        result = aggregate_recovery_frames(
            server,
            identity(final_frame_exclusive=None),
        )

        self.assertEqual(result.received_unique_frames, 1)
        self.assertIsNone(result.missing_frames)
        self.assertIn(
            "cannot calculate missing frames without final_frame_exclusive",
            result.analysis_errors,
        )


if __name__ == "__main__":
    unittest.main()
