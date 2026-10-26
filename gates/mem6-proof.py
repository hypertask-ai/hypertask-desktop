"""CI-only allocation A/B: no eviction, identical pixels/workload, lower commit."""
import json
from pathlib import Path
import statistics
import subprocess
import sys
import xml.etree.ElementTree as ET
from PIL import Image, ImageChops


def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def reduction(new, old):
    assert 0 < new < old, 'Allocation optimization must reduce memory'


current, control = map(Path, sys.argv[1:])
subprocess.run([sys.executable, str(Path(__file__).with_name('windows-memory-check.py')),
                str(current), '--owners', '--compare', str(control), '--require-improvement'], check=True)
a, b = load(current / 'memory.json'), load(control / 'memory.json')
assert a['Revision'] == b['Revision'] == a['BuildRevision'] == b['BuildRevision']
assert a['ExeSHA256'] != b['ExeSHA256']
assert a['SuggestionSeed'] == b['SuggestionSeed'] == 7381
heap_tag = '{http://schemas.microsoft.com/SMI/2020/WindowsSettings}heapType'
assert ET.fromstring(a['ExecutableManifest']).find('.//' + heap_tag).text == 'SegmentHeap'
assert a['ExecutableManifest'] == b['ExecutableManifest'], 'Heap policy must not change'
arena_reduced = False
for screen in ('inbox', 'board', 'task'):
    new = [r for r in a['Samples'] if r['Screen'] == screen]
    old = [r for r in b['Samples'] if r['Screen'] == screen]
    assert all(r['WarmRendererReceiptUpdated'] is True for r in new + old)
    assert all((r['WarmClientWidth'], r['WarmClientHeight']) == (r['ClientWidth'], r['ClientHeight']) for r in new + old), 'Warm geometry must match the cold capture'
    assert all(0 < r['WarmRestoreMaximizeMilliseconds'] < 10000 for r in new + old)
    for folder in (current, control):
        assert not list(folder.glob('*-owners.startup.json')), 'Working-set eviction is forbidden'
    for key in ('WorkingSet64', 'PrivateMemorySize64', 'WarmWorkingSet64', 'WarmPrivateMemorySize64'):
        before = statistics.median(r[key] for r in old)
        after = statistics.median(r[key] for r in new)
        reduction(after, before)
        if 'WorkingSet' in key:
            assert after < 100_000_000
        try:
            reduction(before, before)
        except AssertionError:
            print(f'MEM6_UNIMPROVED_CONTROL_REJECTED {screen} {key}')
        else:
            raise AssertionError('Unchanged memory accepted')
        print(f'MEM6_MEDIAN {screen} {key}: {before} -> {after} bytes')
    for run in range(1, 4):
        gpu_new = load(current / f'{screen}-{run}-owners.json')
        gpu_old = load(control / f'{screen}-{run}-owners.json')
        assert gpu_new == gpu_old, f'{screen}: GPU resource capacity changed'
        assert load(current / f'{screen}-{run}-owners.fonts.json') == load(control / f'{screen}-{run}-owners.fonts.json')
        store_new = load(current / f'{screen}-{run}-owners.store.json')
        store_old = load(control / f'{screen}-{run}-owners.store.json')
        assert store_new.pop('PackedInboxMetadata') is True
        assert store_old.pop('PackedInboxMetadata') is False
        cache_new = store_new.pop('SqliteCacheBytes')
        cache_old = store_old.pop('SqliteCacheBytes')
        assert 0 < cache_new <= cache_old, f'{screen}: SQLite cache grew'
        print(f'MEM6_SQLITE_CACHE {screen} run={run}: {cache_old} -> {cache_new} bytes')
        assert store_new == store_old, f'{screen}: SQLite/store ownership changed'
        app_new = load(current / f'{screen}-{run}-owners.app.json')
        app_old = load(control / f'{screen}-{run}-owners.app.json')
        for key in app_old.keys() - {'RustLiveRequestedBytes', 'RustPeakRequestedBytes', 'DataReads', 'ShapedLineBytes', 'ElementArenaCapacityBytes'}:
            assert app_new[key] == app_old[key], f'{screen}: retained {key} changed'
        assert app_new['ElementArenaCapacityBytes'] <= app_old['ElementArenaCapacityBytes']
        arena_reduced |= app_new['ElementArenaCapacityBytes'] < app_old['ElementArenaCapacityBytes']
        print(f'MEM6_ARENA {screen} run={run}: {app_old["ElementArenaCapacityBytes"]} -> {app_new["ElementArenaCapacityBytes"]} bytes')
        reduction(app_new['ShapedLineBytes'], app_old['ShapedLineBytes'])
        reduction(app_new['RustLiveRequestedBytes'], app_old['RustLiveRequestedBytes'])
        assert app_new['DataReads']['InboxReadNetRequestedBytes'] < app_old['DataReads']['InboxReadNetRequestedBytes']
        print(f'MEM6_INBOX_OWNERS {screen} run={run}: {app_old["DataReads"]["InboxReadNetRequestedBytes"]} -> {app_new["DataReads"]["InboxReadNetRequestedBytes"]} requested bytes')
    with Image.open(control / f'{screen}-warm.png') as before, Image.open(current / f'{screen}-warm.png') as after:
        assert before.size == after.size
        assert ImageChops.difference(before.convert('RGB'), after.convert('RGB')).getbbox() is None, f'{screen}: warm pixels changed'
    print(f'MEM6_WARM_VISUAL_IDENTICAL {screen}')
    layout = load(current / f'{screen}-3.json')
    print(f'MEM6_LAYOUT_COUNTS {screen}: elements={len(layout["elements"])} missing={len(layout["missing"])}')
    for label, folder in [('before', control), ('after', current)]:
        regions = load(folder / f'{screen}-warm-inspection.json')['Regions']
        totals = {kind: sum(r['ResidentBytes'] for r in regions if r['Type'] == kind)
                  for kind in ('Private', 'Image', 'Mapped')}
        print(f'MEM6_RESIDENT {screen} {label}: {json.dumps(totals, sort_keys=True)}')
if not (current / 'heavy-manifest.json').exists():
    assert arena_reduced, 'The demo must release actual unused arena capacity'
    print('MEM6_UNUSED_ARENA_CAPACITY_RELEASED')
print('MEM6_ALLOCATION_REDUCTION_PROVEN')
