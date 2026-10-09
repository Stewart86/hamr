import copy
import io
import json
import multiprocessing
import os
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import urllib.error
from contextlib import redirect_stdout
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from unittest.mock import patch

import handler


PAYLOAD = {
    "data": copy.deepcopy(handler.MOCK_SNAPSHOT["matches"]),
    "meta": {"has_more": False, "total": 1},
}


def refresh_worker(directory, address):
    os.environ["XDG_CACHE_HOME"] = directory
    os.environ.pop("HAMR_TEST_MODE", None)
    handler.API_URL = address
    handler.get_state("test-credential")


class EnvironmentTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.environment = patch.dict(os.environ, {
            "XDG_CACHE_HOME": self.temporary.name,
            "LIVETENNIS_API_KEY": "test-credential",
            "HAMR_TEST_MODE": "0",
        })
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def state_path(self):
        return handler.cache_paths("test-credential")[1]

    def cache(self, state):
        path = self.state_path()
        path.parent.mkdir(parents=True, exist_ok=True)
        handler.write_state(path, state)

class SnapshotTests(EnvironmentTests):
    def test_player_major_games_and_nullable_points(self):
        score = {"games": [[6, 3], [4, 4]], "points": [None, "30"], "server": 2}
        self.assertEqual(handler.score_text(score), "6-4 3-4 | Points: ?-30 | Player 2 serving")

    def test_withheld_and_missing_scores_are_not_zero(self):
        for score, expected in ((None, "Score unavailable"), ({"games": None}, "Score withheld"),
                                ({"games": []}, "Score unavailable"), ({"games": [[], []]}, "Score unavailable")):
            with self.subTest(score=score):
                self.assertEqual(handler.score_text(score), expected)

    def test_unrepresentable_cached_times_fail_before_network_access(self):
        for field in ("attempted_at", "fetched_at"):
            state = {"version": 1, "attempted_at": 10000, "fetched_at": 10000,
                     "snapshot": handler.MOCK_SNAPSHOT, "error": ""}
            state[field] = 1e40
            self.cache(state)
            with self.subTest(field=field), patch.object(handler, "fetch_snapshot") as fetch:
                with self.assertRaises(handler.SnapshotError):
                    handler.get_state("test-credential")
                fetch.assert_not_called()

    def test_failed_first_request_does_not_claim_an_empty_slate(self):
        with patch.object(handler, "fetch_snapshot", side_effect=handler.SnapshotError("The snapshot request failed.")):
            response = handler.handle_request({"step": "initial"})
        self.assertEqual(response["results"][0]["name"], "Snapshot unavailable")

    def test_invalid_games_are_not_partially_displayed(self):
        for games in ([[6, 3], [4]], [[6, True], [4, 4]], [[-1], [4]], "6-4"):
            with self.subTest(games=games):
                self.assertEqual(handler.score_text({"games": games}), "Score unavailable")

    def test_regular_tiebreak_is_labeled(self):
        score = {"games": [[6], [6]], "points": ["8", "7"], "is_tiebreak": True}
        self.assertEqual(handler.score_text(score), "6-6 | Tiebreak points: 8-7")

    def test_tiebreak_games_are_paired_without_guessing_the_format(self):
        score = {"games": [[6, 4, 10], [4, 6, 5]], "points": ["10", "5"], "is_tiebreak": True}
        self.assertEqual(handler.score_text(score), "6-4 4-6 10-5 | Tiebreak points: 10-5")

    def test_doubles_names_remain_opaque(self):
        payload = copy.deepcopy(PAYLOAD)
        payload["data"][0]["players"]["p1"]["name"] = "Alex Example / Jo Example"
        match = handler.parse_snapshot(payload)["matches"][0]
        self.assertEqual(handler.match_text(match)[0], "Alex Example / Jo Example vs Sam Example")

    def test_authoritative_short_page_pagination(self):
        payload = copy.deepcopy(PAYLOAD)
        payload["meta"] = {"has_more": True, "total": 350}
        snapshot = handler.parse_snapshot(payload)
        status = handler.status_text({"snapshot": snapshot, "fetched_at": 10000, "error": ""})
        self.assertIn("Showing the first 1 of 350 matches.", status)

    def test_nullable_total(self):
        payload = copy.deepcopy(PAYLOAD)
        payload["meta"] = {"has_more": True, "total": None}
        status = handler.status_text({"snapshot": handler.parse_snapshot(payload), "fetched_at": 10000, "error": ""})
        self.assertIn("Showing the first 1 matches.", status)
        self.assertNotIn(" of ", status)

    def test_invalid_payloads_are_rejected(self):
        for payload in ([], {}, {"data": [], "meta": {}}, {"data": [], "meta": {"has_more": 0}},
                        {"data": [], "meta": {"has_more": False, "total": True}},
                        {"data": [{"id": 1}], "meta": {"has_more": False}}):
            with self.subTest(payload=payload), self.assertRaises(handler.SnapshotError):
                handler.parse_snapshot(payload)

    def test_successful_empty_snapshot(self):
        with patch.object(handler, "fetch_snapshot", return_value={"matches": [], "has_more": False, "total": 0}):
            response = handler.handle_request({"step": "initial"})
        self.assertEqual(response["results"][0]["name"], "No live matches")
        self.assertIn("Snapshot from", response["results"][0]["description"])

    def test_cooldown_is_shared_by_queries_and_restarts(self):
        with patch.object(handler, "fetch_snapshot", return_value=handler.MOCK_SNAPSHOT) as fetch:
            with patch.object(handler.time, "time", return_value=10000):
                first = handler.handle_request({"step": "initial"})
            with patch.object(handler.time, "time", return_value=10899):
                second = handler.handle_request({"step": "search", "query": "alex"})
                stored = handler.read_state(self.state_path())
        self.assertEqual(fetch.call_count, 1)
        self.assertEqual(first["results"][0], second["results"][0])
        self.assertEqual(stored["attempted_at"], 10000)
        self.assertNotIn("test-credential", self.state_path().read_text())
        self.assertNotIn("test-credential", str(self.state_path()))

    def test_refresh_at_exact_floor(self):
        with patch.object(handler, "fetch_snapshot", return_value=handler.MOCK_SNAPSHOT) as fetch:
            with patch.object(handler.time, "time", return_value=10000):
                handler.get_state("test-credential")
            with patch.object(handler.time, "time", return_value=10900):
                handler.get_state("test-credential")
        self.assertEqual(fetch.call_count, 2)

    def test_failures_consume_the_same_cooldown(self):
        with patch.object(handler, "fetch_snapshot", side_effect=handler.SnapshotError("The snapshot request failed.")) as fetch:
            with patch.object(handler.time, "time", return_value=10000):
                state = handler.get_state("test-credential")
            with patch.object(handler.time, "time", return_value=10899):
                handler.get_state("test-credential")
        self.assertEqual(fetch.call_count, 1)
        self.assertIsNone(state["snapshot"])
        self.assertEqual(handler.read_state(self.state_path())["attempted_at"], 10000)

    def test_failed_refresh_preserves_previous_snapshot(self):
        with patch.object(handler, "fetch_snapshot", return_value=handler.MOCK_SNAPSHOT):
            with patch.object(handler.time, "time", return_value=10000):
                handler.get_state("test-credential")
        with patch.object(handler, "fetch_snapshot", side_effect=handler.SnapshotError("The snapshot request failed.")):
            with patch.object(handler.time, "time", return_value=11000):
                state = handler.get_state("test-credential")
        self.assertEqual(state["snapshot"], handler.MOCK_SNAPSHOT)
        self.assertEqual(state["fetched_at"], 10000)
        self.assertIn("failed", handler.status_text(state))
        self.assertIn("1970-01-01 02:46 UTC", handler.status_text(state))

    def test_attempt_is_durable_before_fetch(self):
        def fetch(_credential):
            self.assertEqual(handler.read_state(self.state_path())["attempted_at"], 10000)
            self.assertIn("did not finish", handler.read_state(self.state_path())["error"])
            return handler.MOCK_SNAPSHOT
        with patch.object(handler, "fetch_snapshot", side_effect=fetch):
            with patch.object(handler.time, "time", return_value=10000):
                handler.get_state("test-credential")
        self.assertEqual(self.state_path().stat().st_mode & 0o777, 0o600)

    def test_failed_attempt_save_sends_no_request(self):
        with patch.object(handler, "write_state", side_effect=OSError), patch.object(handler, "fetch_snapshot") as fetch:
            with self.assertRaises(handler.SnapshotError):
                handler.get_state("test-credential")
        fetch.assert_not_called()

    def test_crash_after_attempt_save_keeps_cooldown(self):
        with patch.object(handler, "fetch_snapshot", side_effect=SystemExit):
            with patch.object(handler.time, "time", return_value=10000), self.assertRaises(SystemExit):
                handler.get_state("test-credential")
        with patch.object(handler, "fetch_snapshot") as fetch:
            with patch.object(handler.time, "time", return_value=10100):
                state = handler.get_state("test-credential")
        fetch.assert_not_called()
        self.assertIn("did not finish", state["error"])

    def test_malformed_cache_never_refreshes(self):
        path = self.state_path()
        path.parent.mkdir(parents=True)
        for data in ("{", "[]", '{"version":1,"attempted_at":null}',
                     '{"version":1,"attempted_at":true}', '{"version":1,"attempted_at":NaN}'):
            with self.subTest(data=data):
                path.write_text(data)
                with patch.object(handler, "fetch_snapshot") as fetch, self.assertRaises(handler.SnapshotError):
                    handler.get_state("test-credential")
                fetch.assert_not_called()

    def test_missing_cached_fields_fail_before_network_access(self):
        for field in ("attempted_at", "fetched_at", "snapshot", "error"):
            state = {"version": 1, "attempted_at": 10000, "fetched_at": None,
                     "snapshot": None, "error": ""}
            del state[field]
            self.cache(state)
            with self.subTest(field=field), patch.object(handler, "fetch_snapshot") as fetch:
                with self.assertRaises(handler.SnapshotError):
                    handler.get_state("test-credential")
                fetch.assert_not_called()

    def test_inconsistent_cached_snapshot_fails_before_network_access(self):
        for snapshot, fetched in ((handler.MOCK_SNAPSHOT, None), (None, 10000)):
            state = {"version": 1, "attempted_at": 10000, "fetched_at": fetched,
                     "snapshot": snapshot, "error": ""}
            self.cache(state)
            with self.subTest(snapshot=snapshot), patch.object(handler, "fetch_snapshot") as fetch:
                with self.assertRaises(handler.SnapshotError):
                    handler.get_state("test-credential")
                fetch.assert_not_called()

    def test_clock_rollback_does_not_bypass_cooldown(self):
        with patch.object(handler, "fetch_snapshot", return_value=handler.MOCK_SNAPSHOT) as fetch:
            with patch.object(handler.time, "time", return_value=10000):
                handler.get_state("test-credential")
            with patch.object(handler.time, "time", return_value=100):
                handler.get_state("test-credential")
        self.assertEqual(fetch.call_count, 1)

    def test_copy_does_not_fetch(self):
        with patch.object(handler, "fetch_snapshot", return_value=handler.MOCK_SNAPSHOT):
            handler.get_state("test-credential")
        with patch.object(handler, "fetch_snapshot") as fetch:
            response = handler.handle_request({"step": "action", "selected": {"id": "1"}})
        fetch.assert_not_called()
        self.assertEqual(response["type"], "execute")
        self.assertIn("6-4 3-4", response["copy"])
        self.assertIn("Snapshot from", response["copy"])

    def test_status_action_does_not_fetch(self):
        with patch.object(handler, "fetch_snapshot") as fetch:
            self.assertEqual(handler.handle_request({"step": "action", "selected": {"id": "__status__"}}), {"type": "noop"})
        fetch.assert_not_called()

    def test_missing_credential_sends_no_request(self):
        with patch.dict(os.environ, {"LIVETENNIS_API_KEY": ""}), patch.object(handler, "fetch_snapshot") as fetch:
            with self.assertRaisesRegex(handler.SnapshotError, "LIVETENNIS_API_KEY"):
                handler.handle_request({"step": "initial"})
        fetch.assert_not_called()
        self.assertFalse(self.state_path().exists())

    def test_test_mode_without_credentials_or_cache(self):
        with patch.dict(os.environ, {"HAMR_TEST_MODE": "1", "LIVETENNIS_API_KEY": ""}), patch.object(handler, "fetch_snapshot") as fetch:
            response = handler.handle_request({"step": "search", "query": "example"})
        self.assertEqual(response["results"][0]["id"], "1")
        fetch.assert_not_called()
        self.assertFalse(self.state_path().exists())

    def test_public_stdio_test_mode(self):
        environment = os.environ.copy()
        environment["HAMR_TEST_MODE"] = "1"
        environment.pop("LIVETENNIS_API_KEY", None)
        for request, expected in (({"step": "initial"}, "results"),
                                  ({"step": "search", "query": "sam"}, "results"),
                                  ({"step": "action", "selected": {"id": "1"}}, "execute")):
            with self.subTest(request=request):
                process = subprocess.run([sys.executable, "-B", str(Path(handler.__file__))],
                                         input=json.dumps(request), capture_output=True, text=True, env=environment, check=True)
                self.assertEqual(json.loads(process.stdout)["type"], expected)
                self.assertEqual(process.stderr, "")

    def test_public_stdio_invalid_request(self):
        for data in ("{", "[]", '{"step":"action","selected":42}'):
            with self.subTest(data=data), patch.object(handler.sys, "stdin", io.StringIO(data)), redirect_stdout(io.StringIO()) as output:
                handler.main()
                self.assertEqual(json.loads(output.getvalue())["type"], "error")

    def test_control_characters_are_removed(self):
        self.assertEqual(handler.clean_text("Alex\x1b\nExample"), "Alex Example")


