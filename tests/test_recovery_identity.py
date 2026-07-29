import unittest

from scripts.mininet.recovery_analysis import parse_text
from scripts.mininet.recovery_identity import extract_recovery_run_identity


MIGRATE_CLIENT = """
event=recovery_strategy_config strategy=migrate
event=media_run_created media_run=R1 fps=10 duration_seconds=6 expected_frames=60
event=client_endpoint_created bind=10.0.1.2:0 local=10.0.1.2:50000
event=session_created session=S1
event=connected session=S1 connection=101 local=10.0.1.2:50000 remote=10.0.0.1:4433
event=hello_sent media_run=R1 session=S1
event=endpoint_rebound old_local=10.0.1.2:50000 new_local=10.0.2.2:50001 mode=automatic
event=migration_confirmed media_run=R1 session=S1 connection=101
event=media_run_completed media_run=R1 sessions=1 completed=true final_frame_exclusive=60
"""

MIGRATE_SERVER = """
event=client_hello media_run=R1 session=S1 connection=201
event=jpeg_video_done media_run=R1 session=S1 final_frame_exclusive=60
"""

RECONNECT_CLIENT = """
event=recovery_strategy_config strategy=reconnect
event=media_run_created media_run=R1 fps=10 duration_seconds=6 expected_frames=60
event=client_endpoint_created bind=10.0.1.2:0 local=10.0.1.2:50250
event=session_created session=S1
event=connected session=S1 connection=101 local=10.0.1.2:50250 remote=10.0.0.1:4433
event=hello_sent media_run=R1 session=S1
event=reconnect_started media_run=R1 old_session=S1 candidate_local=10.0.2.2:0
event=client_endpoint_created bind=10.0.2.2:0 local=10.0.2.2:51234
event=session_created session=S2
event=connected session=S2 connection=202 local=10.0.2.2:51234 remote=10.0.0.1:4433
event=hello_sent media_run=R1 session=S2
event=reconnect_completed media_run=R1 old_session=S1 new_session=S2 old_connection=101 new_connection=202 old_local=10.0.1.2:50250 new_local=10.0.2.2:51234
event=media_run_completed media_run=R1 sessions=2 completed=true final_frame_exclusive=60
"""

RECONNECT_SERVER = """
event=client_hello media_run=R1 session=S1 connection=301
event=client_hello media_run=R1 session=S2 connection=302
event=jpeg_video_done media_run=R1 session=S2 final_frame_exclusive=60
"""


class RecoveryIdentityTest(unittest.TestCase):
    def test_extracts_migration_identity(self) -> None:
        result = extract_recovery_run_identity(
            parse_text(MIGRATE_CLIENT),
            parse_text(MIGRATE_SERVER),
        )
        self.assertEqual(result.strategy, "migrate")
        self.assertEqual(result.media_run_id, "R1")
        self.assertEqual(result.expected_frames, 60)
        self.assertEqual(result.final_frame_exclusive, 60)
        self.assertEqual(result.session_ids, ["S1"])
        self.assertEqual(result.connection_ids, [101])
        self.assertEqual(result.hello_sessions, ["S1"])
        self.assertEqual(result.initial_local_address, "10.0.1.2:50000")
        self.assertEqual(result.recovered_local_address, "10.0.2.2:50001")
        self.assertTrue(result.completed)
        self.assertEqual(result.reported_session_count, 1)
        self.assertEqual(result.analysis_errors, [])

    def test_extracts_reconnect_identity(self) -> None:
        result = extract_recovery_run_identity(
            parse_text(RECONNECT_CLIENT),
            parse_text(RECONNECT_SERVER),
        )
        self.assertEqual(result.strategy, "reconnect")
        self.assertEqual(result.media_run_id, "R1")
        self.assertEqual(result.session_ids, ["S1", "S2"])
        self.assertEqual(result.connection_ids, [101, 202])
        self.assertEqual(result.hello_sessions, ["S1", "S2"])
        self.assertEqual(result.initial_local_address, "10.0.1.2:50250")
        self.assertEqual(result.recovered_local_address, "10.0.2.2:51234")
        self.assertTrue(result.completed)
        self.assertEqual(result.reported_session_count, 2)
        self.assertEqual(result.analysis_errors, [])

    def test_uses_client_hello_events_without_server_log(self) -> None:
        result = extract_recovery_run_identity(parse_text(RECONNECT_CLIENT))
        self.assertEqual(result.hello_sessions, ["S1", "S2"])

    def test_falls_back_to_session_created(self) -> None:
        client = parse_text(
            """
event=recovery_strategy_config strategy=migrate
event=media_run_created media_run=R1 expected_frames=10
event=session_created session=S1
event=hello_sent media_run=R1 session=S1
event=media_run_completed media_run=R1 sessions=1 completed=true
"""
        )
        result = extract_recovery_run_identity(client)
        self.assertEqual(result.session_ids, ["S1"])
        self.assertIn("migration expected 1 connection, found 0", result.analysis_errors)

    def test_reports_missing_completion_without_guessing(self) -> None:
        client = parse_text(
            """
event=recovery_strategy_config strategy=migrate
event=media_run_created media_run=R1 expected_frames=10
event=connected session=S1 connection=1 local=10.0.1.2:5000
event=hello_sent media_run=R1 session=S1
"""
        )
        result = extract_recovery_run_identity(client)
        self.assertFalse(result.completed)
        self.assertIsNone(result.reported_session_count)

    def test_reconnect_requires_distinct_connections(self) -> None:
        client = parse_text(
            RECONNECT_CLIENT.replace(
                "session=S2 connection=202",
                "session=S2 connection=101",
            )
        )
        result = extract_recovery_run_identity(
            client,
            parse_text(RECONNECT_SERVER),
        )
        self.assertIn(
            "reconnect expected at least 2 distinct connections, found 1",
            result.analysis_errors,
        )

    def test_server_hello_evidence_takes_precedence(self) -> None:
        server = parse_text(
            """
event=client_hello media_run=R1 session=S1
event=client_hello media_run=R1 session=S2
"""
        )
        result = extract_recovery_run_identity(
            parse_text(RECONNECT_CLIENT),
            server,
        )
        self.assertEqual(result.hello_sessions, ["S1", "S2"])


if __name__ == "__main__":
    unittest.main()
