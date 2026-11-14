import importlib.util
import io
import json
import os
import sqlite3
import stat
import sys
import tempfile
import unittest
import urllib.error
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

WORK = tempfile.mkdtemp()
FAKE_CLI = os.path.join(WORK, "fake-cli")
FAKE_PLAN = os.path.join(WORK, "fake-plan")
FAKE_CALLS = os.path.join(WORK, "fake-calls")
Path(FAKE_CLI).write_text(
    """#!/usr/bin/env python3
import json, os, pathlib, sqlite3, sys
args = sys.argv[1:]
db = args[args.index("--db") + 1]
args = args[args.index("--db") + 2:]
calls = pathlib.Path(os.environ["FAKE_CALLS"])
with calls.open("a") as handle:
    handle.write(json.dumps(args) + "\\n")
index = len(calls.read_text().splitlines()) - 1
plan = json.loads(pathlib.Path(os.environ["FAKE_PLAN"]).read_text() or "[]")
step = plan[index] if index < len(plan) else plan[-1]
sys.stdout.write(step.get("output", ""))
if step.get("clear"):
    connection = sqlite3.connect(db)
    with connection:
        connection.execute("DELETE FROM mutation_queue WHERE status != 'failed'")
    connection.close()
sys.exit(step.get("exit", 0))
"""
)
os.chmod(FAKE_CLI, os.stat(FAKE_CLI).st_mode | stat.S_IXUSR)
sys.dont_write_bytecode = True
os.environ["HT_CORE_CLI"] = FAKE_CLI
os.environ["FAKE_PLAN"] = FAKE_PLAN
os.environ["FAKE_CALLS"] = FAKE_CALLS

spec = importlib.util.spec_from_file_location("live_readback", Path(__file__).with_name("live-readback.py"))
gate_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate_module)


def new_gate(rows=()):
    db = os.path.join(tempfile.mkdtemp(dir=WORK), "local.db")
    connection = sqlite3.connect(db)
    connection.execute("CREATE TABLE mutation_queue(id INTEGER PRIMARY KEY, kind TEXT, status TEXT, attempts INTEGER, last_error TEXT)")
    connection.executemany("INSERT INTO mutation_queue(kind,status,attempts,last_error) VALUES(?,?,?,?)", rows)
    connection.commit()
    connection.close()
    gate = gate_module.Gate.__new__(gate_module.Gate)
    gate.db = db
    gate.results = {}
    gate.rate_limit_waits = 0
    return gate


def queue_rows(db):
    connection = sqlite3.connect(db)
    try:
        return connection.execute("SELECT kind FROM mutation_queue").fetchall()
    finally:
        connection.close()


def plan(*steps):
    Path(FAKE_PLAN).write_text(json.dumps(list(steps)))
    Path(FAKE_CALLS).write_text("")


def calls():
    return [json.loads(line) for line in Path(FAKE_CALLS).read_text().splitlines() if line.strip()]


def quiet():
    return redirect_stdout(io.StringIO())


def write_with(rows, *steps, payload=None):
    plan(*steps)
    with mock.patch.object(gate_module.time, "sleep") as slept, quiet():
        gate = new_gate(rows)
        outcome = None
        try:
            gate.write(payload or {"kind": "board"})
        except gate_module.Failure as error:
            outcome = error
        return gate, outcome, slept


