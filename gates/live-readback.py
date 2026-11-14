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


class Failure(Exception):
    pass


def redact(text):
    if TOKEN:
        text = text.replace(TOKEN, "[token]")
    return text


def api(method, path, body=None, base=BASE):
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
        raise Failure(f"API {method} {path} status {error.code}: {detail}")
    payload = json.loads(raw) if raw.strip() else {}
    if isinstance(payload, dict) and payload.get("success") is False:
        raise Failure(f"API {method} {path} refused: {redact(json.dumps(payload)[:400])}")
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

    def sync(self, db=None):
        cli(db or self.db, "sync", "--board", str(BOARD))

    def write(self, payload):
        output = cli(self.db, "write-json", json.dumps(payload))
        drained = re.search(r"DRAIN (\w+)", output)
        need(drained is not None, f"no drain result: {output[-300:]}")
        need(drained.group(1) == "Applied", f"write was {drained.group(1)}: {self.queue_state()}")
        self.require_empty_queue()

    def queue_state(self):
        return query(self.db, "SELECT kind,status,attempts,last_error FROM mutation_queue")

    def require_empty_queue(self):
        left = self.queue_state()
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
                query(self.db, "DELETE FROM mutation_queue")
            except sqlite3.Error:
                pass

    def make_task(self, label):
        created = api("POST", "/tasks/create", {"project_id": BOARD, "title": f"HTPR-7381 {label} {self.nonce}"})
        task_id = created["task"]["id"]
        self.created_tasks.append(task_id)
        return task_id


def main():
    if not TOKEN:
        print("LIVE_READBACK_NO_TOKEN")
        return 1
    if BOARD in FORBIDDEN_BOARDS:
        print(f"LIVE_READBACK_REFUSED board {BOARD}")
        return 1
    gate = Gate()
    try:
        return execute(gate)
    finally:
        cleanup_failed = cleanup(gate)
        if cleanup_failed:
            print("CLEANUP_FAILED " + "; ".join(cleanup_failed))
            sys.exit(1)


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
    home_section = local(gate.db, "Task", main_task)["sectionId"]
    other_section = next(row[0] for row in sections if row[0] != home_section)

    def title():
        renamed = f"HTPR-7381 renamed {nonce}"
        gate.write({"kind": "task_title", "task_id": main_task, "title": renamed})
        need(get_task(main_task).get("title") == renamed, "API title differs")
        gate.expect_local("task_title", "Task title", lambda db: (local(db, "Task", main_task) or {}).get("title") == renamed)
        print(f"BOARD_READBACK task {main_task}")

    def section():
        gate.write({"kind": "task_section", "task_id": main_task, "section_id": other_section})
        need(get_task(main_task).get("sectionId") == other_section, "API column differs")
        gate.expect_local("task_section", "Task column", lambda db: (local(db, "Task", main_task) or {}).get("sectionId") == other_section)

    def move():
        gate.write({"kind": "board", "change": {"MoveTask": {"task": main_task, "project": BOARD, "section": home_section}}})
        need(get_task(main_task).get("sectionId") == home_section, "API column after move differs")
        gate.expect_local("move_task", "Task column after move", lambda db: (local(db, "Task", main_task) or {}).get("sectionId") == home_section)

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
        gate.expect_local("create_label", "Task label", lambda db: name in json.dumps((local(db, "Task", main_task) or {}).get("labels")))
        gate.write({"kind": "task_patch", "task_id": main_task, "patch": {"Labels": []}})
        need(not (get_task(main_task).get("labels") or []), "API labels not cleared")
        gate.expect_local("task_labels_clear", "Task labels cleared", lambda db: not ((local(db, "Task", main_task) or {}).get("labels") or []))

    def assignees():
        gate.write({"kind": "task_patch", "task_id": main_task, "patch": {"Assignees": [gate.user]}})
        need(contains_id(get_task(main_task).get("assignees"), gate.user), "API assignee missing")
        gate.expect_local("task_assignees", "Task assignee", lambda db: contains_id((local(db, "Task", main_task) or {}).get("assignees"), gate.user))

    def assign_me():
        gate.write({"kind": "board", "change": {"AssignToMe": {"task": assign_task, "user": gate.user}}})
        need(contains_id(get_task(assign_task).get("assignees"), gate.user), "API self assignment missing")
        gate.expect_local("assign_to_me", "Task self assignment", lambda db: contains_id((local(db, "Task", assign_task) or {}).get("assignees"), gate.user))

    def parent():
        gate.write({"kind": "task_patch", "task_id": main_task, "patch": {"Parent": parent_task}})
        got = get_task(main_task)
        need((got.get("parentTaskId") or got.get("parent_id")) == parent_task, "API parent missing")
        gate.write({"kind": "board", "change": {"RemoveSubtask": {"task": main_task, "parent": parent_task}}})
        got = get_task(main_task)
        need(not (got.get("parentTaskId") or got.get("parent_id")), "API parent not removed")
        gate.expect_local("remove_subtask", "Task parent removed", lambda db: not (local(db, "Task", main_task) or {}).get("parentTaskId"))

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
        gate.write({"kind": "board", "change": {"ToggleTimer": {"task": assign_task}}})
        running = api("GET", "/time/running")
        need(contains_id(running, assign_task) or str(assign_task) in json.dumps(running), "API timer not running")
        gate.write({"kind": "board", "change": {"ToggleTimer": {"task": assign_task}}})
        running = api("GET", "/time/running")
        need(not (contains_id(running, assign_task) or str(assign_task) in json.dumps(running)), "API timer still running")

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
        comment_cached()
        gate.write({"kind": "comment_action", "task_id": main_task, "comment_id": comment_state["id"], "change": "Delete"})
        need(not any(r["id"] == comment_state["id"] for r in comments_of(main_task)), "API comment still present")
        gate.expect_local("comment_delete", "Comment removed", lambda db: local(db, "Comment", comment_state["id"]) is None)

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

    steps = [
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
    ]
    for name, fn in steps:
        gate.run(name, fn)

    reopen_phase(gate)
    failed = {k: v for k, v in gate.results.items() if v}
    print(f"KINDS_PASSED {len(gate.results) - len(failed)}/{len(gate.results)}")
    if failed:
        return 1
    print("LIVE_READBACK_OK")
    return 0


def reopen_phase(gate):
    def check(phase, db):
        for name, description, predicate in gate.reopen:
            if gate.results.get(name) is None or name not in gate.results:
                pass
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
