import importlib.util
import os
import sqlite3
import stat
import tempfile
import unittest
from pathlib import Path

WORK = tempfile.mkdtemp()
FAKE_CLI = os.path.join(WORK, "fake-cli")
FAKE_OUTPUT = os.path.join(WORK, "fake-output")
Path(FAKE_CLI).write_text('#!/bin/sh\ncat "$FAKE_OUTPUT"\n')
os.chmod(FAKE_CLI, os.stat(FAKE_CLI).st_mode | stat.S_IXUSR)
os.environ["HT_CORE_CLI"] = FAKE_CLI
os.environ["FAKE_OUTPUT"] = FAKE_OUTPUT

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
    return gate


def queue_rows(db):
    connection = sqlite3.connect(db)
    try:
        return connection.execute("SELECT kind FROM mutation_queue").fetchall()
    finally:
        connection.close()


def cli_prints(text):
    Path(FAKE_OUTPUT).write_text(text)


class WriteTests(unittest.TestCase):
    def test_a_chained_follow_up_row_applied_in_the_same_call_is_accepted(self):
        cli_prints("QUEUED 1\nDRAIN Applied(1)\nDRAIN Applied(2)\nDRAIN Empty\n")
        new_gate().write({"kind": "board"})

    def test_a_single_applied_row_is_accepted(self):
        cli_prints("QUEUED 1\nDRAIN Applied(1)\nDRAIN Empty\n")
        new_gate().write({"kind": "task_title"})

    def test_a_first_row_that_is_not_applied_fails(self):
        for first in ("Retrying(1)", "Failed(1)", "Waiting(9)", "Empty"):
            cli_prints(f"QUEUED 1\nDRAIN {first}\n")
            with self.assertRaises(gate_module.Failure, msg=first):
                new_gate().write({"kind": "task_title"})

    def test_a_follow_up_row_that_retries_fails(self):
        cli_prints("QUEUED 1\nDRAIN Applied(1)\nDRAIN Retrying(2)\n")
        with self.assertRaises(gate_module.Failure) as raised:
            new_gate([("task_patch", "pending", 1, "HTTP 500")]).write({"kind": "board"})
        self.assertIn("drain ended", str(raised.exception))

    def test_a_follow_up_row_that_fails_is_a_failure_with_its_own_message(self):
        cli_prints("QUEUED 1\nDRAIN Applied(1)\nDRAIN Failed(2)\nDRAIN Empty\n")
        gate = new_gate([("task_patch", "failed", 1, "HTTP 422")])
        with self.assertRaises(gate_module.Failure):
            gate.write({"kind": "board"})
        with self.assertRaises(gate_module.Failure) as raised:
            gate.require_empty_queue()
        self.assertIn("queue row failed", str(raised.exception))

    def test_a_row_left_pending_after_a_clean_drain_fails(self):
        cli_prints("QUEUED 1\nDRAIN Applied(1)\nDRAIN Empty\n")
        with self.assertRaises(gate_module.Failure) as raised:
            new_gate([("task_patch", "pending", 0, None)]).write({"kind": "board"})
        self.assertIn("queue not empty", str(raised.exception))

    def test_output_without_a_drain_line_fails(self):
        cli_prints("QUEUED 1\n")
        with self.assertRaises(gate_module.Failure):
            new_gate().write({"kind": "task_title"})


class RunTests(unittest.TestCase):
    def test_a_failed_kind_clears_the_queue_for_good(self):
        gate = new_gate([("task_patch", "pending", 0, None), ("board_write", "failed", 3, "HTTP 422")])

        def failing():
            raise gate_module.Failure("boom")

        gate.run("labels", failing)
        self.assertIsNotNone(gate.results["labels"])
        self.assertEqual(queue_rows(gate.db), [])

    def test_a_passing_kind_leaves_the_queue_alone(self):
        gate = new_gate([("task_patch", "pending", 0, None)])
        gate.run("task_title", lambda: None)
        self.assertIsNone(gate.results["task_title"])
        self.assertEqual(queue_rows(gate.db), [("task_patch",)])


if __name__ == "__main__":
    unittest.main()
