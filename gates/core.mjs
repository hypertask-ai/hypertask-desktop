import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const run = spawnSync('bash', ['-c', 'source ~/.cargo/env && exec nice cargo test -j 8 -p ht-core --release -- --nocapture'], { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
const output = (run.stdout ?? '') + (run.stderr ?? '');
process.stdout.write(output);
if (run.error || run.status !== 0) process.exit(1);
const passed = [...output.matchAll(/^test (\S+) \.\.\. ok$/gm)].map(m => m[1]);
const contract = passed.filter(n => n.split('::').at(-1).startsWith('contract_')).length;
const prop = passed.filter(n => n.split('::').at(-1).startsWith('prop_')).length;
if (!contract || !prop || !/test result: ok\./.test(output)) {
  console.error('Core gate requires passing contract_ and prop_ tests');
  process.exit(1);
}
console.log(`CORE_TEST_COUNTS total=${passed.length} contract=${contract} prop=${prop}`);
console.log('CORE_GATE_OK');