class TransportTests(EnvironmentTests):
    def start_server(self, mode="success", delay=0):
        calls = []
        class HTTPHandler(BaseHTTPRequestHandler):
            def do_GET(self):
                calls.append((self.path, self.headers.get("X-API-Key")))
                if delay:
                    time.sleep(delay)
                if mode == "redirect":
                    self.send_response(302)
                    self.send_header("Location", "/unexpected")
                    self.end_headers()
                    return
                self.send_response(200 if mode not in ("401", "403", "429", "500") else int(mode))
                self.end_headers()
                if mode == "invalid":
                    self.wfile.write(b"invalid JSON")
                elif mode == "oversized":
                    self.wfile.write(b" " * (handler.MAX_RESPONSE_BYTES + 1))
                else:
                    self.wfile.write(json.dumps(PAYLOAD).encode())
            def log_message(self, *_args):
                pass
        server = ThreadingHTTPServer(("127.0.0.1", 0), HTTPHandler)
        server.daemon_threads = True
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        address = f"http://127.0.0.1:{server.server_port}/matches?status=live&limit=200"
        endpoint = patch.object(handler, "API_URL", address)
        endpoint.start()
        self.addCleanup(endpoint.stop)
        return calls, address

    def test_actual_transport_uses_header_and_only_free_query(self):
        calls, _address = self.start_server()
        self.assertEqual(handler.fetch_snapshot("test-credential"), handler.MOCK_SNAPSHOT)
        self.assertEqual(calls, [("/matches?status=live&limit=200", "test-credential")])

    def test_redirects_do_not_add_a_request(self):
        calls, _address = self.start_server("redirect")
        with self.assertRaises(handler.SnapshotError):
            handler.fetch_snapshot("test-credential")
        self.assertEqual(len(calls), 1)

    def test_errors_do_not_retry_or_expose_credentials(self):
        for mode in ("401", "403", "429", "500", "invalid", "oversized"):
            with self.subTest(mode=mode):
                calls, _address = self.start_server(mode)
                with self.assertRaises(handler.SnapshotError) as error:
                    handler.fetch_snapshot("test-credential")
                self.assertNotIn("test-credential", str(error.exception))
                self.assertEqual(len(calls), 1)

    def test_process_restarts_keep_the_same_floor(self):
        calls, address = self.start_server()
        context = multiprocessing.get_context("spawn")
        for _ in range(2):
            process = context.Process(target=refresh_worker, args=(self.temporary.name, address))
            process.start()
            process.join(5)
            self.assertEqual(process.exitcode, 0)
        self.assertEqual(len(calls), 1)

    def test_concurrent_processes_send_one_request(self):
        calls, address = self.start_server(delay=0.2)
        context = multiprocessing.get_context("spawn")
        processes = [context.Process(target=refresh_worker, args=(self.temporary.name, address)) for _ in range(5)]
        for process in processes:
            process.start()
        for process in processes:
            process.join(5)
            self.assertEqual(process.exitcode, 0)
        self.assertEqual(len(calls), 1)


if __name__ == "__main__":
    unittest.main()
