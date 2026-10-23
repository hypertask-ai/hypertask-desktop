import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { APP, buildRelease, peakMemoryMb, seedLargeBoard, withXvfb } from './lib.mjs';

const BUDGET_MB = 100;
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'ht-mem-'));
const db = path.join(tmp, 'local.db');

buildRelease();
seedLargeBoard(db);
const mb = await withXvfb(async () =>
  peakMemoryMb(APP, ['--db', db, '--offline', '--screen', 'board', '--board', '15', '--bench-input', '300']));
console.log(`peak memory ${mb.toFixed(1)} MB on ${process.platform} with a 2,000+ task board after 300 inputs (budget ${BUDGET_MB} MB)`);
if (mb >= BUDGET_MB) process.exit(1);
console.log('MEMORY_OK');
