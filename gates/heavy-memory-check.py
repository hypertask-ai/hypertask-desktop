"""Verify the heavy receipt and its unchanged pixels; never replace the demo gate."""
import argparse
import copy
import json
import sqlite3
import statistics
from html.parser import HTMLParser
from pathlib import Path
import subprocess
import sys


def load(path):
    return json.loads(path.read_text(encoding='utf-8-sig'))


def workload(folder):
    manifest = load(folder / 'heavy-manifest.json')
    for field, minimum in {'InboxItems': 640, 'BoardCards': 320, 'DescriptionBytes': 8_000_000,
                           'DescriptionImages': 960, 'DistinctAvatars': 160, 'Comments': 48,
                           'AiSessions': 40, 'AiMessages': 960, 'AiHistoryBytes': 1_000_000}.items():
        assert manifest[field] >= minimum, f'Heavy fixture too small: {field}'
    assert len(manifest['FixtureSHA256']) == 64
    return manifest



class Images(HTMLParser):
    def __init__(self):
        super().__init__()
        self.count = 0

    def handle_starttag(self, tag, attrs):
        if tag == 'img':
            attrs = dict(attrs)
            assert attrs['src'].startswith('http://127.0.0.1:8765/capture-')
            assert (attrs['width'], attrs['height']) == ('640', '360')
            self.count += 1


def database_facts(folder, manifest):
    db = sqlite3.connect(f'file:{(folder / "fixtures.db").resolve().as_posix()}?mode=ro', uri=True)
    try:
        tasks = [json.loads(row[0]) for row in db.execute('SELECT raw_json FROM Task')]
        inbox = [json.loads(row[0]) for row in db.execute('SELECT raw_json FROM Notification')]
        comments = db.execute('SELECT count(*) FROM Comment WHERE taskId=58006').fetchone()[0]
    finally:
        db.close()
    images = Images()
    for task in tasks:
        assert task['projectId'] == 15 and len(task['description'].encode()) >= 25_000
        images.feed(task['description'])
    facts = {'InboxItems': len(inbox), 'BoardCards': len(tasks), 'Comments': comments,
             'DescriptionBytes': sum(len(t['description'].encode()) for t in tasks),
             'DescriptionImages': images.count,
             'DistinctAvatars': len({n['fromUser']['photoURL'] for n in inbox})}
    assert all(manifest[key] == value for key, value in facts.items()), (facts, manifest)
    print('HEAVY_DATABASE_FACTS ' + json.dumps(facts, sort_keys=True))


def check(folder, budget=None):
    manifest = workload(folder)
    database_facts(folder, manifest)
    data = load(folder / 'memory.json')
    assert (data['DisplayWidth'], data['DisplayHeight']) == (1920, 1080)
    assert all((s['ClientWidth'], s['ClientHeight'], s['Dpi'], s['Maximized']) ==
               (1920, 1009, 96, True) for s in data['Samples'])
    for screen in ('inbox', 'board', 'task'):
        for run in range(1, 4):
            app = load(folder / f'{screen}-{run}-owners.app.json')
            store = load(folder / f'{screen}-{run}-owners.store.json')
            assert store['StoredTasks'] == manifest['BoardCards']
            assert store['StoredNotifications'] == manifest['InboxItems']
            assert store['StoredDescriptionBytes'] == manifest['DescriptionBytes']
            assert app['InboxItems'] == manifest['InboxItems']
            if screen != 'inbox':
                assert app['BoardCards'] == manifest['BoardCards']
            assert app['Ai']['HistorySessions'] == manifest['AiSessions']
            assert app['Ai']['HistoryMessages'] == manifest['AiMessages']
            assert app['Ai']['OpenChatMessages'] == 24, 'AI transcript never materialized'
            assert 0 < app['RustLiveRequestedBytes'] <= app['RustPeakRequestedBytes']
        owners = load(folder / f'{screen}-3-owners.json')
        regions = load(folder / f'{screen}-map.json')
        resident = {kind: sum(r['ResidentBytes'] for r in regions if r['Type'] == kind)
                    for kind in ('Private', 'Image', 'Mapped')}
        print(f'HEAVY_APP_OWNERS {screen}: {json.dumps(app, sort_keys=True)}')
        print(f'HEAVY_GPU_PAYLOAD {screen}: {json.dumps(owners, sort_keys=True)}')
        print(f'HEAVY_STORE_OWNERS {screen}: {json.dumps(store, sort_keys=True)}')
        print(f'HEAVY_RESIDENT_MAP {screen}: {json.dumps(resident, sort_keys=True)}')
        if budget:
            assert store['CompactTaskReads'] is True, 'Production did not enable compact reads'
            assert app['InboxRetainedTaskAttachments'] == 0, 'Inbox retained hidden attachment trees'
            assert app['BoardRetainedAttachments'] == 0, 'Board retained hidden attachment trees'
            if screen != 'inbox':
                assert app['BoardDescriptionBytes'] == 0, 'Cards retained hidden descriptions'
    if budget:
        assert manifest['Version'] == budget['FixtureVersion']
        if 'Fixture' in budget:
            assert manifest == budget['Fixture'], 'Heavy workload changed; rebaseline explicitly, never shrink it'
    if budget:
        check_limits(data, budget)
    return data


