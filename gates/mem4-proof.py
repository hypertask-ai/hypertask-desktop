"""CI-only A/B: residency (not allocation) savings survive repaint and 30 s idle."""
import json
from pathlib import Path
import statistics
import subprocess
import sys
from PIL import Image, ImageChops


def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def check_warm(new, old):
    assert new < old * 0.9, 'Warm residency must improve by at least 10%'


current, control = map(Path, sys.argv[1:])
subprocess.run([sys.executable, str(Path(__file__).with_name('windows-memory-check.py')),
                str(current), '--owners', '--compare', str(control), '--require-improvement'], check=True)
a, b = load(current / 'memory.json'), load(control / 'memory.json')
assert a['ExeSHA256'] != b['ExeSHA256']
for screen in ('inbox', 'board', 'task'):
    new = [r for r in a['Samples'] if r['Screen'] == screen]
    old = [r for r in b['Samples'] if r['Screen'] == screen]
    for run in range(1, 4):
        trim = load(current / f'{screen}-{run}-owners.startup.json')
        assert trim == {'StartupTrimSucceeded': True, 'DelaySeconds': 5, 'Repeats': False}
        assert not (control / f'{screen}-{run}-owners.startup.json').exists()
    warm_new = statistics.median(r['WarmWorkingSet64'] for r in new)
    warm_old = statistics.median(r['WarmWorkingSet64'] for r in old)
    check_warm(warm_new, warm_old)
    try:
        check_warm(warm_old, warm_old)
    except AssertionError as error:
        assert str(error) == 'Warm residency must improve by at least 10%'
        print(f'MEM4_UNIMPROVED_WARM_CONTROL_REJECTED {screen}')
    else:
        raise AssertionError('Warm gate accepted an unchanged working set')
    assert all(0 <= r['WarmRenderMilliseconds'] < 5000 for r in new + old)
    print(f'MEM4_WARM_RENDER_MS {screen}: control={statistics.median(r["WarmRenderMilliseconds"] for r in old):.3f} optimized={statistics.median(r["WarmRenderMilliseconds"] for r in new):.3f}')
    assert statistics.median(r['WarmPrivateMemorySize64'] for r in new) <= statistics.median(r['WarmPrivateMemorySize64'] for r in old) * 1.05
    # No shrinking textures, altering buffers or fonts to pass the residency gate.
    for suffix in ('', '.fonts'):
        assert load(current / f'{screen}-3-owners{suffix}.json') == load(control / f'{screen}-3-owners{suffix}.json')
    with Image.open(current / f'{screen}.png') as early, Image.open(current / f'{screen}-warm.png') as warm:
        assert early.size == warm.size
        assert ImageChops.difference(early.convert('RGB'), warm.convert('RGB')).getbbox() is None
    for label, folder in [('before', control), ('after', current)]:
        snapshot = load(folder / f'{screen}-inspection.json')
        totals = {key: sum(r[key] for r in snapshot['Regions']) for key in
                  ('ResidentBytes', 'PrivateResidentBytes', 'ShareableResidentBytes', 'SharedResidentBytes')}
        assert totals['ResidentBytes'] == totals['PrivateResidentBytes'] + totals['ShareableResidentBytes']
        assert totals['SharedResidentBytes'] <= totals['ShareableResidentBytes']
        print(f'MEM4_RESIDENT {screen} {label}: {json.dumps(totals, sort_keys=True)}')
        print(f'MEM4_GPU_COUNTERS {screen} {label}: {json.dumps(snapshot["Gpu"])}')
    print(f'MEM4_WARM_RESIDENCY {screen}: {warm_old} -> {warm_new} bytes; repaint pixels identical')
print('MEM4_STARTUP_RESIDENCY_PROVEN')
