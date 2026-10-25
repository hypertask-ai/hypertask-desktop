import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { chromium } from 'playwright';
import { collectRail } from './layout-collect-rail.mjs';

assert.equal(process.env.GITHUB_ACTIONS, 'true', 'Rail collection runs only on GitHub Actions');
const out = 'evidence/layout';
const frozen = 'gates/fixtures/layout-reference-live';
const complete = `${out}/reference-complete-web`;
mkdirSync(complete, { recursive: true });
const source = readFileSync('gates/fixtures/layout-rail/source.html', 'utf8');
const expectedKeys = Array.from({ length: 13 }, (_, i) => `rail.icon.${i}`);
const fields = ['x', 'y', 'w', 'h', 'color', 'bg', 'fontSize', 'fontWeight', 'lineHeight', 'radius', 'borderColor'];
function requireControls(elements) {
  assert.deepEqual(elements.map(element => element.key), expectedKeys, 'Unmeasured rail controls');
}
function compare(live, native) {
  const result = spawnSync('python3', ['gates/layout-compare.py', live, native, '1920'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const total = /^TOTAL (\d+)$/m.exec(result.stdout);
  assert.ok(total, result.stdout);
  return { differences: Number(total[1]), output: result.stdout };
}
const browser = await chromium.launch({ headless: true });
const records = [];
try {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1009 }, deviceScaleFactor: 1 });
  for (const screen of ['inbox', 'board', 'task']) {
    await page.setContent(source);
    await page.evaluate(screen => {
      const selected = screen === 'inbox' ? '[aria-label="Go to inbox"]' : screen === 'board' ? '[aria-label="Board layout"]' : null;
      if (selected) document.querySelector(selected).classList.add('selected');
    }, screen);
    const elements = await page.evaluate(collectRail);
    requireControls(elements);
    const original = JSON.parse(readFileSync(`${frozen}/${screen}.json`, 'utf8'));
    // The source projection must reproduce every already-captured rail field.
    for (const element of original.elements.filter(element => element.key.startsWith('rail.icon.'))) {
      const measured = elements.find(other => other.key === element.key);
      for (const field of fields) {
        if (typeof element[field] === 'number') {
          const tolerance = ['x', 'y', 'w', 'h'].includes(field) ? 1.5 : 0.5;
          assert.ok(Math.abs(element[field] - measured[field]) <= tolerance, `${screen}/${element.key}.${field}: ${element[field]} != ${measured[field]}`);
        } else {
          assert.equal(measured[field], element[field], `${screen}/${element.key}.${field}`);
        }
      }
    }
    const originalKeys = new Set(original.elements.map(element => element.key));
    const added = elements.filter(element => !originalKeys.has(element.key));
    assert.deepEqual(added.map(element => element.key), expectedKeys.slice(5));
    writeFileSync(`${out}/rail-collected-${screen}.json`, JSON.stringify({ screen, elements }, null, 2) + '\n');
    // Existing live records remain byte-for-value unchanged. Only missing keys append.
    writeFileSync(`${complete}/${screen}.json`, JSON.stringify({ ...original, elements: [...original.elements, ...added] }, null, 2) + '\n');
    records.push({ screen, measured: elements.length, added: added.length });
    await page.evaluate(() => { document.querySelector('.bottom').style.display = 'none'; });
    assert.throws(() => requireControls([]), /Unmeasured rail controls/);
    const missingBottom = await page.evaluate(collectRail);
    assert.throws(() => requireControls(missingBottom), /Unmeasured rail controls/);
    console.log(`LAYOUT_COLLECTOR_MISSING_GROUP_REJECTED ${screen}`);
  }
} finally {
  await browser.close();
}
const comparisons = {};
for (const [name, directory] of [['dense', out], ['parity', `${out}/parity`], ['reference', `${out}/reference`]]) {
  const result = compare(complete, directory);
  writeFileSync(`${out}/compare-complete-${name}.txt`, result.output);
  console.log(`COMPLETE_FIXTURE ${name}\n${result.output}`);
  comparisons[name] = result.differences;
}
const negative = `${out}/rail-collector-negative`;
mkdirSync(negative, { recursive: true });
for (const screen of ['inbox', 'board', 'task']) {
  const dump = JSON.parse(readFileSync(`${out}/reference/${screen}.json`, 'utf8'));
  if (screen === 'inbox') dump.elements.find(element => element.key === 'rail.icon.12').x += 4;
  writeFileSync(`${negative}/${screen}.json`, JSON.stringify(dump, null, 2) + '\n');
}
const perturbed = compare(complete, negative);
writeFileSync(`${out}/compare-collector-negative.txt`, perturbed.output);
assert.equal(perturbed.differences, comparisons.reference + 1, perturbed.output);
console.log('LAYOUT_COLLECTOR_BOX_PERTURBATION_REJECTED');
const before = JSON.parse(readFileSync('gates/fixtures/layout-rail/radius-before.json', 'utf8'));
assert.equal(before.records.length, 12);
assert.equal(new Set(before.records.map(record => `${record.screen}/${record.element.key}`)).size, 12);
const oldDirectory = `${out}/rail-radius-before`;
mkdirSync(oldDirectory, { recursive: true });
for (const screen of ['inbox', 'board', 'task']) {
  const dump = JSON.parse(readFileSync(`${out}/reference/${screen}.json`, 'utf8'));
  const old = new Map(before.records.filter(record => record.screen === screen).map(record => [record.element.key, record.element]));
  assert.deepEqual([...old.keys()], expectedKeys.slice(9));
  dump.elements = dump.elements.map(element => old.get(element.key) ?? element);
  writeFileSync(`${oldDirectory}/${screen}.json`, JSON.stringify(dump, null, 2) + '\n');
}
const rejected = compare(complete, oldDirectory);
writeFileSync(`${out}/compare-radius-before.txt`, rejected.output);
assert.equal(rejected.differences, 12, rejected.output);
console.log(`LAYOUT_OLD_RAIL_RADIUS_REJECTED ${before.revision}: ${rejected.differences} differences`);
const receipt = {
  sourceRevision: '10377b20e8c338357fda306b025ef16699c9b4ab',
  source: 'src/components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:65-87,151-360',
  sourceProjectionSha256: createHash('sha256').update(source).digest('hex'),
  coverage: 'Source-projected browser rail measurement; original frozen non-rail records retained. Not a new authenticated live-app capture.',
  records, comparisons,
};
writeFileSync(`${out}/rail-collector-comparison.json`, JSON.stringify(receipt, null, 2) + '\n');
assert.equal(comparisons.reference, 0, 'Real source-backed reference differences remain');
console.log(`LAYOUT_COMPLETE_REFERENCE_OK: ${records.reduce((sum, record) => sum + record.added, 0)} added records; zero reference differences`);
