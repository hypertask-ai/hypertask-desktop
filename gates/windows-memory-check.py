"""Validate CI receipts, the frozen budget, and optional unchanged-screen proof."""
import argparse
import json
from pathlib import Path
import statistics


def receipt(folder):
    data = json.loads((folder / 'memory.json').read_text(encoding='utf-8-sig'))
    assert len(data['Revision']) == 40
    assert data['WaitSeconds'] == 8
    assert len(data['Samples']) == 9
    assert len({s['Pid'] for s in data['Samples']}) == 9
    for screen in ('inbox', 'board', 'task'):
        samples = [s for s in data['Samples'] if s['Screen'] == screen]
        assert sorted(s['Run'] for s in samples) == [1, 2, 3]
        assert all(s['Maximized'] and s['ClientWidth'] > 0 and s['ClientHeight'] > 0 for s in samples)
        assert len({(s['ClientWidth'], s['ClientHeight'], s['Dpi']) for s in samples}) == 1
        median = next(m for m in data['Medians'] if m['Screen'] == screen)
        for key in ('WorkingSet64', 'PrivateMemorySize64'):
            assert median[key] == statistics.median(s[key] for s in samples)
            assert median[key] > 0
        layout = json.loads((folder / f'{screen}-3.json').read_text(encoding='utf-8-sig'))
        assert layout['screen'] == screen and layout['theme'] == 'graphite'
        assert layout['elements'], 'Must measure a rendered, populated screen'
        print(f"{data['Revision']} {screen}: WS={median['WorkingSet64']/1e6:.3f} MB private={median['PrivateMemorySize64']/1e6:.3f} MB")
    return data


def check_budget(data, budget):
    assert budget['InboxWorkingSet64'] == budget['BaselineInboxWorkingSet64'] * 105 // 100
    inbox = next(m for m in data['Medians'] if m['Screen'] == 'inbox')
    assert inbox['WorkingSet64'] <= budget['InboxWorkingSet64'], 'Inbox exceeds measured baseline + 5%'


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('folder', type=Path)
parser.add_argument('--budget', type=Path)
parser.add_argument('--compare', type=Path)
parser.add_argument('--target', action='store_true')
parser.add_argument('--owners', action='store_true')
args = parser.parse_args()
data = receipt(args.folder)
if args.owners:
    for screen in ('inbox', 'board', 'task'):
        owners = json.loads((args.folder / f'{screen}-3-owners.json').read_text())
        assert all(isinstance(value, int) and value >= 0 for value in owners.values())
        assert owners['FontAtlasBytes'] > 0 and owners['InstanceBufferBytes'] > 0
        assert owners['SwapChainBytes'] > 0
        store = json.loads((args.folder / f'{screen}-3-owners.store.json').read_text())
        assert store['SqliteCacheBytes'] > 0 and store['SqliteSchemaBytes'] > 0
        print(f"MEMORY_RESOURCE_PAYLOAD {screen}: {json.dumps(owners, sort_keys=True)}")
        print(f"MEMORY_SQLITE_OWNERS {screen}: {json.dumps(store, sort_keys=True)}")
if args.budget:
    budget = json.loads(args.budget.read_text())
    check_budget(data, budget)
    # Exercise exactly the enforcement oracle with a real receipt over its limit.
    over = json.loads(json.dumps(data))
    next(m for m in over['Medians'] if m['Screen'] == 'inbox')['WorkingSet64'] = budget['InboxWorkingSet64'] + 1
    try:
        check_budget(over, budget)
    except AssertionError as error:
        assert str(error) == 'Inbox exceeds measured baseline + 5%'
        print('MEMORY_OVER_BUDGET_CONTROL_REJECTED')
    else:
        raise AssertionError('Budget accepted an over-limit receipt')
if args.target:
    assert all(m['WorkingSet64'] < 100_000_000 for m in data['Medians']), '100 MB target not reached'
if args.compare:
    from PIL import Image, ImageChops
    before = receipt(args.compare)
    assert (before['DisplayWidth'], before['DisplayHeight']) == (data['DisplayWidth'], data['DisplayHeight'])
    assert before['DatabaseSHA256'] == data['DatabaseSHA256'], 'Fixture database changed'
    for screen in ('inbox', 'board', 'task'):
        old_layout = json.loads((args.compare / f'{screen}-3.json').read_text(encoding='utf-8-sig'))
        new_layout = json.loads((args.folder / f'{screen}-3.json').read_text(encoding='utf-8-sig'))
        assert old_layout == new_layout, f'{screen} layout changed'
        with Image.open(args.compare / f'{screen}.png') as old, Image.open(args.folder / f'{screen}.png') as new:
            old, new = old.convert('RGB'), new.convert('RGB')
            assert old.size == new.size
            assert len(old.getcolors(old.width * old.height)) > 20, 'Blank capture'
            bounds = ImageChops.difference(old, new).getbbox()
            assert bounds is None, f'{screen} pixels changed in {bounds}'
        print(f'MEMORY_VISUAL_IDENTICAL {screen}')
print('MEMORY_RECEIPT_VERIFIED')
