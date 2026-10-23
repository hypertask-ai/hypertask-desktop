"""Read GitHub's SHA-bound checks and unit-test logs; never build or test on the VPS."""
import json
import os
import subprocess
import sys

env = {k: v for k, v in os.environ.items() if k not in ('GH_TOKEN', 'GITHUB_TOKEN')}
repo = 'hypertask-ai/hypertask-desktop'
sha = sys.argv[1]
def gh(*args):
    return subprocess.check_output(['/usr/bin/gh', *args], env=env, text=True)
runs = json.loads(gh('api', f'repos/{repo}/actions/runs?branch=keys&per_page=30'))['workflow_runs']
run = next(r for r in runs if r['head_sha'] == sha and r['name'] == 'CI')
assert run['status'] == 'completed' and run['conclusion'] == 'success', run['html_url']
jobs = json.loads(gh('run', 'view', str(run['id']), '-R', repo, '--json', 'jobs'))['jobs']
for name in ['ubuntu-24.04', 'windows-2025', 'macos-15', 'windows-arm64']:
    job = next(j for j in jobs if j['name'] == name)
    assert job['conclusion'] == 'success', job
    print(f'CI PASS {name}')
logs = gh('run', 'view', str(run['id']), '-R', repo, '--log')
for name in [
    'shortcuts::tests::complete_supported_key_map_and_contexts',
    'shortcuts::tests::windows_shifted_punctuation_retains_physical_chord',
    'command_center::tests::filter_matches_web_tokens_keywords_typos_and_stable_order',
    'command_center::tests::no_task_commands_without_target_and_no_hidden_noops',
    'command_center::tests::usage_matches_web_decay_and_never_duplicates_frequent_results',
    'catalogs_back_durable_label_and_assignment_writes_and_cached_search',
    'typed_patches_are_optimistic_durable_and_use_authenticated_task_update',
]:
    assert any(name in line and '... ok' in line for line in logs.splitlines()), name
    print(f'CI UNIT PASS {name}')
print(f'CI_GREEN_UNIT_PROOF {sha} {run["html_url"]}')
