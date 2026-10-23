import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { APP, buildRelease, run, seedLargeBoard, withXvfb } from './lib.mjs';

const BUDGET_MS = 300;
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'ht-cold-'));
const db = path.join(tmp, 'local.db');
const report = path.join(tmp, 'hyperfine.json');

buildRelease();
seedLargeBoard(db);
await withXvfb(async () => {
  run('hyperfine', ['--warmup', '2', '--runs', '15', '--export-json', report,
    `${APP} --db ${db} --offline --screen board --board 15 --exit-after-ready`]);
});
const { results } = JSON.parse(fs.readFileSync(report, 'utf8'));
const medianMs = results[0].median * 1000;
console.log(`cold start median ${medianMs.toFixed(1)} ms over ${results[0].times.length} runs (budget ${BUDGET_MS} ms)`);
if (medianMs >= BUDGET_MS) process.exit(1);
console.log('COLD_START_OK');
