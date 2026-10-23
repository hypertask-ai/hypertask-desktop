import { spawnSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { dirname, join, resolve } from 'node:path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const schema = process.env.HT_PRISMA_SCHEMA;
if (!schema) {
  console.error('set HT_PRISMA_SCHEMA to the path of the server schema.prisma');
  process.exit(2);
}
const cargo = (...args) => spawnSync('bash', ['-c', 'source ~/.cargo/env && exec nice cargo run -j 8 -p ht-schema-gen -- "$@"', 'schema-fresh', schema, ...args], { cwd: root, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024 });
const run = cargo('--check');
process.stdout.write(run.stdout ?? '');
process.stderr.write(run.stderr ?? '');
if (run.error || run.status !== 0) process.exit(1);
const temp = mkdtempSync(join(tmpdir(), 'ht-schema-fresh-'));
try {
  const output = join(temp, 'generated.rs');
  let result = cargo('--output', output);
  assert.equal(result.status, 0, result.stderr);
  const first = readFileSync(output, 'utf8');
  assert.equal(first, readFileSync(join(root, 'crates/ht-core/src/generated.rs'), 'utf8'));
  result = cargo('--output', output);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(readFileSync(output, 'utf8'), first, 'Generator is not deterministic');
  writeFileSync(output, first + '\n');
  const stale = cargo('--check', '--output', output);
  assert.notEqual(stale.status, 0, 'Freshness check accepted stale output');
  assert.match(stale.stderr, /generated\.rs is stale/);
} finally {
  rmSync(temp, { recursive: true, force: true });
}
console.log('SCHEMA_FRESH_OK');
