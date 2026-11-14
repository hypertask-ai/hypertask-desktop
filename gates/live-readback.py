import json
import os
import re
import sqlite3
import subprocess
import sys
import time
import urllib.error
import urllib.request

APP = "https://app.hypertask.ai"
BASE = APP + "/api/mcp"
BOARD = int(os.environ.get("HT_BOARD", "7049"))
FORBIDDEN_BOARDS = {15, 4060}
CLI = os.environ["HT_CORE_CLI"]
TOKEN = os.environ.get("HT_TOKEN", "")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EVIDENCE = os.path.join(ROOT, "evidence", "live-readback")
# hypertask-ai/hypertask src/lib/mcp/auth/rateLimit.ts: one token gets 120 requests per
# fixed one-minute window (600 for agent tokens) and the route wrapper rejects with 429
# before the handler runs, so a 429 request never changed server state and may be replayed.
# The gate needs more than one window, so it waits for the next window instead of failing.
RATE_LIMIT_WINDOW_SECONDS = 60
RATE_LIMIT_RETRIES = 4
RATE_LIMIT_WAIT_BUDGET_SECONDS = 900
QUEUE_RATE_LIMIT_ERRORS = ("HTTP 429", "Rate limit exceeded. Please slow down and try again shortly.")
waited_seconds = 0.0

STEP_NAMES = (
    "task_title", "task_section", "move_task", "task_description", "task_priority",
    "task_estimate", "task_due_date", "labels", "task_assignees", "assign_to_me",
    "parent_and_remove_subtask", "task_status", "star_task", "waiting_on", "toggle_timer",
    "move_to_inbox", "inbox_seen", "task_comment", "comment_edit", "comment_reaction",
    "comment_saved", "comment_delete", "comment_draft_and_discard", "compose_task", "columns",
    "rename_board", "inbox_remind_and_archive", "labels_clear",
)


class Failure(Exception):
    pass


class RateLimited(Exception):
    pass


def rate_limited(message):
    return "http 429" in (message or "").lower()


def window_wait(now=None):
    now = time.time() if now is None else now
    return float(RATE_LIMIT_WINDOW_SECONDS - (now % RATE_LIMIT_WINDOW_SECONDS)) + 2.0


def pause(reason):
    global waited_seconds
    wait = window_wait()
    if waited_seconds + wait > RATE_LIMIT_WAIT_BUDGET_SECONDS:
        raise Failure(f"rate limit wait budget of {RATE_LIMIT_WAIT_BUDGET_SECONDS}s is spent ({waited_seconds:.0f}s waited): {reason}")
    waited_seconds += wait
    print(f"LIVE_READBACK_WAIT {wait:.0f}s {reason}")
    time.sleep(wait)


def redact(text):
    if TOKEN:
        text = text.replace(TOKEN, "[token]")
    return text


def api(method, path, body=None, base=BASE):
    for attempt in range(RATE_LIMIT_RETRIES + 1):
        try:
            return api_once(method, path, body, base)
        except RateLimited as error:
            if attempt == RATE_LIMIT_RETRIES:
                raise Failure(str(error))
            pause(f"{method} {path}")


