"""Read GitHub's SHA-bound jobs, raw logs and artifacts; no local build/test/app execution."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

REPO = 'hypertask-ai/hypertask-desktop'
ENV = {k: v for k, v in os.environ.items() if k not in ('GH_TOKEN', 'GITHUB_TOKEN')}
sha = sys.argv[1]


def gh(*args):
    return subprocess.check_output(['/usr/bin/gh', *args], env=ENV, text=True)


def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


pr = json.loads(gh('api', f'repos/{REPO}/pulls/20'))
assert pr['draft'] and pr['base']['ref'] == 'main' and pr['head']['ref'] == 'mem3'
assert pr['head']['sha'] == sha
subprocess.run(['git', 'merge-base', '--is-ancestor', 'origin/main', sha], check=True)
messages = subprocess.check_output(['git', 'log', f'origin/main..{sha}', '--format=%B%x00'], text=True)
assert all(m.strip().endswith('Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>')
           for m in messages.split('\0') if m.strip())
runs = json.loads(gh('api', f'repos/{REPO}/actions/runs?branch=mem3&per_page=30'))['workflow_runs']
run = next(r for r in runs if r['head_sha'] == sha and r['name'] == 'CI')
assert run['status'] == 'completed' and run['conclusion'] == 'success', run['html_url']
jobs = json.loads(gh('run', 'view', str(run['id']), '-R', REPO, '--json', 'jobs'))['jobs']
expected = {'ubuntu-24.04', 'windows-2025', 'macos-15', 'windows-arm64',
            'memory-heavy-full-task-reads', 'Windows memory receipt and budget'}
assert {j['name'] for j in jobs} == expected
for job in jobs:
    assert job['conclusion'] == 'success', job['name']
    print(f"CI_JOB_SUCCESS {job['name']} {job['url']}")
    if job['name'] in ('ubuntu-24.04', 'windows-2025', 'macos-15'):
        log = gh('api', f'repos/{REPO}/actions/jobs/{job["databaseId"]}/logs')
        for test in ('compact_cards_preserve_every_display_field_and_full_detail_search_and_store',
                     'compact_board_keeps_duplicate_legacy_section_membership_and_order',
                     'compact_inbox_uses_current_title_and_ticket_without_materializing_description'):
            assert any(test + ' ... ok' in line for line in log.splitlines()), (job['name'], test)
            print(f"CI_TEST_PROVEN {job['name']} {test}")
proof = next(j for j in jobs if j['name'] == 'Windows memory receipt and budget')
log = gh('api', f'repos/{REPO}/actions/jobs/{proof["databaseId"]}/logs')
for screen in ('inbox', 'board', 'task'):
    for marker in ('MEMORY_VISUAL_IDENTICAL', 'MEMORY_OVER_BUDGET_CONTROL_REJECTED',
                   'HEAVY_OVER_BUDGET_CONTROL_REJECTED', 'HEAVY_BEFORE_AFTER', 'HEAVY_APP_OWNERS',
                   'HEAVY_RUST_HEAP', 'HEAVY_GPU_PAYLOAD', 'HEAVY_STORE_OWNERS', 'HEAVY_RESIDENT_MAP'):
        assert f'{marker} {screen}' in log, (marker, screen)
for field in ('InboxItems', 'BoardCards', 'DescriptionBytes', 'DescriptionImages', 'DistinctAvatars',
              'Comments', 'AiSessions', 'AiMessages', 'AiHistoryBytes'):
    assert f'HEAVY_UNDERSIZED_CONTROL_REJECTED {field}' in log
assert 'HEAVY_MEMORY_RECEIPT_VERIFIED' in log
with tempfile.TemporaryDirectory(prefix='mem3-github-evidence-') as tmp:
    folders = []
    for artifact in ('windows-memory-heavy-baseline', 'windows-memory-heavy', 'windows-memory'):
        folder = Path(tmp) / artifact
        gh('run', 'download', str(run['id']), '-R', REPO, '-n', artifact, '-D', str(folder))
        assert load(folder / 'memory.json')['Revision'] == sha
        folders.append(folder)
    old_folder, new_folder, demo_folder = folders
    old, new = load(old_folder / 'memory.json'), load(new_folder / 'memory.json')
    assert old['DatabaseSHA256'] == new['DatabaseSHA256']
    manifest = load(new_folder / 'heavy-manifest.json')
    assert manifest == load(old_folder / 'heavy-manifest.json')
    budget = load(Path('gates/windows-memory-heavy-budget.json'))
    assert manifest == budget['Fixture'], 'Budget must pin the measured fixture, not just lower bounds'
    for screen in ('inbox', 'board', 'task'):
        before = next(m['WorkingSet64'] for m in old['Medians'] if m['Screen'] == screen)
        after = next(m['WorkingSet64'] for m in new['Medians'] if m['Screen'] == screen)
        assert after < before and after <= budget[f'{screen}WorkingSet64'] < 100_000_000
        print(f'CI_HEAVY_MEMORY_WIN {screen} {before/1e6:.3f} -> {after/1e6:.3f} MB')
    print('CI_HEAVY_FIXTURE ' + json.dumps(manifest, sort_keys=True))
print(f"MEM3_CI_PROVEN {sha} {run['html_url']}")
