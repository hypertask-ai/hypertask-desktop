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
failed = 0
for case in cases:
    name = case['id']
    try:
        result = results[name]
        assert result['passed'], result['error']
        before = json.loads((out / f'{name}-before.behavior.json').read_text(encoding='utf-8-sig'))
        after = json.loads((out / f'{name}-after.behavior.json').read_text(encoding='utf-8-sig'))
        assert before['state']['screen'] == case['screen'], 'wrong starting screen'
        for step in case['steps']:
            if step['op'] == 'expect':
                snapshot = json.loads((out / f"{name}-{step['name']}.behavior.json").read_text(encoding='utf-8-sig'))
                assert snapshot['state'].get(step['field']) == step['value'], step
        db = out / result['database']
        assert db.is_file(), 'missing isolated database'
        # Read-only CLI invocation: do not silently create a missing SQLite database.
        rows = json.loads(subprocess.check_output([
            'sqlite3', '-readonly', '-json', str(db),
            'SELECT id,kind,payload_json,status,attempts FROM mutation_queue ORDER BY id'
        ], text=True) or '[]')
        (out / f'{name}-queue.json').write_text(json.dumps(rows, indent=2) + '\n')
        mutation = case.get('mutation')
        if mutation:
            matches = [r for r in rows if r['kind'] == mutation['kind'] and all(
                json.loads(r['payload_json']).get(k) == v for k, v in mutation.items() if k != 'kind'
            )]
            assert len(matches) == 1, f'expected one {mutation}, queue={rows}'
            assert matches[0]['attempts'] == 0 and matches[0]['status'] == 'pending', matches
        else:
            assert not rows, f'navigation unexpectedly wrote mutations: {rows}'
        print(f'PASS {name}' + (f' sqlite id={matches[0]["id"]} kind={mutation["kind"]}' if mutation else ' layout effect verified'))
    except (AssertionError, KeyError, OSError, ValueError, subprocess.CalledProcessError) as error:
        failed += 1
        print(f'FAIL {name}: {error}')
print(f'BEHAVIOR {len(cases)-failed}/{len(cases)} PASS; {failed} FAIL')
sys.exit(bool(failed))