def check_limits(data, budget):
    for screen in ('inbox', 'board', 'task'):
        median = next(m for m in data['Medians'] if m['Screen'] == screen)
        assert 0 < budget[f'{screen}WorkingSet64'] < 100_000_000
        assert budget[f'{screen}WorkingSet64'] == min(99_000_000, budget[f'Baseline{screen.title()}WorkingSet64'] * 102 // 100)
        assert median['WorkingSet64'] <= budget[f'{screen}WorkingSet64'], f'{screen} exceeds heavy budget'
        samples = [s for s in data['Samples'] if s['Screen'] == screen]
        assert all(s['WarmRendererReceiptUpdated'] and s['WarmWorkingSet64'] > 0 for s in samples)
        assert statistics.median(s['WarmWorkingSet64'] for s in samples) <= budget[f'{screen}WorkingSet64'], f'{screen} warmed working set exceeds heavy budget'


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('folder', type=Path)
parser.add_argument('--budget', type=Path)
parser.add_argument('--compare', type=Path)
args = parser.parse_args()
budget = load(args.budget) if args.budget else None
base = Path(__file__).with_name('windows-memory-check.py')
command = [sys.executable, str(base), str(args.folder), '--owners']
if budget:
    command.append('--target')
if args.compare:
    command += ['--compare', str(args.compare), '--require-improvement']
subprocess.run(command, check=True)
data = check(args.folder, budget)
if args.compare:
    assert workload(args.compare) == workload(args.folder), 'Different heavy workloads'
    before = check(args.compare)
    assert before['ExeSHA256'] != data['ExeSHA256'], 'Same executable used for control and optimized build'
    for screen in ('inbox', 'board', 'task'):
        old = next(m['WorkingSet64'] for m in before['Medians'] if m['Screen'] == screen)
        new = next(m['WorkingSet64'] for m in data['Medians'] if m['Screen'] == screen)
        print(f'HEAVY_BEFORE_AFTER {screen}: {old/1e6:.3f} -> {new/1e6:.3f} MB')
        old_app = load(args.compare / f'{screen}-3-owners.app.json')
        new_app = load(args.folder / f'{screen}-3-owners.app.json')
        old_store = load(args.compare / f'{screen}-3-owners.store.json')
        assert old_store['CompactTaskReads'] is False
        assert old_app['InboxRetainedTaskAttachments'] > 0
        assert new_app['DataReads']['InboxReadNetRequestedBytes'] < old_app['DataReads']['InboxReadNetRequestedBytes']
        assert new_app['RustLiveRequestedBytes'] < old_app['RustLiveRequestedBytes']
        assert new_app['RustPeakRequestedBytes'] < old_app['RustPeakRequestedBytes']
        if screen != 'inbox':
            assert old_app['BoardRetainedAttachments'] > 0
            assert new_app['DataReads']['BoardReadNetRequestedBytes'] < old_app['DataReads']['BoardReadNetRequestedBytes']
            assert old_app['BoardDescriptionBytes'] == workload(args.compare)['DescriptionBytes']
            assert new_app['BoardDescriptionBytes'] == 0
            assert new_app['RustLiveRequestedBytes'] < old_app['RustLiveRequestedBytes']
            assert new_app['RustPeakRequestedBytes'] < old_app['RustPeakRequestedBytes']
        print(f"HEAVY_RUST_HEAP {screen}: live {old_app['RustLiveRequestedBytes']} -> {new_app['RustLiveRequestedBytes']}; peak {old_app['RustPeakRequestedBytes']} -> {new_app['RustPeakRequestedBytes']}")
# Negative controls run in CI against actual receipt data, not synthetic passing receipts.
manifest = load(args.folder / 'heavy-manifest.json')
for key in ('InboxItems', 'BoardCards', 'DescriptionBytes', 'DescriptionImages', 'DistinctAvatars', 'Comments', 'AiSessions', 'AiMessages', 'AiHistoryBytes'):
    invalid = copy.deepcopy(manifest)
    invalid[key] = 0
    # Exercise workload's exact predicate without writing or mutating the receipt.
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        folder = Path(tmp)
        (folder / 'heavy-manifest.json').write_text(json.dumps(invalid))
        try:
            workload(folder)
        except AssertionError as error:
            assert str(error) == f'Heavy fixture too small: {key}'
            print(f'HEAVY_UNDERSIZED_CONTROL_REJECTED {key}')
        else:
            raise AssertionError(f'Shrunken {key} accepted')
if budget:
    for screen in ('inbox', 'board', 'task'):
        invalid = copy.deepcopy(data)
        next(m for m in invalid['Medians'] if m['Screen'] == screen)['WorkingSet64'] = budget[f'{screen}WorkingSet64'] + 1
        try:
            check_limits(invalid, budget)
        except AssertionError as error:
            assert str(error) == f'{screen} exceeds heavy budget'
            print(f'HEAVY_OVER_BUDGET_CONTROL_REJECTED {screen}')
        else:
            raise AssertionError(f'Over-budget {screen} accepted')
        invalid = copy.deepcopy(data)
        for sample in invalid['Samples']:
            if sample['Screen'] == screen:
                sample['WarmWorkingSet64'] = budget[f'{screen}WorkingSet64'] + 1
        try:
            check_limits(invalid, budget)
        except AssertionError as error:
            assert str(error) == f'{screen} warmed working set exceeds heavy budget'
            print(f'HEAVY_WARM_OVER_BUDGET_CONTROL_REJECTED {screen}')
        else:
            raise AssertionError(f'Over-budget warm {screen} accepted')
print('HEAVY_MEMORY_RECEIPT_VERIFIED')
