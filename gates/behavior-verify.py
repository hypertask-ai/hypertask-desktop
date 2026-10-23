"""Verify real laptop input evidence; a missing case/database is always a failure."""
import json
from pathlib import Path
import subprocess
import sys

cases = json.loads(Path(sys.argv[1]).read_text())
out = Path(sys.argv[2])
results = {}
for file in sorted(out.glob('results-*.json')):
    batch = json.loads(file.read_text(encoding='utf-8-sig'))
    for result in batch if isinstance(batch, list) else [batch]:
        assert result['id'] not in results, 'duplicate case result'
        results[result['id']] = result
def load(file):
    return json.loads(file.read_text(encoding='utf-8-sig'))

def expected(value, state, before):
    if not isinstance(value, str) or not value.startswith('$'):
        return value
    return {'$last': state.get('last_index'), '$count': state.get('selection_count'),
            '$all': state.get('inbox_count'), '$composer': state.get('composer_index'),
            '$before.task_id': before.get('task_id')}[value]

def mutation_value(value, state):
    if value == '$column-1': return state['column_ids'][1]
    if value == '$today':
        # Windows captures the selected UTC date; validate the queued midnight below.
        return value
    if value == '$assigned':
        ids = [m['id'] for m in state.get('task_assignees') or []]
        first = state['first_assignee']
        assert first is not None, 'No cached member to exercise'
        return [i for i in ids if i != first] if first in ids else ids + [first]
    if value == '$labels':
        ids = [l['id'] for l in state.get('task_labels') or []]
        first = state['first_label']
        assert first is not None, 'No cached tag to exercise'
        return [i for i in ids if i != first] if first in ids else ids + [first]
    return value

def mutation_matches(row, mutation, state):
    if row['kind'] != mutation['kind']: return False
    payload = json.loads(row['payload_json'])
    for key, value in mutation.items():
        if key == 'kind': continue
        want = mutation_value(value, state)
        actual = payload.get(key)
        if want == '$today':
            from datetime import datetime, timezone
            date = datetime.fromisoformat(actual.replace('Z','+00:00'))
            assert date.hour == date.minute == date.second == 0
            assert date.date() == datetime.now(timezone.utc).date()
        elif actual != want: return False
    return True

failed = 0
for case in cases:
    name = case['id']
    try:
        result = results[name]
        assert result['passed'], result['error']
        before = json.loads((out / f'{name}-before.behavior.json').read_text(encoding='utf-8-sig'))
        after = json.loads((out / f'{name}-after.behavior.json').read_text(encoding='utf-8-sig'))
        assert before['state']['screen'] == case['screen'], 'wrong starting screen'
        if case['screen'] == 'task':
            import re
            elements = {e['key']:e for e in before['elements']}
            comments = [e for k,e in elements.items() if re.fullmatch(r'task\.comment\.[0-9]+',k)]
            composer = elements['task.composer']
            assert all(composer['y'] >= e['y'] + e['h'] - 0.01 for e in comments), 'composer precedes comments'
        footer = {
            'inbox':['CTRL K command center','J down','K up','Enter open task','TAB cycle inbox splits','E Archive','CTRL E Mark task as done'],
            'board':['CTRL K command center','J down','K up','TAB next column','C add task','CTRL E archive','CTRL B Change Board','/ search'],
            'task':['CTRL K command center','A assign','M change status(move)','P priority','S task size','T tags','CTRL E archive','ESC back','TAB cycle'],
        }
        assert before['state']['footer'] == footer[case['screen']], 'footer differs from web ConstantsQuickTips.ts' 
        for step in case['steps']:
            if step['op'] == 'expect':
                snapshot = json.loads((out / f"{name}-{step['name']}.behavior.json").read_text(encoding='utf-8-sig'))
                assert snapshot['state'].get(step['field']) == expected(step['value'], snapshot['state'], before['state']), step
            elif step['op'] == 'expect-change':
                snapshot = load(out / f"{name}-{step['name']}.behavior.json")
                assert snapshot['state'][step['field']] != before['state'][step['field']], step
        db = out / result['database']
        assert db.is_file(), 'missing isolated database'
        # Read-only CLI invocation: do not silently create a missing SQLite database.
        rows = json.loads(subprocess.check_output([
            'sqlite3', '-readonly', '-json', str(db),
            'SELECT id,kind,payload_json,status,attempts FROM mutation_queue ORDER BY id'
        ], text=True) or '[]')
        (out / f'{name}-queue.json').write_text(json.dumps(rows, indent=2) + '\n')
        mutations = [case[k] for k in ['mutation', 'also_mutation'] if k in case] + case.get('also_mutations', [])
        for mutation in mutations:
            matches = [r for r in rows if mutation_matches(r, mutation, before['state'])]
            assert len(matches) == 1, f'expected one {mutation}, queue={rows}'
            assert matches[0]['attempts'] == 0 and matches[0]['status'] == 'pending', matches
        if not mutations:
            assert not rows, f'navigation unexpectedly wrote mutations: {rows}'
        else:
            assert len(rows) == len(mutations), f'unexpected extra or missing writes: {rows}'
        if case.get('clipboard'):
            raw = json.loads(subprocess.check_output(['sqlite3','-readonly',str(db),f"SELECT raw_json FROM Task WHERE id={int(before['state']['task_id'])}"],text=True))
            ticket = raw.get('ticketNumber') or str(raw['id'])
            url = f"https://app.hypertask.ai/detail/project-{raw['projectId']}/{raw['uniqueIndex']}"
            want = {'id':ticket, 'title-id':f"{ticket.upper()} | {raw['title']}", 'url':url, 'title-url':f"{ticket.upper()} | {raw['title']} | {url}"}[case['clipboard']]
            assert (out / f'{name}-clipboard.txt').read_text(encoding='utf-8-sig') == want, 'clipboard differs from cached task'
        print(f'PASS {name}' + (' sqlite mutation verified' if mutations else ' clipboard verified' if case.get('clipboard') else ' layout effect verified'))
    except (AssertionError, KeyError, OSError, ValueError, subprocess.CalledProcessError) as error:
        failed += 1
        print(f'FAIL {name}: {error}')
print(f'BEHAVIOR {len(cases)-failed}/{len(cases)} PASS; {failed} FAIL')
sys.exit(bool(failed))