def api_once(method, path, body=None, base=BASE):
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(
        base + path,
        data=data,
        method=method,
        headers={"Authorization": f"Bearer {TOKEN}", "Accept": "application/json", "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            raw = response.read().decode()
    except urllib.error.HTTPError as error:
        detail = redact(error.read().decode(errors="replace")[:400])
        message = f"API {method} {path} status {error.code}: {detail}"
        if error.code == 429:
            raise RateLimited(message)
        raise Failure(message)
    payload = json.loads(raw) if raw.strip() else {}
    if isinstance(payload, dict) and payload.get("success") is False:
        detail = redact(json.dumps(payload)[:400])
        raise Failure(f"API {method} {path} refused: {detail}")
    return payload


def cli(db, *args, timeout=300, env=None):
    result = subprocess.run([CLI, "--db", db, *args], text=True, capture_output=True, timeout=timeout, env=env)
    output = redact((result.stderr or "") + (result.stdout or ""))
    if result.returncode != 0:
        raise Failure(f"cli {' '.join(args)[:80]} failed: {output[-500:]}")
    return output


def query(db, sql, params=()):
    connection = sqlite3.connect(db)
    try:
        return connection.execute(sql, params).fetchall()
    finally:
        connection.close()


def execute_sql(db, sql, params=()):
    connection = sqlite3.connect(db)
    try:
        with connection:
            connection.execute(sql, params)
    finally:
        connection.close()


def local(db, table, row_id):
    rows = query(db, f'SELECT raw_json FROM "{table}" WHERE id=?', (row_id,))
    return json.loads(rows[0][0]) if rows else None


def contains_id(value, wanted):
    if isinstance(value, dict):
        if value.get("id") == wanted:
            return True
        return any(contains_id(item, wanted) for item in value.values())
    if isinstance(value, list):
        return any(contains_id(item, wanted) for item in value)
    return False


def task_of(payload):
    task = payload.get("task")
    if isinstance(task, dict):
        return task
    tasks = payload.get("tasks")
    if isinstance(tasks, list) and tasks and isinstance(tasks[0], dict):
        return tasks[0]
    raise Failure("task response has no task")


def get_task(task_id):
    return task_of(api("GET", f"/tasks?task_id={task_id}"))


def need(condition, message):
    if not condition:
        raise Failure(message)


def index_of(value, key):
    if isinstance(value, dict):
        return value.get(key)
    return value


def comments_of(task_id):
    payload = api("GET", f"/comments?task_id={task_id}")
    rows = payload.get("comments")
    need(isinstance(rows, list), "comments response has no list")
    return rows


def sections_of(board):
    payload = api("GET", f"/projects/{board}/sections")
    rows = payload.get("sections")
    need(isinstance(rows, list), "sections response has no list")
    return rows


def section_title(row):
    return row.get("section_title") or row.get("title") or row.get("name")


def parent_of(task):
    return (task or {}).get("parentTaskId") or (task or {}).get("parent_id")


def timer_running_on(running, task_id):
    timers = running.get("timers") if isinstance(running, dict) else None
    return any(isinstance(timer, dict) and isinstance(timer.get("task"), dict) and timer["task"].get("id") == task_id for timer in timers or [])


class Gate:
    def __init__(self):
        self.nonce = str(time.time_ns())
        os.makedirs(EVIDENCE, exist_ok=True)
        self.db = os.path.join(EVIDENCE, "local.db")
        for suffix in ("", "-wal", "-shm"):
            if os.path.exists(self.db + suffix):
                os.remove(self.db + suffix)
        self.user = None
        self.results = {}
        self.reopen = []
        self.created_tasks = []
        self.notification_archived = None
        self.board_title = None
        self.extra_columns = []
        self.timer_task = None
        self.rate_limit_waits = 0

    def read_with_retry(self, db, reason, *args):
        for attempt in range(RATE_LIMIT_RETRIES + 1):
            try:
                cli(db, *args)
                return
            except Failure as error:
                if attempt == RATE_LIMIT_RETRIES or not rate_limited(str(error)):
                    raise
                self.rate_limit_waits += 1
                pause(reason)

    def sync(self, db=None):
        self.read_with_retry(db or self.db, "sync --board", "sync", "--board", str(BOARD))

    def fetch_task(self, task_id):
        self.read_with_retry(self.db, f"task --task {task_id}", "task", "--task", str(task_id))

    def drain_outcomes(self, output):
        outcomes = re.findall(r"DRAIN (\w+)", output)
        need(outcomes, f"no drain result: {output[-300:]}")
        return outcomes

    def write(self, payload):
        outcomes = self.drain_outcomes(cli(self.db, "write-json", json.dumps(payload)))
        retries = 0
        while outcomes[-1] == "Retrying" and self.rate_limit_pending() and retries < RATE_LIMIT_RETRIES:
            retries += 1
            self.rate_limit_waits += 1
            pause(f"drain after {outcomes[-1]}")
            outcomes += self.drain_outcomes(cli(self.db, "drain"))
        applied = [outcome for outcome in outcomes if outcome != "Retrying"]
        need(applied, f"write was {outcomes[-1]}: {self.queue_state()}")
        need(applied[0] == "Applied", f"write was {applied[0]}: {self.queue_state()}")
        need(all(o == "Applied" for o in applied[:-1]) and applied[-1] == "Empty", f"drain ended {outcomes}: {self.queue_state()}")
        self.require_empty_queue()

    def queue_state(self):
        return query(self.db, "SELECT kind,status,attempts,last_error FROM mutation_queue")

    def rate_limit_pending(self):
        pending = [row for row in self.queue_state() if row[1] == "pending"]
        return bool(pending) and all(row[3] in QUEUE_RATE_LIMIT_ERRORS for row in pending)

    def require_empty_queue(self):
        left = self.queue_state()
        failed = [row for row in left if row[1] == "failed"]
        need(not failed, f"queue row failed: {failed}")
        need(not left, f"queue not empty after drain: {left}")

    def expect_local(self, name, description, check):
        self.reopen.append((name, description, check))

    def run(self, name, fn):
        try:
            fn()
            self.results[name] = None
            print(f"KIND_PASS {name}")
        except (Failure, KeyError, IndexError, TypeError, ValueError, json.JSONDecodeError, subprocess.TimeoutExpired) as error:
            self.results[name] = redact(f"{type(error).__name__}: {error}")[:500]
            print(f"KIND_FAIL {name}: {self.results[name]}")
            try:
                execute_sql(self.db, "DELETE FROM mutation_queue")
            except sqlite3.Error:
                pass

    def make_task(self, label):
        created = api("POST", "/tasks/create", {"project_id": BOARD, "title": f"HTPR-7381 {label} {self.nonce}"})
        task_id = created["task"]["id"]
        self.created_tasks.append(task_id)
        return task_id


REOPEN_OWNER = {"create_label": "labels", "remove_subtask": "parent_and_remove_subtask", "inbox_archive": "inbox_remind_and_archive"}


def report(gate):
    reopen_failed = [key for key, value in gate.results.items() if key.startswith("reopen:") and value]
    reopened_kinds = {REOPEN_OWNER.get(key.split(":", 2)[2], key.split(":", 2)[2]) for key in reopen_failed}
    passed = sum(1 for name in STEP_NAMES if name in gate.results and gate.results[name] is None and name not in reopened_kinds)
    if reopen_failed:
        print(f"REOPEN_FAILED {len(reopen_failed)}")
    if gate.rate_limit_waits:
        print(f"LIVE_READBACK_RATE_LIMIT_WAITS {gate.rate_limit_waits}")
    print(f"KINDS_PASSED {passed}/{len(STEP_NAMES)}")
    return 0 if passed == len(STEP_NAMES) and not reopen_failed else 1


def main():
    if not TOKEN:
        print("LIVE_READBACK_NO_TOKEN")
        return 1
    if BOARD in FORBIDDEN_BOARDS:
        print(f"LIVE_READBACK_REFUSED board {BOARD}")
        return 1
    gate = Gate()
    status = 1
    try:
        execute(gate)
        status = 0
    except (Exception, SystemExit, KeyboardInterrupt) as error:
        print(f"LIVE_READBACK_CRASH {type(error).__name__}: {redact(str(error))[:300]}")
        status = 1
    cleanup_failed = []
    try:
        cleanup_failed = cleanup(gate)
    except (Exception, SystemExit, KeyboardInterrupt) as error:
        cleanup_failed = [f"cleanup crashed: {type(error).__name__}: {redact(str(error))[:200]}"]
    if cleanup_failed:
        print("CLEANUP_FAILED " + "; ".join(cleanup_failed))
    counted = report(gate)
    if status == 0 and counted == 0 and not cleanup_failed:
        print("LIVE_READBACK_OK")
        return 0
    return 1


def parent_kind(gate, task, parent):
    def api_parent():
        return parent_of(get_task(task))

    def local_parent():
        return parent_of(local(gate.db, "Task", task))

    def local_children():
        return (local(gate.db, "Task", parent) or {}).get("sub_tasks")

    def set_parent():
        gate.write({"kind": "task_patch", "task_id": task, "patch": {"Parent": parent}})
        need(api_parent() == parent, "API parent missing")
        need(local_parent() == parent, "local task lacks the new parent")

    set_parent()
    gate.write({"kind": "task_patch", "task_id": task, "patch": {"Parent": None}})
    need(not api_parent(), "API parent not removed by Parent(None)")
    need(local(gate.db, "Task", task) is not None and not local_parent(), "local task still has its parent after Parent(None)")

    set_parent()
    gate.fetch_task(parent)
    need(contains_id(local_children(), task), f"parent's cached sub-tasks lack the child after a fresh task fetch: {redact(json.dumps(local_children())[:200])}")
    gate.write({"kind": "board", "change": {"RemoveSubtask": {"task": task, "parent": parent}}})
    need(not api_parent(), "API parent not removed by RemoveSubtask")
    need(not contains_id(get_task(parent).get("sub_tasks"), task), "API parent still lists the removed sub-task")
    need(not local_parent(), "local task still has its parent after RemoveSubtask")
    need(not contains_id(local_children(), task), "parent's cached sub-tasks still list the removed sub-task")
    gate.expect_local("remove_subtask", "Task parent and parent's sub-task list cleared", lambda db: local(db, "Task", task) is not None and not parent_of(local(db, "Task", task)) and not contains_id((local(db, "Task", parent) or {}).get("sub_tasks"), task))


def timer_kind(gate, task):
    toggle = {"kind": "board", "change": {"ToggleTimer": {"task": task}}}
    summary = api("POST", "/time/status", {"task": str(task)}).get("summary") or {}
    need(summary.get("enabled") is True, f"board {BOARD} has time tracking disabled: POST /time/status said {redact(json.dumps(summary)[:300])}")
    gate.timer_task = task
    gate.write(toggle)
    running = api("GET", "/time/running")
    need(timer_running_on(running, task), f"API timer not running: GET /time/running said {redact(json.dumps(running)[:400])}")
    gate.write(toggle)
    running = api("GET", "/time/running")
    need(not timer_running_on(running, task), f"API timer still running: GET /time/running said {redact(json.dumps(running)[:400])}")
    gate.timer_task = None


def labels_clear_kind(gate, task):
    gate.write({"kind": "task_patch", "task_id": task, "patch": {"Labels": []}})
    need(not (get_task(task).get("labels") or []), "API labels not cleared")
    need(not ((local(gate.db, "Task", task) or {}).get("labels") or []), "local task still carries the labels")
    gate.sync()
    need(not ((local(gate.db, "Task", task) or {}).get("labels") or []), "labels came back after a resync")


def label_reopen_check(name, task):
    def check(db):
        board_label = any(name in row[0] for row in query(db, 'SELECT raw_json FROM "Label" WHERE projectId=?', (BOARD,)))
        task_label = name in json.dumps((local(db, "Task", task) or {}).get("labels"))
        return board_label and task_label

    return check


def execute(gate):
    nonce = gate.nonce
    context = api("GET", "/user/context")
    gate.user = context["user"]["id"]
    projects = api("GET", f"/projects?status=Normal&search=QA%20Sandbox&limit=100")
    boards = [p for p in projects.get("projects", []) if p.get("id") == BOARD]
    need(boards, f"board {BOARD} is not available to this login")
    gate.board_title = boards[0].get("title") or boards[0].get("name")

    main_task = gate.make_task("main")
    parent_task = gate.make_task("parent")
    assign_task = gate.make_task("assign")
    gate.sync()
    cli(gate.db, "dump")
    query(gate.db, "SELECT 1")
    connection = sqlite3.connect(gate.db)
    connection.execute("INSERT OR REPLACE INTO kv(key,value) VALUES('current-user-id',?)", (str(gate.user),))
    connection.commit()
    connection.close()
    sections = query(gate.db, 'SELECT id,section_title FROM "Section" WHERE projectId=? AND deleted=0 AND visibility=1 ORDER BY id', (BOARD,))
    need(len(sections) >= 2, f"board {BOARD} needs two visible columns, has {len(sections)}")
    def other_column(task_id):
        current = local(gate.db, "Task", task_id)["sectionId"]
        return next(row[0] for row in sections if row[0] != current)

    parent_target = other_column(parent_task)
    main_target = other_column(main_task)

    def title():
        renamed = f"HTPR-7381 renamed {nonce}"
        gate.write({"kind": "task_title", "task_id": main_task, "title": renamed})
        need(get_task(main_task).get("title") == renamed, "API title differs")
        gate.expect_local("task_title", "Task title", lambda db: (local(db, "Task", main_task) or {}).get("title") == renamed)
        print(f"BOARD_READBACK task {main_task}")

    def section():
        gate.write({"kind": "task_section", "task_id": parent_task, "section_id": parent_target})
        need(get_task(parent_task).get("sectionId") == parent_target, "API column differs")
        gate.expect_local("task_section", "Task column", lambda db: (local(db, "Task", parent_task) or {}).get("sectionId") == parent_target)

    def move():
        gate.write({"kind": "board", "change": {"MoveTask": {"task": main_task, "project": BOARD, "section": main_target}}})
        need(get_task(main_task).get("sectionId") == main_target, "API column after move differs")
        gate.expect_local("move_task", "Task column after move", lambda db: (local(db, "Task", main_task) or {}).get("sectionId") == main_target)

    def description():
        html = f"<p>HTPR-7381 description {nonce}</p>"
        gate.write({"kind": "task_patch", "task_id": main_task, "patch": {"Description": html}})
        need(nonce in (get_task(main_task).get("description") or ""), "API description differs")
        gate.expect_local("task_description", "Task description", lambda db: nonce in ((local(db, "Task", main_task) or {}).get("description") or ""))

    def priority():
        gate.write({"kind": "task_patch", "task_id": main_task, "patch": {"Priority": 2}})
        need(index_of(get_task(main_task).get("priority"), "priority_index") == 2, "API priority differs")
        gate.expect_local("task_priority", "Task priority", lambda db: index_of((local(db, "Task", main_task) or {}).get("priority"), "priority_index") == 2)

    def estimate():
        gate.write({"kind": "task_patch", "task_id": main_task, "patch": {"Estimate": 3}})
        need(index_of(get_task(main_task).get("estimate"), "estimate_index") == 3, "API estimate differs")
        gate.expect_local("task_estimate", "Task size", lambda db: index_of((local(db, "Task", main_task) or {}).get("estimate"), "estimate_index") == 3)

    def due_date():
        gate.write({"kind": "task_patch", "task_id": main_task, "patch": {"DueDate": "2031-06-01T09:00:00Z"}})
        need(str(get_task(main_task).get("dueDate") or "").startswith("2031-06-01"), "API due date differs")
        gate.expect_local("task_due_date", "Task due date", lambda db: str((local(db, "Task", main_task) or {}).get("dueDate") or "").startswith("2031-06-01"))

    def label_names(task):
        return [item.get("value") or item.get("name") or item.get("label") if isinstance(item, dict) else item for item in (task.get("labels") or [])]

    def label():
        name = f"ht7381-{nonce[-8:]}"
        gate.write({"kind": "board", "change": {"CreateTaskLabel": {"project": BOARD, "task": main_task, "name": name}}})
        need(name in json.dumps(get_task(main_task).get("labels")), "API label missing")
        need(name in json.dumps((local(gate.db, "Task", main_task) or {}).get("labels")), "local task lacks the new label")
        gate.expect_local("create_label", "Board label and task label", label_reopen_check(name, main_task))

    def assignees():
        gate.write({"kind": "task_patch", "task_id": main_task, "patch": {"Assignees": [gate.user]}})
        need(contains_id(get_task(main_task).get("assignees"), gate.user), "API assignee missing")
        gate.expect_local("task_assignees", "Task assignee", lambda db: contains_id((local(db, "Task", main_task) or {}).get("assignees"), gate.user))

    def assign_me():
        gate.write({"kind": "board", "change": {"AssignToMe": {"task": assign_task, "user": gate.user}}})
        need(contains_id(get_task(assign_task).get("assignees"), gate.user), "API self assignment missing")
        gate.expect_local("assign_to_me", "Task self assignment", lambda db: contains_id((local(db, "Task", assign_task) or {}).get("assignees"), gate.user))

    def parent():
        parent_kind(gate, main_task, parent_task)

    def status():
        gate.write({"kind": "task_patch", "task_id": assign_task, "patch": {"Status": "Archive"}})
        need(get_task(assign_task).get("status") == "Archive", "API archive status differs")
        gate.write({"kind": "task_patch", "task_id": assign_task, "patch": {"Status": "Normal"}})
        need(get_task(assign_task).get("status") == "Normal", "API restored status differs")
        gate.expect_local("task_status", "Task status restored", lambda db: (local(db, "Task", assign_task) or {}).get("status") == "Normal")

    def star():
        gate.write({"kind": "board", "change": {"StarTask": {"task": main_task, "project": BOARD, "active": True}}})
        need(get_task(main_task).get("savedContent"), "API star missing")
        gate.write({"kind": "board", "change": {"StarTask": {"task": main_task, "project": BOARD, "active": False}}})
        need(not get_task(main_task).get("savedContent"), "API star not removed")
        gate.expect_local("star_task", "Task star removed", lambda db: not (local(db, "Task", main_task) or {}).get("savedContent"))

    def waiting_on():
        gate.write({"kind": "board", "change": {"WaitingOn": {"task": main_task, "user": gate.user}}})
        need(get_task(main_task).get("waitingOnUserId") == gate.user, "API waiting-on missing")
        gate.write({"kind": "board", "change": {"WaitingOn": {"task": main_task, "user": None}}})
        need(get_task(main_task).get("waitingOnUserId") is None, "API waiting-on not cleared")
        gate.expect_local("waiting_on", "Task waiting-on cleared", lambda db: (local(db, "Task", main_task) or {}).get("waitingOnUserId") is None)

    def timer():
        timer_kind(gate, assign_task)

    moved_notification = {}

    def move_to_inbox():
        gate.write({"kind": "board", "change": {"MoveToInbox": {"task": assign_task, "project": BOARD}}})
        listing = api("GET", "/inbox/list")
        rows = [row for row in listing.get("user_notifications", []) if row.get("taskId") == assign_task and row.get("status") == "Normal"]
        need(rows, "API inbox lacks the moved task")
        moved_notification["id"] = rows[0]["id"]

    def inbox_seen():
        need("id" in moved_notification, "no moved-task notification to mark seen")
        gate.sync()
        notification_id = moved_notification["id"]
        gate.write({"kind": "inbox_seen", "notification_id": notification_id})
        listing = api("GET", "/inbox/list")
        row = next((r for r in listing.get("user_notifications", []) if r.get("id") == notification_id), None)
        need(row is not None and row.get("seen") is True, f"API seen flag differs: {None if row is None else row.get('seen')}")
        gate.expect_local("inbox_seen", "Inbox seen", lambda db: (local(db, "Notification", notification_id) or {}).get("seen") in (1, True))

    comment_state = {}

    def comment_send():
        text = f"<p>HTPR-7381 comment {nonce}</p>"
        gate.write({"kind": "task_comment", "task_id": main_task, "text": text})
        rows = [row for row in comments_of(main_task) if nonce in json.dumps(row)]
        need(len(rows) == 1, f"API comments hold {len(rows)} matching rows")
        comment_state["id"] = rows[0]["id"]
        print(f"TASK_READBACK task {main_task}")
        gate.expect_local("task_comment", "Task comment", lambda db: any(nonce in json.dumps(json.loads(r[0])) for r in query(db, 'SELECT raw_json FROM "Comment" WHERE taskId=? AND id>0', (main_task,))))

    def comment_cached():
        need("id" in comment_state, "no comment to act on")
        if local(gate.db, "Comment", comment_state["id"]) is None:
            gate.sync()
        need(local(gate.db, "Comment", comment_state["id"]) is not None, "comment is not in the local cache")

    def comment_edit():
        comment_cached()
        edited = f"<p>HTPR-7381 edited {nonce}</p>"
        gate.write({"kind": "comment_action", "task_id": main_task, "comment_id": comment_state["id"], "change": {"Edit": {"text": edited}}})
        row = next(r for r in comments_of(main_task) if r["id"] == comment_state["id"])
        need(f"edited {nonce}" in json.dumps(row), "API comment text not edited")
        gate.expect_local("comment_edit", "Comment edit", lambda db: f"edited {nonce}" in json.dumps(local(db, "Comment", comment_state["id"]) or {}))

    def comment_reaction():
        comment_cached()
        gate.write({"kind": "comment_action", "task_id": main_task, "comment_id": comment_state["id"], "change": {"Reaction": {"emoji": "\U0001F44D", "active": True}}})
        row = next(r for r in comments_of(main_task) if r["id"] == comment_state["id"])
        need("\U0001F44D" in json.dumps(row, ensure_ascii=False), "API reaction missing")
        gate.write({"kind": "comment_action", "task_id": main_task, "comment_id": comment_state["id"], "change": {"Reaction": {"emoji": "\U0001F44D", "active": False}}})
        row = next(r for r in comments_of(main_task) if r["id"] == comment_state["id"])
        need("\U0001F44D" not in json.dumps(row, ensure_ascii=False), "API reaction not removed")

    def comment_saved():
        comment_cached()
        gate.write({"kind": "comment_action", "task_id": main_task, "comment_id": comment_state["id"], "change": {"Saved": {"public": False}}})
        saved = api("GET", "/api/savedContent/getAll", base=APP)
        need(str(comment_state["id"]) in json.dumps(saved), "API saved comment missing")
        gate.write({"kind": "comment_action", "task_id": main_task, "comment_id": comment_state["id"], "change": {"Saved": {"public": False}}})
        saved = api("GET", "/api/savedContent/getAll", base=APP)
        need(f'"commentId": "{comment_state["id"]}"' not in json.dumps(saved) and f'"commentId": {comment_state["id"]}' not in json.dumps(saved), "API saved comment not removed")

    def comment_delete():
        marker = f"doomed {nonce}"
        gate.write({"kind": "task_comment", "task_id": main_task, "text": f"<p>HTPR-7381 {marker}</p>"})
        rows = [row for row in comments_of(main_task) if marker in json.dumps(row)]
        need(len(rows) == 1, f"API comments hold {len(rows)} rows to delete")
        doomed = rows[0]["id"]
        gate.sync()
        need(local(gate.db, "Comment", doomed) is not None, "comment to delete is not in the local cache")
        gate.write({"kind": "comment_action", "task_id": main_task, "comment_id": doomed, "change": "Delete"})
        need(not any(r["id"] == doomed for r in comments_of(main_task)), "API comment still present")
        gate.expect_local("comment_delete", "Comment removed", lambda db: local(db, "Comment", doomed) is None)

    def draft():
        text = f"<p>HTPR-7381 draft {nonce}</p>"
        gate.write({"kind": "comment_draft", "task_id": main_task, "text": text})
        drafts = api("GET", f"/drafts?task_id={main_task}")
        need(nonce in json.dumps(drafts), "API draft missing")
        gate.write({"kind": "discard_draft", "task_id": main_task, "draft_type": "comment"})
        drafts = api("GET", f"/drafts?task_id={main_task}")
        need(nonce not in json.dumps(drafts), "API draft not discarded")

    def compose():
        text = f"HTPR-7381 compose {nonce}"
        gate.write({"kind": "compose_task", "project_id": BOARD, "text": text, "time_zone": "Europe/London"})
        rows = query(gate.db, 'SELECT id FROM "Task" WHERE title LIKE ? AND id>0', (f"%{nonce}%compose%",)) or query(gate.db, 'SELECT id FROM "Task" WHERE raw_json LIKE ? AND id>0 AND id NOT IN (?,?,?)', (f"%compose {nonce}%", main_task, parent_task, assign_task))
        need(rows, "composed task is not in the local cache")
        composed = rows[0][0]
        gate.created_tasks.append(composed)
        need(nonce in json.dumps(get_task(composed)), "API composed task differs")
        gate.expect_local("compose_task", "Composed task", lambda db: local(db, "Task", composed) is not None)

    def columns():
        name = f"ht7381 column {nonce[-8:]}"
        renamed = name + " renamed"
        gate.write({"kind": "board", "change": {"AddColumn": {"project": BOARD, "title": name}}})
        rows = [row for row in sections_of(BOARD) if section_title(row) == name]
        need(len(rows) == 1, "API column missing after add")
        column = rows[0]["id"]
        gate.extra_columns.append(column)
        gate.sync()
        gate.write({"kind": "board", "change": {"RenameColumn": {"project": BOARD, "section": column, "title": renamed}}})
        need(any(section_title(row) == renamed for row in sections_of(BOARD)), "API column not renamed")
        gate.write({"kind": "board", "change": {"DeleteColumn": {"project": BOARD, "section": column}}})
        listed = [row for row in sections_of(BOARD) if row.get("id") == column and not row.get("deleted")]
        need(not listed, "API column still active")
        gate.extra_columns.remove(column)
        gate.expect_local("columns", "Column removed", lambda db: (local(db, "Section", column) or {}).get("deleted") in (None, 1, True))

    def rename_board():
        renamed = f"{gate.board_title} ht{nonce[-6:]}"
        gate.write({"kind": "board", "change": {"RenameBoard": {"project": BOARD, "title": renamed}}})
        found = api("GET", f"/projects?status=Normal&search=ht{nonce[-6:]}&limit=100")
        need(any(p.get("id") == BOARD for p in found.get("projects", [])), "API board title not renamed")
        api("PATCH", f"/projects/{BOARD}", {"title": gate.board_title})
        restored = api("GET", f"/projects?status=Normal&search=QA%20Sandbox&limit=100")
        need(any(p.get("id") == BOARD and (p.get("title") or p.get("name")) == gate.board_title for p in restored.get("projects", [])), "board title was not restored")

    def inbox_archive_and_remind():
        rows = query(gate.db, 'SELECT id FROM "Notification" WHERE status=\'Normal\' AND id>0 AND taskId IS NOT NULL AND projectId IS NOT NULL ORDER BY id LIMIT 1')
        need(rows, "INBOX_EMPTY")
        notification_id = rows[0][0]
        remind_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(time.time() + 3 * 86400))
        gate.notification_archived = notification_id
        gate.write({"kind": "inbox_remind", "notification_ids": [notification_id], "remind_at": remind_at})
        listing = api("GET", "/inbox/list")
        need(not contains_id(listing.get("user_notifications", []), notification_id) or any(r.get("id") == notification_id and r.get("status") != "Normal" for r in listing.get("user_notifications", [])), "API still lists the reminded row as active")
        restored = api("POST", "/inbox/unarchive", {"notification_ids": [notification_id]})
        need(restored.get("unarchived_count") == 1, f"unarchive count {restored.get('unarchived_count')}")
        gate.sync()
        gate.write({"kind": "inbox_archive", "notification_id": notification_id})
        listing = api("GET", "/inbox/list")
        need(not any(r.get("id") == notification_id and r.get("status") == "Normal" for r in listing.get("user_notifications", [])), f"notification {notification_id} is still active")
        print(f"INBOX_READBACK notification {notification_id}")
        gate.expect_local("inbox_archive", "Inbox archive", lambda db: (local(db, "Notification", notification_id) or {}).get("status") != "Normal")
        restored = api("POST", "/inbox/unarchive", {"notification_ids": [notification_id]})
        need(restored.get("unarchived_count") == 1, f"unarchive count {restored.get('unarchived_count')}")
        gate.notification_archived = None
        gate.reopen[:] = [item for item in gate.reopen if item[0] != "inbox_archive"]

    def labels_clear():
        labels_clear_kind(gate, main_task)

    steps = dict([
        ("task_title", title), ("task_section", section), ("move_task", move),
        ("task_description", description), ("task_priority", priority), ("task_estimate", estimate),
        ("task_due_date", due_date), ("labels", label), ("task_assignees", assignees),
        ("assign_to_me", assign_me), ("parent_and_remove_subtask", parent), ("task_status", status),
        ("star_task", star), ("waiting_on", waiting_on), ("toggle_timer", timer),
        ("move_to_inbox", move_to_inbox), ("inbox_seen", inbox_seen),
        ("task_comment", comment_send), ("comment_edit", comment_edit), ("comment_reaction", comment_reaction),
        ("comment_saved", comment_saved), ("comment_delete", comment_delete),
        ("comment_draft_and_discard", draft), ("compose_task", compose), ("columns", columns),
        ("rename_board", rename_board), ("inbox_remind_and_archive", inbox_archive_and_remind),
    ])
    after_reopen = {"labels_clear": labels_clear}
    need(set(steps) | set(after_reopen) == set(STEP_NAMES), "step list and STEP_NAMES disagree")
    for name in STEP_NAMES:
        if name in steps:
            gate.run(name, steps[name])
    reopen_phase(gate)
    for name, fn in after_reopen.items():
        gate.run(name, fn)


def reopen_phase(gate):
    def check(phase, db):
        for name, description, predicate in gate.reopen:
            try:
                ok = bool(predicate(db))
            except (sqlite3.Error, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
                ok = False
                description = f"{description} ({type(error).__name__})"
            print(f"REOPEN_{'PASS' if ok else 'FAIL'} {phase} {name}")
            if not ok:
                gate.results[f"reopen:{phase}:{name}"] = description

    cli(gate.db, "dump")
    check("same_database", gate.db)
    gate.sync()
    check("after_resync", gate.db)
    fresh = os.path.join(EVIDENCE, "fresh.db")
    for suffix in ("", "-wal", "-shm"):
        if os.path.exists(fresh + suffix):
            os.remove(fresh + suffix)
    gate.sync(fresh)
    skip_fresh = {"task_comment", "comment_edit", "comment_delete", "compose_task", "inbox_seen"}
    gate.reopen[:] = [item for item in gate.reopen if item[0] not in skip_fresh]
    check("fresh_database", fresh)


def cleanup(gate):
    problems = []

    def attempt(label, action):
        try:
            action()
        except (Failure, SystemExit, KeyError, TypeError) as error:
            problems.append(f"{label}: {redact(str(error))[:200]}")

    def stop_running_timer(task_id):
        if timer_running_on(api("GET", "/time/running"), task_id):
            api("POST", "/time/stop", {"task": str(task_id)})

    if gate.timer_task is not None:
        attempt("stop timer", lambda: stop_running_timer(gate.timer_task))
    if gate.notification_archived is not None:
        attempt("unarchive notification", lambda: api("POST", "/inbox/unarchive", {"notification_ids": [gate.notification_archived]}))
    if gate.board_title:
        attempt("restore board title", lambda: api("PATCH", f"/projects/{BOARD}", {"title": gate.board_title}))
    for column in gate.extra_columns:
        attempt(f"delete column {column}", lambda column=column: api("DELETE", f"/projects/{BOARD}/sections/{column}"))
    for task_id in gate.created_tasks:
        attempt(f"delete task {task_id}", lambda task_id=task_id: api("POST", "/tasks/delete", {"task_id": task_id}))
    if not problems:
        print("CLEANUP_OK")
    return problems


if __name__ == "__main__":
    sys.exit(main())
