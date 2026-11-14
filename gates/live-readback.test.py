import ast
import importlib.util
import io
import json
import os
import re
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

# Minimal payloads with a valid serde shape, for tests that only exercise queue behaviour.
BOARD_WRITE = {"kind": "board", "change": {"ToggleTimer": {"task": 1}}}
TITLE_WRITE = {"kind": "task_title", "task_id": 1, "title": "title"}
GATE_SOURCE = Path(__file__).with_name("live-readback.py")
RUST_SRC = Path(__file__).resolve().parents[1] / "crates" / "ht-core" / "src"


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


class GateCase(unittest.TestCase):
    def setUp(self):
        gate_module.waited_seconds = 0.0


def write_with(rows, *steps, payload=None):
    plan(*steps)
    with mock.patch.object(gate_module.time, "sleep") as slept, quiet():
        gate = new_gate(rows)
        outcome = None
        try:
            gate.write(payload or BOARD_WRITE)
        except gate_module.Failure as error:
            outcome = error
        return gate, outcome, slept


class WriteTests(GateCase):
    def test_a_chained_follow_up_row_applied_in_the_same_call_is_accepted(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Applied(2)\nDRAIN Empty\n"})
        new_gate().write(BOARD_WRITE)

    def test_a_single_applied_row_is_accepted(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Empty\n"})
        new_gate().write(TITLE_WRITE)

    def test_a_first_row_that_is_not_applied_fails(self):
        for first in ("Retrying(1)", "Failed(1)", "Waiting(9)", "Empty"):
            plan({"output": f"QUEUED 1\nDRAIN {first}\n"})
            with self.assertRaises(gate_module.Failure, msg=first):
                new_gate().write(TITLE_WRITE)

    def test_a_follow_up_row_that_retries_fails(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Retrying(2)\n"})
        with self.assertRaises(gate_module.Failure) as raised:
            new_gate([("task_patch", "pending", 1, "HTTP 500")]).write(BOARD_WRITE)
        self.assertIn("drain ended", str(raised.exception))

    def test_a_follow_up_row_that_fails_is_a_failure_with_its_own_message(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Failed(2)\nDRAIN Empty\n", "clear": True})
        gate = new_gate([("task_patch", "failed", 1, "HTTP 422")])
        with self.assertRaises(gate_module.Failure):
            gate.write(BOARD_WRITE)
        with self.assertRaises(gate_module.Failure) as raised:
            gate.require_empty_queue()
        self.assertIn("queue row failed", str(raised.exception))

    def test_a_row_left_pending_after_a_clean_drain_fails(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Empty\n"})
        with self.assertRaises(gate_module.Failure) as raised:
            new_gate([("task_patch", "pending", 0, None)]).write(BOARD_WRITE)
        self.assertIn("queue not empty", str(raised.exception))

    def test_output_without_a_drain_line_fails(self):
        plan({"output": "QUEUED 1\n"})
        with self.assertRaises(gate_module.Failure):
            new_gate().write(TITLE_WRITE)


class RateLimitWriteTests(GateCase):
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

    def test_a_row_that_recorded_the_routes_exact_rate_limit_text_is_also_retried(self):
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


class SyncTests(GateCase):
    def test_a_rate_limited_sync_is_retried(self):
        plan(
            {"output": "cli sync --board 7049 failed: HTTP 429\n", "exit": 1},
            {"output": "SYNC_OK\n"},
        )
        gate = new_gate()
        with mock.patch.object(gate_module.time, "sleep") as slept, quiet():
            gate.sync()
        self.assertEqual(len(calls()), 2)
        self.assertEqual(gate.rate_limit_waits, 1)
        self.assertGreaterEqual(slept.call_count, 1)

    def test_a_sync_failure_that_only_mentions_the_rate_limit_is_not_retried(self):
        plan({"output": "rate limit exceeded. Please slow down and try again shortly.\n", "exit": 1})
        gate = new_gate()
        with mock.patch.object(gate_module.time, "sleep") as slept, quiet():
            with self.assertRaises(gate_module.Failure):
                gate.sync()
        self.assertEqual(len(calls()), 1)
        self.assertEqual(slept.call_count, 0)

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


class ApiTests(GateCase):
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


class ReportTests(GateCase):
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

    def test_a_reopen_failure_never_reads_as_all_passed(self):
        gate = new_gate()
        gate.results = {name: None for name in gate_module.STEP_NAMES}
        gate.results["reopen:after_resync:create_label"] = "Board label and task label"
        output = io.StringIO()
        with redirect_stdout(output):
            self.assertEqual(gate_module.report(gate), 1)
        printed = output.getvalue()
        total = len(gate_module.STEP_NAMES)
        self.assertIn(f"KINDS_PASSED {total - 1}/{total}", printed)
        self.assertNotIn(f"KINDS_PASSED {total}/{total}", printed)
        self.assertIn("REOPEN_FAILED 1", printed)

    def test_every_reopen_check_name_maps_to_a_kind_that_loses_its_pass(self):
        owners = {"create_label": "labels", "remove_subtask": "parent_and_remove_subtask", "inbox_archive": "inbox_remind_and_archive", "task_title": "task_title", "columns": "columns"}
        total = len(gate_module.STEP_NAMES)
        for check, owner in owners.items():
            self.assertIn(owner, gate_module.STEP_NAMES)
            gate = new_gate()
            gate.results = {name: None for name in gate_module.STEP_NAMES}
            gate.results[f"reopen:fresh_database:{check}"] = "lost"
            output = io.StringIO()
            with redirect_stdout(output):
                self.assertEqual(gate_module.report(gate), 1, check)
            self.assertIn(f"KINDS_PASSED {total - 1}/{total}", output.getvalue(), check)

    def test_two_reopen_failures_of_one_kind_lower_the_count_once(self):
        gate = new_gate()
        gate.results = {name: None for name in gate_module.STEP_NAMES}
        gate.results["reopen:same_database:task_title"] = "lost"
        gate.results["reopen:after_resync:task_title"] = "lost"
        output = io.StringIO()
        with redirect_stdout(output):
            gate_module.report(gate)
        total = len(gate_module.STEP_NAMES)
        self.assertIn(f"KINDS_PASSED {total - 1}/{total}", output.getvalue())
        self.assertIn("REOPEN_FAILED 2", output.getvalue())

    def test_the_kinds_passed_line_comes_after_every_other_report_line(self):
        gate = new_gate()
        gate.results = {name: None for name in gate_module.STEP_NAMES}
        gate.results["reopen:after_resync:create_label"] = "lost"
        gate.rate_limit_waits = 2
        output = io.StringIO()
        with redirect_stdout(output):
            gate_module.report(gate)
        self.assertTrue(output.getvalue().strip().splitlines()[-1].startswith("KINDS_PASSED"))

    def test_the_step_list_has_no_duplicates(self):
        self.assertEqual(len(set(gate_module.STEP_NAMES)), len(gate_module.STEP_NAMES))


class CrashTests(GateCase):
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

    def test_a_system_exit_outside_a_kind_still_prints_kinds_passed(self):
        def leave(gate):
            raise SystemExit("sync failed")

        status, printed = self.run_main(leave)
        self.assertEqual(status, 1)
        self.assertIn("LIVE_READBACK_CRASH SystemExit: sync failed", printed)
        self.assertIn(f"KINDS_PASSED 0/{len(gate_module.STEP_NAMES)}", printed)
        self.assertNotIn("LIVE_READBACK_OK", printed)

    def test_an_interrupt_still_prints_kinds_passed(self):
        def interrupted(gate):
            raise KeyboardInterrupt()

        status, printed = self.run_main(interrupted)
        self.assertEqual(status, 1)
        self.assertIn("LIVE_READBACK_CRASH KeyboardInterrupt", printed)
        self.assertIn("KINDS_PASSED", printed)

    def test_a_system_exit_in_cleanup_is_reported_and_the_run_still_ends_with_kinds_passed(self):
        def pass_everything(gate):
            gate.results = {name: None for name in gate_module.STEP_NAMES}

        def leave(gate):
            raise SystemExit("cleanup died")

        output = io.StringIO()
        with mock.patch.object(gate_module, "TOKEN", "token"), \
                mock.patch.object(gate_module, "Gate", CrashTests.Gate), \
                mock.patch.object(gate_module, "execute", pass_everything), \
                mock.patch.object(gate_module, "cleanup", leave):
            with redirect_stdout(output):
                status = gate_module.main()
        self.assertEqual(status, 1)
        self.assertIn("CLEANUP_FAILED cleanup crashed: SystemExit: cleanup died", output.getvalue())
        self.assertIn("KINDS_PASSED", output.getvalue())
        self.assertNotIn("LIVE_READBACK_OK", output.getvalue())

    def test_a_clean_run_prints_ok(self):
        def pass_everything(gate):
            gate.results = {name: None for name in gate_module.STEP_NAMES}

        status, printed = self.run_main(pass_everything)
        self.assertEqual(status, 0)
        self.assertIn(f"KINDS_PASSED {len(gate_module.STEP_NAMES)}/{len(gate_module.STEP_NAMES)}", printed)
        self.assertIn("LIVE_READBACK_OK", printed)


class RunTests(GateCase):
    def test_a_failed_kind_clears_the_queue_for_good(self):
        gate = new_gate([("task_patch", "pending", 0, None), ("board_write", "failed", 3, "HTTP 422")])

        def failing():
            raise gate_module.Failure("boom")

        with quiet():
            gate.run("labels", failing)
        self.assertIsNotNone(gate.results["labels"])
        self.assertEqual(queue_rows(gate.db), [])

    def test_a_passing_kind_leaves_the_queue_alone(self):
        gate = new_gate([("task_patch", "pending", 0, None)])
        with quiet():
            gate.run("task_title", lambda: None)
        self.assertIsNone(gate.results["task_title"])
        self.assertEqual(queue_rows(gate.db), [("task_patch",)])


class RateLimitTextTests(GateCase):
    def test_only_a_real_429_status_text_is_rate_limited(self):
        self.assertTrue(gate_module.rate_limited("cli sync --board 7049 failed: HTTP 429"))
        self.assertFalse(gate_module.rate_limited("Rate limit exceeded. Please slow down and try again shortly."))
        self.assertFalse(gate_module.rate_limited("HTTP 500"))
        self.assertFalse(gate_module.rate_limited(None))

    def test_a_queue_row_counts_as_rate_limited_only_for_the_exact_recorded_texts(self):
        exact = [("board_write", "pending", 1, "HTTP 429")]
        self.assertTrue(new_gate(exact).rate_limit_pending())
        route_text = [("compose_task", "pending", 1, "Rate limit exceeded. Please slow down and try again shortly.")]
        self.assertTrue(new_gate(route_text).rate_limit_pending())
        for text in (
            "Timer outcome unavailable; refresh before retrying (HTTP 429)",
            "HTTP 500: rate limit exceeded",
            "Offline",
            None,
        ):
            self.assertFalse(new_gate([("board_write", "pending", 1, text)]).rate_limit_pending(), text)
        self.assertFalse(new_gate(exact + [("board_write", "pending", 1, "HTTP 500")]).rate_limit_pending())
        self.assertFalse(new_gate([("board_write", "failed", 1, "HTTP 429")]).rate_limit_pending())
        self.assertFalse(new_gate().rate_limit_pending())

    def test_a_text_that_only_mentions_the_rate_limit_is_not_waited_out(self):
        gate, outcome, slept = write_with(
            [("board_write", "pending", 1, "HTTP 500: Rate limit exceeded")],
            {"output": "QUEUED 1\nDRAIN Retrying(1)\n"},
            {"output": "DRAIN Applied(1)\nDRAIN Empty\n", "clear": True},
        )
        self.assertIsNotNone(outcome)
        self.assertEqual(len(calls()), 1)
        self.assertEqual(slept.call_count, 0)


class WaitBudgetTests(GateCase):
    def test_every_wait_spends_the_one_global_budget(self):
        with mock.patch.object(gate_module, "window_wait", return_value=62.0), mock.patch.object(gate_module.time, "sleep") as slept, quiet():
            gate_module.pause("a")
            gate_module.pause("b")
        self.assertEqual(gate_module.waited_seconds, 124.0)
        self.assertEqual(slept.call_count, 2)

    def test_a_wait_past_the_budget_fails_without_sleeping(self):
        gate_module.waited_seconds = gate_module.RATE_LIMIT_WAIT_BUDGET_SECONDS - 10
        with mock.patch.object(gate_module, "window_wait", return_value=62.0), mock.patch.object(gate_module.time, "sleep") as slept, quiet():
            with self.assertRaises(gate_module.Failure) as raised:
                gate_module.pause("drain")
        self.assertIn("wait budget", str(raised.exception))
        self.assertEqual(slept.call_count, 0)

    def test_a_write_stops_waiting_when_the_budget_is_spent(self):
        gate_module.waited_seconds = gate_module.RATE_LIMIT_WAIT_BUDGET_SECONDS - 1
        gate, outcome, slept = write_with(
            [("task_comment", "pending", 1, "HTTP 429")],
            {"output": "QUEUED 1\nDRAIN Retrying(1)\n"},
            {"output": "DRAIN Applied(1)\nDRAIN Empty\n", "clear": True},
        )
        self.assertIn("wait budget", str(outcome))
        self.assertEqual(slept.call_count, 0)

    def test_the_budget_is_shared_between_api_calls_and_cli_reads(self):
        throttled = urllib.error.HTTPError("https://x", 429, "Too Many Requests", {}, io.BytesIO(b"{}"))
        with mock.patch.object(gate_module, "window_wait", return_value=62.0), mock.patch.object(gate_module.time, "sleep"), quiet():
            with mock.patch.object(gate_module.urllib.request, "urlopen", side_effect=[throttled, FakeResponse(b"{}")]):
                gate_module.api("GET", "/time/running")
            plan({"output": "HTTP 429\n", "exit": 1}, {"output": "SYNC_OK\n"})
            new_gate().sync()
        self.assertEqual(gate_module.waited_seconds, 124.0)


class FetchTaskTests(GateCase):
    def test_fetch_task_runs_the_cli_task_command(self):
        plan({"output": "TASK_OK 5\n"})
        new_gate().fetch_task(5)
        self.assertEqual(calls(), [["task", "--task", "5"]])

    def test_fetch_task_waits_out_a_rate_limit_and_retries(self):
        plan({"output": "HTTP 429\n", "exit": 1}, {"output": "TASK_OK 5\n"})
        gate = new_gate()
        with mock.patch.object(gate_module.time, "sleep"), quiet():
            gate.fetch_task(5)
        self.assertEqual(len(calls()), 2)
        self.assertEqual(gate.rate_limit_waits, 1)

    def test_fetch_task_failure_is_not_hidden(self):
        plan({"output": "task 5 is not cached\n", "exit": 1})
        with self.assertRaises(gate_module.Failure):
            new_gate().fetch_task(5)
        self.assertEqual(len(calls()), 1)


class ApiStatusTests(GateCase):
    def test_a_400_whose_body_mentions_the_rate_limit_is_not_retried(self):
        refused = urllib.error.HTTPError("https://x", 400, "Bad Request", {}, io.BytesIO(b'{"message":"Rate limit exceeded in the handler"}'))
        with mock.patch.object(gate_module.urllib.request, "urlopen", side_effect=[refused, FakeResponse(b"{}")]) as called:
            with mock.patch.object(gate_module.time, "sleep") as slept:
                with self.assertRaises(gate_module.Failure):
                    gate_module.api("POST", "/tasks")
        self.assertEqual(called.call_count, 1)
        self.assertEqual(slept.call_count, 0)

    def test_a_200_refusal_that_mentions_the_rate_limit_is_not_retried(self):
        body = b'{"success":false,"error":"Rate limit exceeded. Please slow down"}'
        with mock.patch.object(gate_module.urllib.request, "urlopen", side_effect=[FakeResponse(body), FakeResponse(b"{}")]) as called:
            with mock.patch.object(gate_module.time, "sleep") as slept:
                with self.assertRaises(gate_module.Failure) as raised:
                    gate_module.api("POST", "/tasks")
        self.assertEqual(called.call_count, 1)
        self.assertEqual(slept.call_count, 0)
        self.assertIn("refused", str(raised.exception))


class World:
    def __init__(self, task=7, parent=8, other=19):
        self.task = task
        self.parent = parent
        self.other = other
        self.server_parent = None
        self.server_children = []
        self.local_parent = None
        self.local_children = []
        self.fetch_refreshes_parent = True
        self.parent_none_works = True
        self.remove_subtask_works = True
        self.local_clears_after_remove = True
        self.writes = []
        self.fetched = []

    def server_task(self, task_id):
        if task_id == self.task:
            return {"id": task_id, "parentTaskId": self.server_parent}
        return {"id": task_id, "sub_tasks": [{"id": child} for child in self.server_children]}

    def local_task(self, _db, _table, task_id):
        if task_id == self.task:
            return {"id": task_id, "parent_id": self.local_parent}
        return {"id": task_id, "sub_tasks": [{"id": child} for child in self.local_children]}

    def write(self, payload):
        self.writes.append(payload)
        if payload["kind"] == "task_patch":
            parent = payload["patch"]["Parent"]
            if parent is not None:
                self.server_parent = parent
                self.local_parent = parent
                if self.task not in self.server_children:
                    self.server_children.append(self.task)
            elif self.parent_none_works:
                self.server_parent = None
                self.local_parent = None
                self.server_children = []
            return
        if self.task not in self.local_children:
            raise gate_module.Failure("task is not a child of this parent")
        if self.remove_subtask_works:
            self.server_parent = None
            self.server_children = []
        if self.local_clears_after_remove:
            self.local_parent = None
            self.local_children = []

    def fetch_task(self, task_id):
        self.fetched.append(task_id)
        if self.fetch_refreshes_parent:
            self.local_children = list(self.server_children)


class FakeParentGate:
    def __init__(self, world):
        self.db = "db"
        self.world = world
        self.reopen = []

    def write(self, payload):
        self.world.write(payload)

    def fetch_task(self, task_id):
        self.world.fetch_task(task_id)

    def expect_local(self, name, description, check):
        self.reopen.append((name, description, check))


class ParentKindTests(GateCase):
    def run_kind(self, world):
        gate = FakeParentGate(world)
        with mock.patch.object(gate_module, "get_task", world.server_task), mock.patch.object(gate_module, "local", world.local_task):
            gate_module.parent_kind(gate, world.task, world.parent)
        return gate

    def test_both_writes_are_sent_and_the_child_list_is_fetched_before_remove_subtask(self):
        world = World()
        gate = self.run_kind(world)
        shapes = [
            ("task_patch", write["patch"]["Parent"]) if write["kind"] == "task_patch" else ("remove_subtask", write["change"]["RemoveSubtask"]["parent"])
            for write in world.writes
        ]
        self.assertEqual(shapes, [("task_patch", 8), ("task_patch", None), ("task_patch", 8), ("remove_subtask", 8)])
        self.assertEqual(world.fetched, [8])
        self.assertEqual([item[0] for item in gate.reopen], ["remove_subtask"])

    def test_remove_subtask_on_a_stale_parent_cache_is_reported_not_hidden(self):
        world = World()
        world.fetch_refreshes_parent = False
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("cached sub-tasks lack the child after a fresh task fetch", str(raised.exception))
        self.assertEqual(len(world.writes), 3)

    def test_a_remove_subtask_the_server_never_applied_fails(self):
        world = World()
        world.remove_subtask_works = False
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("API parent not removed by RemoveSubtask", str(raised.exception))

    def test_a_remove_subtask_that_leaves_the_local_cache_stale_fails(self):
        world = World()
        world.local_clears_after_remove = False
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("local task still has its parent after RemoveSubtask", str(raised.exception))

    def test_a_parent_none_the_server_never_applied_fails_before_remove_subtask_runs(self):
        world = World()
        world.parent_none_works = False
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("Parent(None)", str(raised.exception))
        self.assertEqual(len(world.writes), 2)

    def test_the_reopen_check_needs_the_parent_cleared_on_the_child_and_in_the_parents_list(self):
        world = World()
        gate = self.run_kind(world)
        check = gate.reopen[0][2]
        with mock.patch.object(gate_module, "local", world.local_task):
            self.assertTrue(check("db"))
            world.local_parent = 8
            self.assertFalse(check("db"))
            world.local_parent = None
            world.local_children = [world.task]
            self.assertFalse(check("db"))

    def test_the_parent_is_read_from_either_alias(self):
        self.assertEqual(gate_module.parent_of({"parentTaskId": 4}), 4)
        self.assertEqual(gate_module.parent_of({"parent_id": 5}), 5)
        self.assertIsNone(gate_module.parent_of({}))
        self.assertIsNone(gate_module.parent_of(None))


class TimerRunningOnTests(GateCase):
    def test_only_the_timer_task_id_counts(self):
        self.assertTrue(gate_module.timer_running_on({"timers": [{"id": 1, "task": {"id": 9}}]}, 9))
        self.assertFalse(gate_module.timer_running_on({"timers": [{"id": 9, "task": {"id": 19}}]}, 9))
        self.assertFalse(gate_module.timer_running_on({"timers": [{"id": 1, "note": "task 9", "task": {"id": 91}}]}, 9))
        self.assertFalse(gate_module.timer_running_on({"timers": []}, 9))
        self.assertFalse(gate_module.timer_running_on({"timers": [{"id": 9}]}, 9))
        self.assertFalse(gate_module.timer_running_on({}, 9))
        self.assertFalse(gate_module.timer_running_on([], 9))


class FakeTimerGate:
    def __init__(self, world):
        self.world = world
        self.timer_task = None
        self.seen_timer_task = []

    def write(self, payload):
        self.seen_timer_task.append(self.timer_task)
        self.world.toggle()


class TimerWorld:
    def __init__(self, task=9, enabled=True):
        self.task = task
        self.enabled = enabled
        self.running = False
        self.toggles_that_do_nothing = 0
        self.other_timer = None
        self.requests = []

    def toggle(self):
        if self.toggles_that_do_nothing:
            self.toggles_that_do_nothing -= 1
            return
        self.running = not self.running

    def api(self, method, path, body=None, base=None):
        self.requests.append((method, path))
        if path == "/time/status":
            return {"success": True, "summary": {"enabled": self.enabled}}
        timers = []
        if self.running:
            timers.append({"id": 1, "task": {"id": self.task}})
        if self.other_timer is not None:
            timers.append({"id": self.task, "task": {"id": self.other_timer}})
        return {"success": True, "timers": timers}


class TimerKindTests(GateCase):
    def run_kind(self, world):
        gate = FakeTimerGate(world)
        with mock.patch.object(gate_module, "api", world.api):
            try:
                gate_module.timer_kind(gate, world.task)
            finally:
                self.gate = gate
        return gate

    def test_a_clean_toggle_pair_passes_and_clears_the_cleanup_marker(self):
        gate = self.run_kind(TimerWorld())
        self.assertIsNone(gate.timer_task)

    def test_the_cleanup_marker_is_set_before_the_first_write(self):
        gate = self.run_kind(TimerWorld())
        self.assertEqual(gate.seen_timer_task, [9, 9])

    def test_time_tracking_disabled_fails_before_any_write(self):
        world = TimerWorld(enabled=False)
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("time tracking disabled", str(raised.exception))
        self.assertEqual(self.gate.seen_timer_task, [])

    def test_a_first_toggle_that_starts_nothing_fails_and_keeps_the_cleanup_marker(self):
        world = TimerWorld()
        world.toggles_that_do_nothing = 1
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("API timer not running", str(raised.exception))
        self.assertEqual(self.gate.timer_task, 9)

    def test_a_second_toggle_that_leaves_the_timer_running_fails_and_keeps_the_cleanup_marker(self):
        world = TimerWorld()

        def sticky(self=world):
            if self.running and self.requests.count(("GET", "/time/running")) >= 1:
                return
            self.running = not self.running

        world.toggle = sticky
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("API timer still running", str(raised.exception))
        self.assertEqual(self.gate.timer_task, 9)

    def test_another_task_whose_timer_row_id_equals_this_task_does_not_count_as_running(self):
        world = TimerWorld()
        world.other_timer = 40
        world.toggles_that_do_nothing = 1
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("API timer not running", str(raised.exception))


class LabelWorld:
    def __init__(self):
        self.labels = ["old"]
        self.local_labels = ["old"]
        self.clear_works = True
        self.local_clears = True
        self.labels_return_after_sync = False

    def server_task(self, _id):
        return {"id": 3, "labels": self.labels}

    def local_task(self, _db, _table, _id):
        return {"id": 3, "labels": self.local_labels}


class FakeLabelGate:
    def __init__(self, world):
        self.db = "db"
        self.world = world
        self.synced = 0

    def write(self, payload):
        self.world.labels = [] if self.world.clear_works else ["old"]
        self.world.local_labels = [] if self.world.local_clears else ["old"]

    def sync(self):
        self.synced += 1
        if self.world.labels_return_after_sync:
            self.world.local_labels = ["old"]


class LabelsClearTests(GateCase):
    def run_kind(self, world):
        gate = FakeLabelGate(world)
        with mock.patch.object(gate_module, "get_task", world.server_task), mock.patch.object(gate_module, "local", world.local_task):
            gate_module.labels_clear_kind(gate, 3)
        return gate

    def test_cleared_labels_pass_and_a_resync_runs(self):
        gate = self.run_kind(LabelWorld())
        self.assertEqual(gate.synced, 1)

    def test_the_server_keeping_a_label_fails(self):
        world = LabelWorld()
        world.clear_works = False
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("API labels not cleared", str(raised.exception))

    def test_the_local_task_keeping_a_label_fails(self):
        world = LabelWorld()
        world.local_clears = False
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("local task still carries the labels", str(raised.exception))

    def test_a_label_that_comes_back_after_a_resync_fails(self):
        world = LabelWorld()
        world.labels_return_after_sync = True
        with self.assertRaises(gate_module.Failure) as raised:
            self.run_kind(world)
        self.assertIn("labels came back after a resync", str(raised.exception))


class LabelReopenCheckTests(GateCase):
    def check(self, board_rows, task_labels, name="gate-label-1"):
        rows = [(json.dumps(row),) for row in board_rows]
        task = {"id": 3, "labels": task_labels}
        with mock.patch.object(gate_module, "query", return_value=rows), mock.patch.object(gate_module, "local", return_value=task):
            return gate_module.label_reopen_check(name, 3)("db")

    def test_both_the_board_row_and_the_task_label_must_survive(self):
        self.assertTrue(self.check([{"name": "gate-label-1"}], [{"name": "gate-label-1"}]))

    def test_a_missing_board_label_row_fails(self):
        self.assertFalse(self.check([{"name": "someone-else"}], [{"name": "gate-label-1"}]))
        self.assertFalse(self.check([], [{"name": "gate-label-1"}]))

    def test_a_missing_task_label_fails(self):
        self.assertFalse(self.check([{"name": "gate-label-1"}], []))
        self.assertFalse(self.check([{"name": "gate-label-1"}], None))


class InboxWorld:
    """The live inbox as the gate sees it: one row, one Reminder, one card-action route."""

    def __init__(self, notification_id=41, task=9, project=7049):
        self.notification_id = notification_id
        self.task = task
        self.project = project
        self.row = {"id": notification_id, "taskId": task, "status": "Normal", "type": "TaskMovedToInbox"}
        self.reminder = False
        self.connected_agent = None
        self.remind_hides = True
        self.archive_hides = True
        self.cancel_clears_reminder = True
        self.unarchive_limit = None
        self.unarchives = 0
        self.unarchive_lies_after = None
        self.local_follows_sync = True
        self.local_status = "Normal"
        self.writes = []
        self.requests = []

    def listing(self):
        # hypertask getAll.ts:138-143 hides the row while its task holds a Normal Reminder.
        if self.reminder or self.row["status"] != "Normal":
            return []
        return [dict(self.row)]

    def api(self, method, path, body=None):
        self.requests.append((method, path, body))
        if path == "/user/context":
            return {"success": True, "user": {"id": 5}, "connected_agent": self.connected_agent}
        if path == "/inbox/list":
            return {"success": True, "user_notifications": self.listing()}
        if path == "/inbox/unarchive":
            self.unarchives += 1
            if self.unarchive_limit is not None and self.unarchives > self.unarchive_limit:
                return {"success": True, "unarchived_count": 0}
            if (
                self.unarchive_lies_after is not None
                and self.unarchives > self.unarchive_lies_after
                and self.row["status"] == "Archive"
            ):
                return {"success": True, "unarchived_count": 1}
            count = 0
            if self.row["status"] == "Archive":
                self.row["status"] = "Normal"
                count = 1
            return {"success": True, "unarchived_count": count}
        if path == "/tasks/card-actions":
            # hypertask inboxState.ts:94-111: archive this task's TaskMovedToInbox rows, cancel its Reminders.
            if self.cancel_clears_reminder:
                self.reminder = False
            if self.row["type"] == "TaskMovedToInbox" and self.row["status"] == "Normal":
                self.row["status"] = "Archive"
            return {"success": True, "action": "inbox", "active": False}
        raise AssertionError(f"unexpected request {method} {path}")

    def write(self, payload):
        self.writes.append(payload)
        if payload["kind"] == "inbox_remind":
            if self.remind_hides:
                self.reminder = True
                self.row["status"] = "Archive"
        elif payload["kind"] == "inbox_archive":
            if self.archive_hides:
                self.row["status"] = "Archive"
        self.local_status = "Archive"

    def sync(self):
        self.local_status = self.row["status"] if self.local_follows_sync else "Normal"

    def local(self, _db, _table, notification_id):
        if notification_id != self.notification_id:
            return None
        return {"id": self.notification_id, "status": self.local_status}


class FakeInboxGate:
    def __init__(self, world):
        self.db = "db"
        self.world = world
        self.reopen = []
        self.notification_archived = None
        self.inbox_reminder = None
        self.synced = 0

    def write(self, payload):
        self.world.write(payload)

    def sync(self):
        self.synced += 1
        self.world.sync()

    def expect_local(self, name, description, check):
        self.reopen.append((name, description, check))


class InboxRemindKindTests(GateCase):
    def run_kind(self, world):
        gate = FakeInboxGate(world)
        with mock.patch.object(gate_module, "api", world.api), mock.patch.object(gate_module, "local", world.local), quiet():
            gate_module.inbox_remind_kind(gate, world.task, world.project, world.notification_id)
        return gate

    def run_kind_with_failure(self, world):
        self.gate = FakeInboxGate(world)
        with mock.patch.object(gate_module, "api", world.api), mock.patch.object(gate_module, "local", world.local), quiet():
            with self.assertRaises(gate_module.Failure) as raised:
                gate_module.inbox_remind_kind(self.gate, world.task, world.project, world.notification_id)
        return str(raised.exception)

    def test_a_clean_run_reminds_archives_cancels_the_reminder_and_restores_the_row(self):
        world = InboxWorld()
        gate = self.run_kind(world)
        self.assertEqual([write["kind"] for write in world.writes], ["inbox_remind", "inbox_archive"])
        self.assertEqual([write.get("notification_ids") or [write.get("notification_id")] for write in world.writes], [[41], [41]])
        self.assertIn(("POST", "/tasks/card-actions", {"task_id": 9, "project_id": 7049, "action": "inbox", "active": False}), world.requests)
        self.assertEqual(world.row["status"], "Normal")
        self.assertFalse(world.reminder)
        self.assertEqual(gate.synced, 2, "the cache must be re-read after the archive and after the restore")
        self.assertIsNone(gate.notification_archived)
        self.assertIsNone(gate.inbox_reminder)

    def test_the_kind_only_ever_writes_the_run_row_it_was_given(self):
        world = InboxWorld()
        self.run_kind(world)
        for write in world.writes:
            ids = write.get("notification_ids") or [write.get("notification_id")]
            self.assertEqual(set(ids), {world.notification_id})

    def test_the_row_is_proven_hidden_by_the_reminder_before_the_cancel_route_runs(self):
        world = InboxWorld()
        self.run_kind(world)
        paths = [(method, path) for method, path, _ in world.requests]
        cancel = paths.index(("POST", "/tasks/card-actions"))
        self.assertEqual(paths[cancel - 1], ("GET", "/inbox/list"))
        self.assertEqual(paths[cancel - 2], ("POST", "/inbox/unarchive"))
        self.assertEqual(paths[cancel - 3], ("GET", "/inbox/list"))
        self.assertLess(cancel, paths.index(("GET", "/inbox/list"), cancel))

    def test_a_cancel_route_that_leaves_the_reminder_fails_and_keeps_the_cleanup_marker(self):
        world = InboxWorld()
        world.cancel_clears_reminder = False
        message = self.run_kind_with_failure(world)
        self.assertIn("is not back in the inbox after its Reminder was cancelled", message)
        self.assertEqual(self.gate.inbox_reminder, {"task_id": 9, "project_id": 7049})
        self.assertEqual([write["kind"] for write in world.writes], ["inbox_remind"])

    def test_an_agent_token_is_refused_before_any_write(self):
        world = InboxWorld()
        world.connected_agent = {"id": "agent-1", "displayName": "helper"}
        message = self.run_kind_with_failure(world)
        self.assertIn("INBOX_REMIND_NEEDS_HUMAN_TOKEN", message)
        self.assertEqual(world.writes, [])
        self.assertIsNone(self.gate.notification_archived)

    def test_a_remind_the_server_does_not_hide_fails(self):
        world = InboxWorld()
        world.remind_hides = False
        self.assertIn("still lists the reminded row", self.run_kind_with_failure(world))

    def test_an_archive_the_server_does_not_hide_fails(self):
        world = InboxWorld()
        world.archive_hides = False
        self.assertIn("is still active", self.run_kind_with_failure(world))

    def test_a_restore_the_server_refuses_fails(self):
        world = InboxWorld()
        world.unarchive_limit = 2
        self.assertIn("unarchive count 0", self.run_kind_with_failure(world))

    def test_the_unarchive_after_the_reminder_cancel_must_report_the_one_row(self):
        world = InboxWorld()
        world.unarchive_limit = 1
        self.assertIn("unarchive count 0 after the reminder was cancelled", self.run_kind_with_failure(world))

    def test_a_restore_the_server_only_claims_fails(self):
        world = InboxWorld()
        world.unarchive_lies_after = 2
        self.assertIn("did not return to the inbox", self.run_kind_with_failure(world))

    def test_a_local_cache_that_keeps_the_row_after_the_archive_fails(self):
        world = InboxWorld()
        world.local_follows_sync = False
        self.assertIn("after an archive and a resync", self.run_kind_with_failure(world))

    def test_the_reopen_check_wants_the_restored_row_in_the_cache(self):
        world = InboxWorld()
        gate = self.run_kind(world)
        self.assertEqual([item[0] for item in gate.reopen], ["inbox_archive"])
        check = gate.reopen[0][2]
        with mock.patch.object(gate_module, "local", world.local):
            self.assertTrue(check("db"))
            world.local_status = "Archive"
            self.assertFalse(check("db"))

    def test_the_readback_line_names_the_row_and_the_task_the_run_owns(self):
        world = InboxWorld()
        gate = FakeInboxGate(world)
        printed = io.StringIO()
        with mock.patch.object(gate_module, "api", world.api), mock.patch.object(gate_module, "local", world.local), redirect_stdout(printed):
            gate_module.inbox_remind_kind(gate, world.task, world.project, world.notification_id)
        self.assertEqual(re.findall(r"INBOX_READBACK notification (\d+) task (\d+)", printed.getvalue()), [("41", "9")])

    def test_the_gate_no_longer_borrows_a_preexisting_inbox_row(self):
        source = GATE_SOURCE.read_text()
        self.assertNotIn('SELECT id FROM "Notification"', source)
        self.assertIn('inbox_remind_kind(gate, assign_task, BOARD, moved_notification["id"])', source)
        self.assertIn("cancel_inbox_reminder(task, project)", source)


class CleanupTests(GateCase):
    class Gate:
        def __init__(self, timer_task, inbox_reminder=None):
            self.timer_task = timer_task
            self.notification_archived = None
            self.inbox_reminder = inbox_reminder
            self.board_title = None
            self.extra_columns = []
            self.created_tasks = []

    def run_cleanup(self, running, timer_task=9, fail_on=None, inbox_reminder=None, notification_archived=None):
        requests = []

        def fake(method, path, body=None, base=None):
            requests.append((method, path, body))
            if fail_on == path:
                raise gate_module.Failure(f"API {method} {path} status 500")
            return {"success": True, "active": False, "unarchived_count": 1, **running}

        gate = CleanupTests.Gate(timer_task, inbox_reminder)
        gate.notification_archived = notification_archived
        with mock.patch.object(gate_module, "api", fake), quiet():
            problems = gate_module.cleanup(gate)
        return problems, requests

    def test_a_timer_still_running_on_the_task_is_stopped(self):
        problems, requests = self.run_cleanup({"timers": [{"id": 1, "task": {"id": 9}}]})
        self.assertEqual(problems, [])
        self.assertEqual(requests, [("GET", "/time/running", None), ("POST", "/time/stop", {"task": "9"})])

    def test_a_timer_that_is_not_running_is_not_stopped(self):
        problems, requests = self.run_cleanup({"timers": [{"id": 9, "task": {"id": 40}}]})
        self.assertEqual(problems, [])
        self.assertEqual(requests, [("GET", "/time/running", None)])

    def test_a_cleanup_without_a_timer_task_never_touches_timers(self):
        problems, requests = self.run_cleanup({"timers": []}, timer_task=None)
        self.assertEqual((problems, requests), ([], []))

    def test_a_failed_timer_stop_is_reported(self):
        problems, _ = self.run_cleanup({"timers": [{"id": 1, "task": {"id": 9}}]}, fail_on="/time/stop")
        self.assertEqual(len(problems), 1)
        self.assertIn("stop timer", problems[0])

    def test_a_reminder_the_kind_left_behind_is_cancelled_before_the_unarchive(self):
        problems, requests = self.run_cleanup(
            {"success": True},
            timer_task=None,
            inbox_reminder={"task_id": 9, "project_id": 7049},
            notification_archived=41,
        )
        self.assertEqual(problems, [])
        self.assertEqual(
            requests,
            [
                ("POST", "/tasks/card-actions", {"task_id": 9, "project_id": 7049, "action": "inbox", "active": False}),
                ("POST", "/inbox/unarchive", {"notification_ids": [41]}),
            ],
        )

    def test_a_failed_reminder_cancel_is_reported(self):
        problems, _ = self.run_cleanup(
            {"success": True},
            timer_task=None,
            fail_on="/tasks/card-actions",
            inbox_reminder={"task_id": 9, "project_id": 7049},
        )
        self.assertEqual(len(problems), 1)
        self.assertIn("cancel inbox reminder", problems[0])

    def test_a_green_kind_leaves_no_reminder_or_archive_for_cleanup(self):
        problems, requests = self.run_cleanup({"timers": []}, timer_task=None)
        self.assertEqual((problems, requests), ([], []))


def snake_case(variant):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", variant).lower()


def literal_shape(node):
    """A payload literal with every computed value replaced by one sentinel string."""
    if isinstance(node, ast.Dict):
        return {literal_shape(key): literal_shape(value) for key, value in zip(node.keys, node.values)}
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, (ast.List, ast.Tuple)):
        return [literal_shape(item) for item in node.elts]
    return "<computed>"


def gate_write_shapes():
    """Every payload literal passed to gate.write() in the gate source."""
    tree = ast.parse(GATE_SOURCE.read_text())
    assigned = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            assigned.setdefault(node.targets[0].id, node.value)
    shapes = []
    for node in ast.walk(tree):
        if not (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "write"
            and isinstance(node.func.value, ast.Name)
            and node.func.value.id == "gate"
        ):
            continue
        argument = node.args[0]
        if isinstance(argument, ast.Name) and argument.id in assigned:
            argument = assigned[argument.id]
        shapes.append(literal_shape(argument))
    return shapes


def with_sampled_values(shape):
    """Fill AST sentinels with one value of each TaskPatch variant's field type.

    The scan cannot evaluate names or f-strings, so a computed value reaches this table
    as a sentinel. The key/tag shape is still the scanned one; the field types are the
    ones pinned to crates/ht-core/src/task_patch.rs.
    """
    patch = shape.get("patch")
    if shape.get("kind") != "task_patch" or not isinstance(patch, dict):
        return shape
    sampled = dict(shape)
    sampled["patch"] = {}
    for variant, value in patch.items():
        allowed = gate_module.TASK_PATCH_SHAPES[variant]
        if value == "<computed>":
            allowed = allowed if isinstance(allowed, tuple) else (allowed,)
            kind = next((item for item in allowed if item is not type(None)), type(None))
            value = {int: 1, str: "<computed>", list: []}[kind]
        sampled["patch"][variant] = value
    return sampled


def rust_enum(source, name):
    """The serde attributes and top-level variant names of a Rust enum."""
    text = source.read_text()
    match = re.search(r"pub enum %s\s*\{" % name, text)
    if match is None:
        raise AssertionError(f"{name} is not declared in {source}")
    attributes = []
    for line in reversed(text[: match.start()].splitlines()):
        if line.strip().startswith("#["):
            attributes.append(line.strip())
        elif line.strip() == "":
            continue
        else:
            break
    depth = 1
    body = []
    for char in text[match.end():]:
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                break
        body.append(char)
    segments, current, depth = [], "", 0
    for char in "".join(body):
        if char in "{([<":
            depth += 1
        elif char in "})]>":
            depth -= 1
        if char == "," and depth == 0:
            segments.append(current)
            current = ""
        else:
            current += char
    segments.append(current)
    variants = []
    for segment in segments:
        segment = "\n".join(line for line in segment.splitlines() if not line.strip().startswith("//"))
        found = re.match(r"\s*([A-Z][A-Za-z0-9_]*)", segment)
        if found:
            variants.append(found.group(1))
    return attributes, variants


class WriteShapeValidationTests(GateCase):
    def test_every_write_the_gate_sends_matches_a_serde_shape(self):
        shapes = gate_write_shapes()
        self.assertGreaterEqual(len(shapes), 30)
        for shape in shapes:
            with self.subTest(write=shape):
                gate_module.validate_write(with_sampled_values(shape))

    def test_board_and_task_patch_shapes_are_checked(self):
        for payload in [
            {"kind": "board"},
            {"kind": "board", "change": {"MoveTask": {"task": 1, "project": 2}}},
            {"kind": "board", "change": {"ToggleTimer": {"task": 1}, "MoveTask": {"task": 1, "project": 2, "section": 3}}},
            {"kind": "board", "change": {"ToggleTimer": {"task": 1}, "comment": 4}},
            {"kind": "task_patch", "task_id": 1, "patch": {"Priority": "high"}},
            {"kind": "task_patch", "task_id": 1, "patch": {"Ranking": 1}},
            {"kind": "task_patch", "task_id": 1, "patch": {"Priority": 2, "Estimate": 3}},
        ]:
            with self.subTest(payload=payload):
                with self.assertRaises(gate_module.Failure):
                    gate_module.validate_write(payload)

    def test_unknown_kinds_missing_fields_and_unknown_fields_are_checked(self):
        for payload in [
            {"kind": "nope"},
            {"kind": "task_title", "task_id": 1},
            {"kind": "task_title", "task_id": 1, "title": "t", "extra": True},
            {"kind": "discard_draft", "task_id": 1, "draft_type": "commentary"},
        ]:
            with self.subTest(payload=payload):
                with self.assertRaises(gate_module.Failure):
                    gate_module.validate_write(payload)

    def test_a_valid_comment_draft_without_text_is_accepted(self):
        gate_module.validate_write({"kind": "comment_draft", "task_id": 1})

    def test_a_refused_shape_never_reaches_the_cli(self):
        plan({"output": "QUEUED 1\nDRAIN Applied(1)\nDRAIN Empty\n"})
        with self.assertRaises(gate_module.Failure):
            new_gate().write({"kind": "comment_action", "task_id": 1, "comment_id": 2, "change": {"Edit": {"text": "x"}}})
        self.assertEqual(calls(), [])


class CommentActionShapeTests(GateCase):
    def test_a_comment_action_is_tagged_with_its_action(self):
        for change in [
            {"action": "edit", "text": "<p>x</p>"},
            {"action": "reaction", "emoji": "\U0001F44D", "active": True},
            {"action": "saved", "public": False},
            {"action": "delete"},
        ]:
            with self.subTest(change=change):
                gate_module.validate_write({"kind": "comment_action", "task_id": 1, "comment_id": 2, "change": change})

    def test_the_externally_tagged_shapes_that_failed_live_are_refused(self):
        for change in [
            {"Edit": {"text": "<p>x</p>"}},
            {"Reaction": {"emoji": "\U0001F44D", "active": True}},
            {"Saved": {"public": False}},
            "Delete",
        ]:
            with self.subTest(change=change):
                with self.assertRaises(gate_module.Failure):
                    gate_module.validate_write({"kind": "comment_action", "task_id": 1, "comment_id": 2, "change": change})

    def test_a_comment_action_needs_its_field_and_no_extra_field(self):
        for change in [{"action": "edit"}, {"action": "edit", "text": "x", "extra": 1}, {"action": "Delete"}]:
            with self.subTest(change=change):
                with self.assertRaises(gate_module.Failure):
                    gate_module.validate_write({"kind": "comment_action", "task_id": 1, "comment_id": 2, "change": change})


class RustWriteShapeTests(unittest.TestCase):
    def test_the_write_table_matches_the_rust_enum_and_its_kind_tag(self):
        attributes, variants = rust_enum(RUST_SRC / "queue.rs", "Write")
        self.assertTrue(any('tag = "kind"' in line for line in attributes), attributes)
        self.assertTrue(any('rename_all = "snake_case"' in line for line in attributes), attributes)
        self.assertEqual({snake_case(variant) for variant in variants}, set(gate_module.WRITE_SHAPES))

    def test_the_comment_table_matches_the_rust_enum_and_its_action_tag(self):
        attributes, variants = rust_enum(RUST_SRC / "comment_write.rs", "CommentWrite")
        self.assertTrue(any('tag = "action"' in line for line in attributes), attributes)
        self.assertTrue(any('rename_all = "snake_case"' in line for line in attributes), attributes)
        self.assertEqual({snake_case(variant) for variant in variants}, set(gate_module.COMMENT_SHAPES))

    def test_board_and_patch_tables_match_their_untagged_external_enums(self):
        for source, name, table in [
            ("board_write.rs", "BoardWrite", gate_module.BOARD_SHAPES),
            ("task_patch.rs", "TaskPatch", gate_module.TASK_PATCH_SHAPES),
        ]:
            with self.subTest(enum=name):
                attributes, variants = rust_enum(RUST_SRC / source, name)
                self.assertFalse(any("tag =" in line for line in attributes), f"{name} must stay externally tagged")
                self.assertEqual(set(variants), set(table))

    def test_the_nested_write_fields_point_at_the_types_these_tables_cover(self):
        text = (RUST_SRC / "queue.rs").read_text()
        for field in ("change: crate::CommentWrite", "change: crate::BoardWrite", "patch: crate::TaskPatch"):
            self.assertIn(field, text)


class PriorityReopenTests(GateCase):
    def test_both_cached_forms_name_the_same_served_priority(self):
        self.assertEqual(gate_module.priority_index({"priority_index": 2, "Priority_Value": "High"}), 2)
        self.assertEqual(gate_module.priority_index("High"), 2)

    def test_a_different_missing_or_misspelled_priority_does_not_pass_the_check(self):
        for value in [
            {"priority_index": 1, "Priority_Value": "Urgent"},
            "Urgent",
            "No Priority",
            "high",
            "",
            {},
            None,
            ["High"],
        ]:
            with self.subTest(value=value):
                self.assertNotEqual(gate_module.priority_index(value), 2)


if __name__ == "__main__":
    unittest.main()