class WriteTests(unittest.TestCase):
    def test_a_chained_follow_up_row_applied_in_the_same_call_is_accepted(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Applied(2)\nDRAIN Empty\n"})
        new_gate().write({"kind": "board"})

    def test_a_single_applied_row_is_accepted(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Empty\n"})
        new_gate().write({"kind": "task_title"})

    def test_a_first_row_that_is_not_applied_fails(self):
        for first in ("Retrying(1)", "Failed(1)", "Waiting(9)", "Empty"):
            plan({"output": f"QUEUED 1\nDRAIN {first}\n"})
            with self.assertRaises(gate_module.Failure, msg=first):
                new_gate().write({"kind": "task_title"})

    def test_a_follow_up_row_that_retries_fails(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Retrying(2)\n"})
        with self.assertRaises(gate_module.Failure) as raised:
            new_gate([("task_patch", "pending", 1, "HTTP 500")]).write({"kind": "board"})
        self.assertIn("drain ended", str(raised.exception))

    def test_a_follow_up_row_that_fails_is_a_failure_with_its_own_message(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Failed(2)\nDRAIN Empty\n", "clear": True})
        gate = new_gate([("task_patch", "failed", 1, "HTTP 422")])
        with self.assertRaises(gate_module.Failure):
            gate.write({"kind": "board"})
        with self.assertRaises(gate_module.Failure) as raised:
            gate.require_empty_queue()
        self.assertIn("queue row failed", str(raised.exception))

    def test_a_row_left_pending_after_a_clean_drain_fails(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Empty\n"})
        with self.assertRaises(gate_module.Failure) as raised:
            new_gate([("task_patch", "pending", 0, None)]).write({"kind": "board"})
        self.assertIn("queue not empty", str(raised.exception))

    def test_output_without_a_drain_line_fails(self):
        plan({"output": "QUEUED 1\n"})
        with self.assertRaises(gate_module.Failure):
            new_gate().write({"kind": "task_title"})


class RateLimitWriteTests(unittest.TestCase):
    def test_a_rate_limited_write_waits_for_the_next_window_and_drains_again(self):
        gate, outcome, slept = write_with(
            [("task_comment", "pending", 1, "HTTP 429")],
            {"output": "QUEUED 1\nDRAIN Retrying(1)\n"},
            {"output": "DRAIN Applied(1)\nDRAIN Empty\n", "clear": True},
        )
        self.assertIsNone(outcome)
        self.assertEqual([call[0] for call in calls()], ["write-json", "drain"])
        self.assertEqual(gate.rate_limit_waits, 1)
        self.assertGreaterEqual(slept.call_count, 1)

    def test_a_rate_limit_message_without_a_status_is_also_retried(self):
        gate, outcome, slept = write_with(
            [("compose_task", "pending", 1, "Rate limit exceeded. Please slow down and try again shortly.")],
            {"output": "QUEUED 1\nDRAIN Retrying(1)\n"},
            {"output": "DRAIN Applied(1)\nDRAIN Empty\n", "clear": True},
        )
        self.assertIsNone(outcome)
        self.assertEqual(gate.rate_limit_waits, 1)
        self.assertGreaterEqual(slept.call_count, 1)

    def test_a_retry_that_is_rate_limited_again_keeps_waiting_a_bounded_number_of_times(self):
        retrying = {"output": "DRAIN Retrying(1)\n"}
        first = {"output": "QUEUED 1\nDRAIN Retrying(1)\n"}
        gate, outcome, slept = write_with([("board_write", "pending", 1, "HTTP 429")], first, retrying)
        self.assertIsNotNone(outcome)
        self.assertIn("write was Retrying", str(outcome))
        self.assertEqual(len(calls()), 1 + gate_module.RATE_LIMIT_RETRIES)
        self.assertEqual(gate.rate_limit_waits, gate_module.RATE_LIMIT_RETRIES)

    def test_a_write_that_never_applies_still_fails(self):
        gate, outcome, slept = write_with(
            [("board_write", "pending", 1, "HTTP 429")],
            {"output": "QUEUED 1\nDRAIN Retrying(1)\n"},
            {"output": "DRAIN Retrying(1)\n"},
        )
        self.assertIsNotNone(outcome)

    def test_another_retry_reason_is_not_retried_or_hidden(self):
        gate, outcome, slept = write_with(
            [("task_patch", "pending", 1, "HTTP 500")],
            {"output": "QUEUED 1\nDRAIN Retrying(1)\n"},
            {"output": "DRAIN Applied(1)\nDRAIN Empty\n", "clear": True},
        )
        self.assertIsNotNone(outcome)
        self.assertIn("Retrying", str(outcome))
        self.assertEqual(len(calls()), 1)
        self.assertEqual(gate.rate_limit_waits, 0)

    def test_a_retrying_row_with_no_error_is_not_retried(self):
        gate, outcome, slept = write_with(
            [("task_patch", "pending", 0, None)],
            {"output": "QUEUED 1\nDRAIN Retrying(1)\n"},
        )
        self.assertIsNotNone(outcome)
        self.assertEqual(len(calls()), 1)

    def test_a_rate_limit_retry_wait_is_aligned_to_the_next_window(self):
        self.assertAlmostEqual(gate_module.window_wait(0.0), 62.0)
        self.assertAlmostEqual(gate_module.window_wait(59.0), 3.0)
        self.assertAlmostEqual(gate_module.window_wait(60.0), 62.0)


class SyncTests(unittest.TestCase):
    def test_a_rate_limited_sync_is_retried(self):
        plan(
            {"output": "rate limit exceeded. Please slow down and try again shortly.\n", "exit": 1},
            {"output": "SYNC_OK\n"},
        )
        gate = new_gate()
        with mock.patch.object(gate_module.time, "sleep") as slept, quiet():
            gate.sync()
        self.assertEqual(len(calls()), 2)
        self.assertEqual(gate.rate_limit_waits, 1)
        self.assertGreaterEqual(slept.call_count, 1)

    def test_another_sync_failure_is_not_retried(self):
        plan({"output": "board is unavailable\n", "exit": 1})
        gate = new_gate()
        with mock.patch.object(gate_module.time, "sleep") as slept, quiet():
            with self.assertRaises(gate_module.Failure):
                gate.sync()
        self.assertEqual(len(calls()), 1)
        self.assertEqual(gate.rate_limit_waits, 0)


class FakeResponse:
    def __init__(self, body):
        self.body = body

    def read(self):
        return self.body

    def __enter__(self):
        return self

    def __exit__(self, *_):
        return False


class ApiTests(unittest.TestCase):
    def test_a_429_is_waited_out_and_replayed(self):
        throttled = urllib.error.HTTPError("https://x", 429, "Too Many Requests", {}, io.BytesIO(b'{"message":"Rate limit exceeded."}'))
        with mock.patch.object(gate_module.urllib.request, "urlopen", side_effect=[throttled, FakeResponse(b'{"success":true,"timers":[]}')]):
            with mock.patch.object(gate_module.time, "sleep") as slept, quiet():
                self.assertEqual(gate_module.api("GET", "/time/running"), {"success": True, "timers": []})
        self.assertEqual(slept.call_count, 1)

    def test_a_500_is_not_retried(self):
        broken = urllib.error.HTTPError("https://x", 500, "Server Error", {}, io.BytesIO(b"boom"))
        with mock.patch.object(gate_module.urllib.request, "urlopen", side_effect=[broken, FakeResponse(b'{"success":true}')]) as called:
            with self.assertRaises(gate_module.Failure):
                gate_module.api("GET", "/tasks")
        self.assertEqual(called.call_count, 1)

    def test_a_persistent_429_ends_as_a_failure(self):
        throttled = urllib.error.HTTPError("https://x", 429, "Too Many Requests", {}, io.BytesIO(b"{}"))
        with mock.patch.object(gate_module.urllib.request, "urlopen", side_effect=throttled) as called:
            with mock.patch.object(gate_module.time, "sleep"), quiet():
                with self.assertRaises(gate_module.Failure) as raised:
                    gate_module.api("GET", "/time/running")
        self.assertEqual(called.call_count, gate_module.RATE_LIMIT_RETRIES + 1)
        self.assertIn("429", str(raised.exception))


class ReportTests(unittest.TestCase):
    def test_a_fully_green_run_reports_every_kind(self):
        gate = new_gate()
        gate.results = {name: None for name in gate_module.STEP_NAMES}
        output = io.StringIO()
        with redirect_stdout(output):
            self.assertEqual(gate_module.report(gate), 0)
        self.assertIn(f"KINDS_PASSED {len(gate_module.STEP_NAMES)}/{len(gate_module.STEP_NAMES)}", output.getvalue())

    def test_a_failed_kind_lowers_the_count(self):
        gate = new_gate()
        gate.results = {name: None for name in gate_module.STEP_NAMES}
        gate.results["toggle_timer"] = "Failure: API timer not running"
        output = io.StringIO()
        with redirect_stdout(output):
            self.assertEqual(gate_module.report(gate), 1)
        total = len(gate_module.STEP_NAMES)
        self.assertIn(f"KINDS_PASSED {total - 1}/{total}", output.getvalue())

    def test_a_kind_that_never_ran_does_not_count_as_passed(self):
        gate = new_gate()
        gate.results = {"task_title": None}
        output = io.StringIO()
        with redirect_stdout(output):
            self.assertEqual(gate_module.report(gate), 1)
        self.assertIn(f"KINDS_PASSED 1/{len(gate_module.STEP_NAMES)}", output.getvalue())

    def test_reopen_failures_are_counted_separately_and_still_fail_the_run(self):
        gate = new_gate()
        gate.results = {name: None for name in gate_module.STEP_NAMES}
        gate.results["reopen:after_resync:create_label"] = "Task label"
        output = io.StringIO()
        with redirect_stdout(output):
            self.assertEqual(gate_module.report(gate), 1)
        printed = output.getvalue()
        self.assertIn(f"KINDS_PASSED {len(gate_module.STEP_NAMES)}/{len(gate_module.STEP_NAMES)}", printed)
        self.assertIn("REOPEN_FAILED 1", printed)

    def test_the_step_list_has_no_duplicates(self):
        self.assertEqual(len(set(gate_module.STEP_NAMES)), len(gate_module.STEP_NAMES))


class CrashTests(unittest.TestCase):
    class Gate:
        def __init__(self):
            self.results = {}
            self.rate_limit_waits = 0

    def run_main(self, execute):
        output = io.StringIO()
        with mock.patch.object(gate_module, "TOKEN", "token"), \
                mock.patch.object(gate_module, "Gate", CrashTests.Gate), \
                mock.patch.object(gate_module, "execute", execute), \
                mock.patch.object(gate_module, "cleanup", lambda gate: []):
            with redirect_stdout(output):
                status = gate_module.main()
        return status, output.getvalue()

    def test_a_crash_outside_a_kind_still_prints_kinds_passed(self):
        def boom(gate):
            raise gate_module.Failure("cli sync --board 7049 failed: API status 429")

        status, printed = self.run_main(boom)
        self.assertEqual(status, 1)
        self.assertIn("LIVE_READBACK_CRASH Failure: cli sync --board 7049 failed", printed)
        self.assertIn(f"KINDS_PASSED 0/{len(gate_module.STEP_NAMES)}", printed)
        self.assertNotIn("LIVE_READBACK_OK", printed)

    def test_a_crash_of_an_unexpected_type_is_also_reported(self):
        def boom(gate):
            raise urllib.error.URLError("network down")

        status, printed = self.run_main(boom)
        self.assertEqual(status, 1)
        self.assertIn("LIVE_READBACK_CRASH URLError", printed)
        self.assertIn("KINDS_PASSED", printed)

    def test_a_clean_run_prints_ok(self):
        def pass_everything(gate):
            gate.results = {name: None for name in gate_module.STEP_NAMES}

        status, printed = self.run_main(pass_everything)
        self.assertEqual(status, 0)
        self.assertIn(f"KINDS_PASSED {len(gate_module.STEP_NAMES)}/{len(gate_module.STEP_NAMES)}", printed)
        self.assertIn("LIVE_READBACK_OK", printed)


if __name__ == "__main__":
    unittest.main()
