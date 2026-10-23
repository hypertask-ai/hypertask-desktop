#!/usr/bin/env python3
"""Verify downloaded laptop evidence, without executing the app on this host."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('rounds', nargs='+', type=Path)
parser.add_argument('--target', action='store_true')
parser.add_argument('--pixels', action='store_true')
args = parser.parse_args()
previous = None
baseline = None
for folder in args.rounds:
    data = json.loads((folder / 'ram.json').read_text(encoding='utf-8-sig'))
    sha = (folder / 'sha.txt').read_text().strip()
    assert len(sha) == 40 and all(c in '0123456789abcdef' for c in sha)
    assert f'completed success {sha}' in (folder / 'ci.txt').read_text()
    assert hashlib.sha256((folder / 'ht-desktop.exe').read_bytes()).hexdigest().upper() == data['ExeSHA256']
    assert (data['DisplayWidth'], data['DisplayHeight']) == (1920, 1080)
    assert len(data['Samples']) == 9
    assert len({s['Pid'] for s in data['Samples']}) == 9
    medians = {}
    for screen in ('inbox', 'board', 'task'):
        samples = [s for s in data['Samples'] if s['Screen'] == screen]
        assert sorted(s['Run'] for s in samples) == [1, 2, 3]
        assert all(s['Maximized'] and s['Dpi'] == 96 and s['ClientWidth'] == 1920 and s['ClientHeight'] > 900 for s in samples)
        median = next(m for m in data['Medians'] if m['Screen'] == screen)
        for key in ('WorkingSet64', 'PrivateMemorySize64'):
            assert median[key] == statistics.median(s[key] for s in samples)
        medians[screen] = median['WorkingSet64']
        print(f"{folder.name} {screen}: WS={median['WorkingSet64']/1e6:.3f} MB private={median['PrivateMemorySize64']/1e6:.3f} MB")
    if previous is not None:
        assert data['DatabaseSHA256'] == previous['DatabaseSHA256'], 'Database changed between rounds'
        old = {m['Screen']: m['WorkingSet64'] for m in previous['Medians']}
        assert sum(medians.values()) < sum(old.values()), 'Change did not reduce combined working set'
    previous = data
    if baseline is None:
        baseline = folder
if args.target:
    assert all(m['WorkingSet64'] < 100_000_000 for m in previous['Medians']), '100 MB target not reached'
if args.pixels:
    from PIL import Image, ImageChops
    for folder in args.rounds[1:]:
        for screen in ('inbox', 'board', 'task'):
            with Image.open(baseline / f'{screen}.png') as before, Image.open(folder / f'{screen}.png') as after:
                assert before.size == after.size
                assert ImageChops.difference(before.convert('RGB'), after.convert('RGB')).getbbox() is None, f'{folder.name}/{screen}: pixels changed'
            print(f'{folder.name}/{screen}: pixels identical')
print('RAM_EVIDENCE_VERIFIED')
