"""Inspect SHA-bound GitHub CI output only; never build, test, or launch an app locally."""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REPO = "hypertask-ai/hypertask-desktop"
ENV = {k: v for k, v in os.environ.items() if k not in ("GH_TOKEN", "GITHUB_TOKEN")}
GROUPS = {
    "presentation": [
        "web_resize_bounds_and_persisted_pin_suppression_and_mode",
        "pin_close_modes_preserve_conversation_and_draft",
        "ai_resize_mouse_drag_clamps_and_persists_after_reopen",
        "ai_header_pin_modes_close_and_commands_are_real_mouse_paths",
        "incoming_stream_keeps_user_scroll_position_until_latest_is_clicked",
        "scope_matches_web_board_task_and_inbox_rules",
        "suggestions_use_task_reference_and_real_board_counts",
    ],
    "contract": [
        "http_stream_and_stop_use_real_endpoints_headers_and_ids",
        "http_refusals_and_eof_are_visible_errors_not_fake_replies",
        "request_matches_web_contract_and_cancel_identity",
        "recorded_spec_frames_survive_every_chunk_boundary_and_crlf",
        "history_accepts_all_1517_transcripts_beyond_ureq_json_limit",
        "history_is_lazy_background_results_are_polled_without_an_active_turn",
        "late_stream_updates_stay_with_the_original_chat_and_preserve_errors",
        "quoted_comment_preserves_attribution_in_request_and_inserts_at_cursor",
        "response_copy_mouse_action_copies_rendered_unicode_text",
    ],
    "comments": [
        "focused_comment_copy_link_and_quote_target_the_selected_comment",
        "overflow_ellipsis_click_sets_exact_comment_before_opening_command_palette",
        "timestamp_tooltip_uses_web_local_exact_date_not_relative_text",
        "comment_drafts_restore_per_task_and_do_not_leak_between_users",
        "comment_autosave_debounces_and_freezes_task_identity_before_navigation",
        "draft_queue_uses_bearer_routes_exact_body_and_coalesces_unattempted_saves",
        "server_draft_restore_is_scoped_and_never_overwrites_a_newer_local_edit",
        "failed_comment_preserves_draft_and_success_clears_only_after_acknowledgement",
        "sending_older_comment_never_clears_a_new_draft_or_replays_old_draft_over_it",
        "drafts_survive_restart_and_rejected_saves_are_not_reported_as_success",
        "old_account_drafts_never_write_with_a_different_current_user",
    ],
}


def gh(*args):
    return subprocess.check_output(["/usr/bin/gh", *args], env=ENV, text=True, cwd=ROOT)


sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
group = sys.argv[1] if len(sys.argv) > 1 else "all"
assert group == "all" or group in GROUPS, group
runs = json.loads(gh("api", f"repos/{REPO}/actions/runs?branch=aiside&per_page=50"))["workflow_runs"]
run = next(r for r in runs if r["head_sha"] == sha and r["name"] == "CI")
assert run["status"] == "completed" and run["conclusion"] == "success", run["html_url"]
jobs = json.loads(gh("run", "view", str(run["id"]), "-R", REPO, "--json", "jobs"))["jobs"]
for name in ["ubuntu-24.04", "windows-2025", "macos-15", "windows-arm64"]:
    job = next(j for j in jobs if j["name"] == name)
    assert job["conclusion"] == "success", job
    print(f"CI PASS {name}")
linux = next(j for j in jobs if j["name"] == "ubuntu-24.04")
assert next(s for s in linux["steps"] if s["name"] == "Layout dump contract")["conclusion"] == "success"
logs = gh("api", f"repos/{REPO}/actions/jobs/{linux['databaseId']}/logs")
for test in sum((names for key, names in GROUPS.items() if group in ("all", key)), []):
    assert any(test + " ... ok" in line for line in logs.splitlines()), test
    print(f"CI TEST PASS {test}")
for theme in ["porcelain", "graphite", "amoled", "dia"]:
    for screen in ["inbox", "board", "task"]:
        marker = f"LAYOUT {screen}/{theme}: covered="
        assert any(marker in line and "missing=[]" in line for line in logs.splitlines()), marker
print(f"AI_CI_{group.upper()}_PROOF {sha} {run['html_url']}")
