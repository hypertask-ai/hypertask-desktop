import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { APP, ROOT, buildRelease, run, seedLargeBoard, withXvfb } from './lib.mjs';
import { reportStartup } from './cold-start-report.mjs';

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'ht-cold-'));
const db = path.join(tmp, 'local.db');
const report = path.join(tmp, 'hyperfine.json');

buildRelease();
seedLargeBoard(db);
await withXvfb(async () => {
  run('hyperfine', ['--warmup', '2', '--runs', '15', '--export-json', report,
    `${APP} --db ${db} --offline --screen board --board 15 --exit-after-ready`]);
});
const evidence = path.join(ROOT, 'evidence/cold-start');
fs.mkdirSync(evidence, { recursive: true });
fs.copyFileSync(report, path.join(evidence, 'hyperfine.json'));
if (!reportStartup(JSON.parse(fs.readFileSync(report, 'utf8')))) process.exit(1);
console.log('COLD_START_OK');
