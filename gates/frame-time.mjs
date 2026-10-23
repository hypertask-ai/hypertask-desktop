import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { APP, buildRelease, run, seedLargeBoard, withXvfb } from './lib.mjs';

const BUDGET_MS = 16;
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'ht-frames-'));
const db = path.join(tmp, 'local.db');

buildRelease();
seedLargeBoard(db);
const out = await withXvfb(async () =>
  run(APP, ['--db', db, '--offline', '--screen', 'board', '--board', '15', '--bench-input', '300']));
const line = out.split('\n').find(l => l.startsWith('INPUT_TO_FRAME'));
if (!line) {
  console.log('no INPUT_TO_FRAME line from the app');
  process.exit(1);
}
const stats = Object.fromEntries([...line.matchAll(/(\w+)=([\d.]+)/g)].map(m => [m[1], Number(m[2])]));
console.log(`${line} (budget p95 < ${BUDGET_MS} ms)`);
if (!(stats.n >= 300) || !(stats.p95 < BUDGET_MS)) process.exit(1);
console.log('FRAME_TIME_OK');
