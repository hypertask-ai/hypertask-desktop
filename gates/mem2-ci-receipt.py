"""Verify SHA-bound GitHub evidence only; never run application builds or tests."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

repo = 'hypertask-ai/hypertask-desktop'
sha = sys.argv[1]
env = {key: value for key, value in os.environ.items() if key not in ('GH_TOKEN', 'GITHUB_TOKEN')}


def gh(*args):
    return subprocess.check_output(['/usr/bin/gh', *args], env=env, text=True)


pr = json.loads(gh('api', f'repos/{repo}/pulls/19'))
assert pr['draft'] and pr['base']['ref'] == 'main' and pr['head']['ref'] == 'mem2'
assert pr['head']['sha'] == sha
subprocess.run(['git', 'merge-base', '--is-ancestor', 'origin/main', sha], check=True)
messages = subprocess.check_output(['git', 'log', f'origin/main..{sha}', '--format=%B%x00'], text=True)
assert all(message.strip().endswith('Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>')
           for message in messages.split('\0') if message.strip())
runs = json.loads(gh('api', f'repos/{repo}/actions/runs?branch=mem2&per_page=30'))['workflow_runs']
run = next(run for run in runs if run['head_sha'] == sha and run['name'] == 'CI')
assert run['status'] == 'completed' and run['conclusion'] == 'success', run['html_url']
jobs = json.loads(gh('run', 'view', str(run['id']), '-R', repo, '--json', 'jobs'))['jobs']
expected = {'ubuntu-24.04', 'windows-2025', 'macos-15', 'windows-arm64',
            'memory-eager-fonts-triple-buffer', 'Windows memory receipt and budget'}
assert {job['name'] for job in jobs} == expected
for job in jobs:
    assert job['conclusion'] == 'success', job['name']
    print(f"CI_JOB_SUCCESS {job['name']} {job['url']}")
logs = gh('run', 'view', str(run['id']), '-R', repo, '--log')
proof = next(job for job in jobs if job['name'] == 'Windows memory receipt and budget')
logs += gh('api', f'repos/{repo}/actions/jobs/{proof["databaseId"]}/logs')
for test in (
    'theme_fonts_load_before_use_and_repeated_switches_do_not_reload',
    'memory_receipt_observes_sqlite_without_changing_cached_tasks',
    'bounded_sync_cache_evicts_pages_without_losing_persisted_offline_content',
    'fidelity_upgrades_list_only_database_and_persists_authors_summary_and_drafts',
    'synced_fidelity_fixture_renders_real_authors_saved_summary_and_web_draft_markers',
    'creator_prefers_the_agent_not_the_agents_owner_and_handles_missing_agent_names',
    'optional_ai_summary_survives_partial_updates_but_detail_absence_clears_it',
    'draft_emptiness_matches_use_has_drafts_and_wrong_task_is_rejected',
):
    for platform in ('ubuntu-24.04', 'windows-2025', 'macos-15'):
        assert any(line.startswith(platform + '\t') and test in line and ' ... ok' in line
                   for line in logs.splitlines()), (platform, test)
    print(f'CI_FIXTURE_PROVEN {test}')
for screen in ('inbox', 'board', 'task'):
    assert f'MEMORY_VISUAL_IDENTICAL {screen}' in logs
for screen in ('inbox', 'board', 'task'):
    assert f'MEMORY_OVER_BUDGET_CONTROL_REJECTED {screen}' in logs
with tempfile.TemporaryDirectory(prefix='mem2-ci-evidence-') as temp:
    receipts = []
    for artifact in (f'windows-memory-{sha}', 'windows-memory'):
        folder = Path(temp) / artifact
        gh('run', 'download', str(run['id']), '-R', repo, '-n', artifact, '-D', str(folder))
        data = json.loads((folder / 'memory.json').read_text(encoding='utf-8-sig'))
        assert data['Revision'] == sha
        assert len(data['Samples']) == 9 and all(sample['Maximized'] for sample in data['Samples'])
        receipts.append(data)
    before, after = receipts
    assert before['DatabaseSHA256'] == after['DatabaseSHA256']
    for screen in ('inbox', 'board', 'task'):
        old_folder = Path(temp) / f'windows-memory-{sha}'
        new_folder = Path(temp) / 'windows-memory'
        old_gpu = json.loads((old_folder / f'{screen}-3-owners.json').read_text())
        new_gpu = json.loads((new_folder / f'{screen}-3-owners.json').read_text())
        assert old_gpu['SwapChainBytes'] * 2 == new_gpu['SwapChainBytes'] * 3
        old_fonts = json.loads((old_folder / f'{screen}-3-owners.fonts.json').read_text())
        new_fonts = json.loads((new_folder / f'{screen}-3-owners.fonts.json').read_text())
        assert old_fonts['RegisteredFontFaces'] == 10 and new_fonts['RegisteredFontFaces'] == 5
        assert old_fonts['RegisteredFontBytes'] > new_fonts['RegisteredFontBytes']
        old_store = json.loads((old_folder / f'{screen}-3-owners.store.json').read_text())
        new_store = json.loads((new_folder / f'{screen}-3-owners.store.json').read_text())
        assert new_store['SqliteCacheBytes'] < old_store['SqliteCacheBytes']
    for old, new in zip(before['Medians'], after['Medians'], strict=True):
        assert old['Screen'] == new['Screen']
        assert new['WorkingSet64'] < 100_000_000
        assert new['WorkingSet64'] < old['WorkingSet64'], (old, new)
        print(f"CI_MEMORY_WIN {new['Screen']} {old['WorkingSet64']/1e6:.6f} -> {new['WorkingSet64']/1e6:.6f} MB")
print(f"MEM2_CI_PROVEN {sha} {run['html_url']}")
