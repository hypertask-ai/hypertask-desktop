import { spawnSync } from 'node:child_process';
import path from 'node:path';

const REPORTER = path.join(import.meta.dirname, 'cold-start-report.mjs');
const FIXTURES = path.join(import.meta.dirname, 'fixtures');

function report(fixture) {
  return spawnSync(process.execPath, [REPORTER, path.join(FIXTURES, fixture)], {
    encoding: 'utf8',
  });
}

const fast = report('hyperfine-v2-fast.json');
if (fast.status !== 0) {
  console.error(fast.stdout, fast.stderr);
  throw new Error(`fast report exited ${fast.status}, expected 0`);
}
if (!fast.stdout.includes('cold start samples (ms):')) {
  throw new Error('fast report did not print every startup sample');
}
const fastSamples = fast.stdout.match(/cold start samples \(ms\): (.*)/)[1].split(', ');
if (fastSamples.length !== 15) {
  throw new Error(`fast report printed ${fastSamples.length} samples, expected 15`);
}
if (!fast.stdout.includes('COLD_START_OK')) {
  throw new Error('fast report did not print COLD_START_OK');
}

const slow = report('hyperfine-v2-slow.json');
if (slow.status !== 1) {
  console.error(slow.stdout, slow.stderr);
  throw new Error(`slow report exited ${slow.status}, expected 1`);
}
if (!slow.stdout.includes('cold start samples (ms):')) {
  throw new Error('slow report did not print every startup sample before failing');
}
if (slow.stdout.includes('COLD_START_OK')) {
  throw new Error('slow report passed under the budget');
}

const legacy = report('hyperfine-v1-fast.json');
if (legacy.status !== 0) {
  console.error(legacy.stdout, legacy.stderr);
  throw new Error(`Hyperfine 1 report exited ${legacy.status}, expected 0`);
}
const legacySamples = legacy.stdout.match(/cold start samples \(ms\): (.*)/)[1].split(', ');
if (legacySamples.length !== 3 || !legacy.stdout.includes('COLD_START_OK')) {
  throw new Error('Hyperfine 1 report did not print its samples and pass');
}

console.log('fast report: 15 samples, COLD_START_OK');
console.log(`slow report: exit ${slow.status}, all samples printed, no COLD_START_OK`);
console.log('legacy report: 3 samples, COLD_START_OK');
console.log('COLD_START_REPORT_OK');
