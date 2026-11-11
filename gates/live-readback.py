import json
import os
import sqlite3
import subprocess
import sys
import time
import urllib.error
import urllib.request

BASE = "https://app.hypertask.ai/api/mcp"
BOARD = int(os.environ.get("HT_BOARD", "15"))
CLI = os.environ["HT_CORE_CLI"]
TOKEN = os.environ.get("HT_TOKEN", "")
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def redact(text):
    if TOKEN:
        text = text.replace(TOKEN, "[token]")
    return text


def api(method, path, body=None):
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(
        BASE + path,
        data=data,
        method=method,
        headers={"Authorization": f"Bearer {TOKEN}", "Accept": "application/json", "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            payload = json.loads(response.read().decode())
    except urllib.error.HTTPError as error:
        detail = redact(error.read().decode(errors="replace")[:500])
        raise SystemExit(f"API {method} {path} status {error.code}: {detail}")
    if payload.get("success") is False:
        raise SystemExit(f"API {method} {path} refused: {redact(json.dumps(payload)[:500])}")
    return payload


def cli(db, *args, timeout=300):
    result = subprocess.run([CLI, "--db", db, *args], text=True, capture_output=True, timeout=timeout)
    output = redact((result.stderr or "") + (result.stdout or ""))
    if result.returncode != 0:
        raise SystemExit(f"cli {' '.join(args)} failed: {output[-800:]}")
    return output


def query(db, sql, params=()):
    connection = sqlite3.connect(db)
    try:
        return connection.execute(sql, params).fetchall()
    finally:
        connection.close()


def contains_id(value, wanted):
    if isinstance(value, dict):
        if value.get("id") == wanted:
            return True
        return any(contains_id(item, wanted) for item in value.values())
    if isinstance(value, list):
        return any(contains_id(item, wanted) for item in value)
    return False


def task_title(payload):
    task = payload.get("task")
    if isinstance(task, dict) and "title" in task:
        return task["title"]
    tasks = payload.get("tasks")
    if isinstance(tasks, list) and tasks and isinstance(tasks[0], dict):
        return tasks[0].get("title")
    return None


def main():
    if not TOKEN:
        print("LIVE_READBACK_NO_TOKEN")
        return 1
    nonce = str(time.time_ns())
    created = f"HTPR-7381 readback {nonce}"
    renamed = f"HTPR-7381 renamed {nonce}"
    comment = f"<p>HTPR-7381 comment {nonce}</p>"
    db = os.path.join(ROOT, "evidence", "live-readback", "local.db")
    os.makedirs(os.path.dirname(db), exist_ok=True)
    if os.path.exists(db):
        os.remove(db)
    created_task = None
    notification_id = None
    archived = False
    code = 1
    cleanup_error = None
    try:
        created_body = api("POST", "/tasks/create", {"project_id": BOARD, "title": created})
        created_task = created_body["task"]["id"]
        cli(db, "sync", "--board", str(BOARD))
        cached = query(db, 'SELECT title FROM Task WHERE id=?', (created_task,))
        if not cached:
            raise SystemExit(f"synced board did not contain task {created_task}")
        cli(db, "write", "task_title", str(created_task), renamed)
        failed = query(db, "SELECT status, last_error FROM mutation_queue WHERE kind='task_title'")
        if failed:
            raise SystemExit(f"title write stayed queued: {failed}")
        seen = task_title(api("GET", f"/tasks?task_id={created_task}"))
        if seen != renamed:
            raise SystemExit(f"API title is {seen!r}")
        print(f"BOARD_READBACK task {created_task}")
        cli(db, "write", "task_comment", str(created_task), comment)
        failed = query(db, "SELECT status, last_error FROM mutation_queue WHERE kind='task_comment'")
        if failed:
            raise SystemExit(f"comment write stayed queued: {failed}")
        comments = api("GET", f"/comments?task_id={created_task}")
        if nonce not in json.dumps(comments):
            raise SystemExit("API comments do not contain the desktop comment")
        print(f"TASK_READBACK task {created_task}")
        rows = query(db, "SELECT id FROM Notification WHERE status='Normal' AND id>0 ORDER BY id LIMIT 1")
        if not rows:
            raise SystemExit("INBOX_EMPTY")
        notification_id = rows[0][0]
        cli(db, "write", "inbox_archive", str(notification_id))
        archived = True
        failed = query(db, "SELECT status, last_error FROM mutation_queue WHERE kind='inbox_archive'")
        if failed:
            raise SystemExit(f"inbox write stayed queued: {failed}")
        listing = api("GET", "/inbox/list")
        if contains_id(listing, notification_id):
            raise SystemExit(f"notification {notification_id} is still in the active inbox")
        print(f"INBOX_READBACK notification {notification_id}")
        restored = api("POST", "/inbox/unarchive", {"notification_ids": [notification_id]})
        archived = False
        if restored.get("unarchived_count") != 1:
            raise SystemExit(f"unarchive count {restored.get('unarchived_count')}")
        print("LIVE_READBACK_OK")
        code = 0
    finally:
        if archived and notification_id is not None:
            try:
                api("POST", "/inbox/unarchive", {"notification_ids": [notification_id]})
            except SystemExit as error:
                cleanup_error = error
                print(redact(str(error)), file=sys.stderr)
        if created_task is not None:
            try:
                api("POST", "/tasks/delete", {"task_id": created_task})
            except SystemExit as error:
                cleanup_error = error
                print(redact(str(error)), file=sys.stderr)
    if cleanup_error is not None:
        return 1
    return code


if __name__ == "__main__":
    sys.exit(main())
