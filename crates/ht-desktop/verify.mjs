import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { compare } from 'odiff-bin';

const ROOT = path.resolve(import.meta.dirname, '../..');
const HERE = import.meta.dirname;
const APP = path.join(ROOT, 'target/release/ht-desktop');
const SEED = path.join(ROOT, 'target/release/ht-core-cli');
const EVIDENCE = path.join(HERE, 'evidence');
const assertNonblank = file => assert.ok(Number(execFileSync('identify', ['-format', '%k', file], { encoding: 'utf8' })) > 10, `${file}: expected a nonblank presented frame`);
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'ht-ui-'));
const mode = process.argv[2];
fs.mkdirSync(EVIDENCE, { recursive: true });
const xvfb = spawn('Xvfb', ['-displayfd', '1', '-screen', '0', '1440x900x24', '-nolisten', 'tcp'], { stdio: ['ignore', 'pipe', 'ignore'] });
const display = await new Promise((resolve, reject) => {
  xvfb.stdout.once('data', d => resolve(`:${d.toString().trim()}`));
  xvfb.once('exit', code => reject(new Error(`Xvfb exited ${code}`)));
  xvfb.once('error', reject);
});
const env = { ...process.env, DISPLAY: display, WAYLAND_DISPLAY: '' };
delete env.HT_TOKEN;
let active;

function seed(name, count = 0) {
  const db = path.join(tmp, `${name}.db`);
  const out = execFileSync(SEED, ['--db', db, 'import-fixtures', path.join(ROOT, 'fixtures/demo'), '--synthetic-tasks', `${count}`], { encoding: 'utf8' });
  if (count) assert.ok(query(db, "SELECT count(*) FROM Task WHERE projectId=15 AND status='Normal'")[0][0] >= 2000, out);
  return db;
}

async function launch(db, screen, extra = []) {
  const app = spawn(APP, ['--db', db, '--offline', '--screen', screen, '--board', '15', '--task', '58006', ...extra], { env, stdio: ['ignore', 'pipe', 'pipe'] });
  active = app;
  app.output = '';
  app.errors = '';
  app.stdout.on('data', chunk => { app.output += chunk; });
  app.stderr.on('data', chunk => { app.errors += chunk; });
  await waitOutput(app, /^READY/m);
  return app;
}

function waitOutput(app, pattern) {
  if (pattern.test(app.output)) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { cleanup(); app.kill(); reject(new Error(`app timeout ${pattern}: ${app.output}\n${app.errors}`)); }, 30000);
    const data = () => { if (pattern.test(app.output)) { cleanup(); resolve(); } };
    const exit = code => { cleanup(); reject(new Error(`app exited ${code}: ${app.output}\n${app.errors}`)); };
    const cleanup = () => { clearTimeout(timer); app.stdout.off('data', data); app.off('exit', exit); };
    app.stdout.on('data', data);
    app.once('exit', exit);
  });
}

async function stop(app) {
  if (app.exitCode !== null) return;
  const done = once(app, 'exit');
  app.kill();
  await done;
  if (active === app) active = undefined;
}

function xdo(...args) { return execFileSync('xdotool', args, { env, encoding: 'utf8' }).trim(); }
function query(db, sql) {
  return JSON.parse(execFileSync('python3', ['-c', 'import sqlite3,json,sys; print(json.dumps(sqlite3.connect(sys.argv[1]).execute(sys.argv[2]).fetchall()))', db, sql], { encoding: 'utf8' }));
}

try {
  if (mode === 'shots') {
    const black = path.join(tmp, 'black-control.png');
    execFileSync('convert', ['-size', '1440x900', 'xc:black', black]);
    assert.throws(() => assertNonblank(black), /nonblank presented frame/);
    const db = seed('screens');
    const diffs = {};
    for (const theme of ['porcelain', 'graphite', 'amoled', 'dia']) {
      for (const screen of ['inbox', 'board', 'task']) {
        const app = await launch(db, screen, ['--theme', theme, '--hold-after-ready']);
        await new Promise(resolve => setTimeout(resolve, 300));
        const file = path.join(EVIDENCE, `${screen}-${theme}.png`);
        execFileSync('import', ['-display', display, '-window', 'root', file]);
        assertNonblank(file);
        const window = xdo('search', '--class', 'ai.hypertask.desktop').split('\n').at(-1);
        const geometry = execFileSync('xwininfo', ['-display', display, '-id', window], { encoding: 'utf8' });
        assert.match(geometry, /Absolute upper-left X:\s+0\n/);
        assert.match(geometry, /Absolute upper-left Y:\s+0\n/);
        assert.match(geometry, /Width: 1440\n/);
        assert.match(geometry, /Height: 900\n/);
        await stop(app);
        const reference = path.join(ROOT, 'evidence/live', `${screen}-${theme}.png`);
        const nativeMasked = path.join(tmp, `${screen}-${theme}.png`);
        const liveMasked = path.join(tmp, `${screen}-${theme}.live.png`);
        fs.copyFileSync(file, nativeMasked);
        fs.copyFileSync(reference, liveMasked);
        if (screen !== 'inbox') {
          for (const mask of [nativeMasked, liveMasked]) execFileSync('convert', [mask, '-fill', 'black', '-draw', 'rectangle 1020,0 1440,900', mask]);
        }
        const diff = await compare(liveMasked, nativeMasked, path.join(tmp, `${screen}-${theme}.diff.png`), { antialiasing: true, threshold: 0.1 });
        diffs[`${screen}-${theme}`] = diff.match ? 0 : diff.diffPercentage ?? 100;
        console.log(`${screen}/${theme}: ${diffs[`${screen}-${theme}`].toFixed(2)}% diff`);
      }
    }
    fs.writeFileSync(path.join(EVIDENCE, 'pixel-diff.json'), JSON.stringify(diffs, null, 2));
    console.log('TWELVE_NATIVE_CAPTURES_OK');
  } else if (mode === 'edits') {
    const db = seed('edits');
    let app = await launch(db, 'task', ['--hold-after-ready']);
    let window = xdo('search', '--class', 'ai.hypertask.desktop').split('\n').at(-1);
    xdo('windowfocus', '--sync', window);
    xdo('key', 'Return', 'ctrl+a');
    xdo('type', '--clearmodifiers', '--delay', '8', 'Native queue proof title');
    const titleSaved = waitOutput(app, /^QUEUED 1$/m);
    xdo('key', 'Return');
    await titleSaved;
    assert.equal(query(db, 'SELECT title FROM Task WHERE id=58006')[0][0], 'Native queue proof title');
    xdo('key', 'Return', 'ctrl+a');
    xdo('type', '--clearmodifiers', '--delay', '8', 'Cancelled title');
    xdo('key', 'Escape');
    assert.equal(query(db, 'SELECT title FROM Task WHERE id=58006')[0][0], 'Native queue proof title');
    xdo('key', 'ctrl+m');
    xdo('type', '--clearmodifiers', '--delay', '8', 'Native queue proof comment <literal> &');
    const commentSaved = waitOutput(app, /^QUEUED 2$/m);
    xdo('key', 'ctrl+Return');
    await commentSaved;
    assert.equal(query(db, 'SELECT text FROM Comment WHERE id=-2')[0][0], 'Native queue proof comment &lt;literal&gt; &amp;');
    await stop(app);
    app = await launch(db, 'board', ['--hold-after-ready']);
    window = xdo('search', '--class', 'ai.hypertask.desktop').split('\n').at(-1);
    xdo('windowfocus', '--sync', window);
    const firstSection = query(db, "SELECT id FROM Section WHERE projectId=15 AND visibility=1 AND deleted=0 ORDER BY ranking,id LIMIT 1")[0][0];
    const secondSection = query(db, "SELECT id FROM Section WHERE projectId=15 AND visibility=1 AND deleted=0 ORDER BY ranking,id LIMIT 1 OFFSET 1")[0][0];
    const firstTask = query(db, `SELECT id FROM Task WHERE projectId=15 AND status='Normal' AND sectionId=${firstSection} ORDER BY updatedAt DESC,id ASC LIMIT 1`)[0][0];
    const moved = waitOutput(app, /^QUEUED 3$/m);
    xdo('key', 'shift+l');
    await moved;
    assert.equal(query(db, `SELECT sectionId FROM Task WHERE id=${firstTask}`)[0][0], secondSection);
    await stop(app);
    app = await launch(db, 'inbox', ['--hold-after-ready']);
    window = xdo('search', '--class', 'ai.hypertask.desktop').split('\n').at(-1);
    xdo('windowfocus', '--sync', window);
    const archived = waitOutput(app, /^QUEUED 4$/m);
    xdo('key', 'e');
    await archived;
    assert.equal(query(db, "SELECT count(*) FROM Notification WHERE status='Archive' AND archivedAt IS NOT NULL")[0][0], 1);
    const kinds = query(db, 'SELECT kind FROM mutation_queue ORDER BY id').flat();
    assert.deepEqual(kinds, ['task_title', 'task_comment', 'task_section', 'inbox_archive']);
    await stop(app);
    fs.writeFileSync(path.join(EVIDENCE, 'edits.json'), JSON.stringify({ kinds, title: 'Native queue proof title', comment: 'Native queue proof comment &lt;literal&gt; &amp;', movedTask: firstTask, from: firstSection, to: secondSection, offline: true }, null, 2));
    console.log('FOUR_NATIVE_QUEUED_EDITS_OK');
  } else if (mode === 'performance') {
    const db = seed('large', 2000);
    const memoryFile = path.join(tmp, 'memory.txt');
    const app = spawn('/usr/bin/time', ['-v', '-o', memoryFile, APP, '--db', db, '--offline', '--screen', 'board', '--board', '15', '--bench-input', '300'], { env, stdio: ['ignore', 'pipe', 'pipe'] });
    active = app;
    let output = '';
    app.stdout.on('data', chunk => { output += chunk; });
    app.stderr.pipe(process.stderr);
    const [code] = await once(app, 'exit');
    assert.equal(code, 0, output);
    assert.match(output, /^READY/m);
    const line = output.split('\n').find(l => l.startsWith('INPUT_TO_FRAME'));
    assert.ok(line, output);
    const stats = Object.fromEntries([...line.matchAll(/(\w+)=([\d.]+)/g)].map(m => [m[1], Number(m[2])]));
    assert.equal(stats.n, 300);
    const memory = fs.readFileSync(memoryFile, 'utf8');
    const peakMb = Number(memory.match(/Maximum resident set size \(kbytes\): (\d+)/)[1]) / 1024;
    const coldFile = path.join(tmp, 'cold.json');
    execFileSync('hyperfine', ['--warmup', '2', '--runs', '15', '--export-json', coldFile, `${APP} --db ${db} --offline --screen board --board 15 --exit-after-ready`], { env, stdio: 'inherit' });
    const cold = JSON.parse(fs.readFileSync(coldFile, 'utf8')).results[0];
    const report = { binarySha256: createHash('sha256').update(fs.readFileSync(APP)).digest('hex'), tasks: query(db, "SELECT count(*) FROM Task WHERE projectId=15 AND status='Normal'")[0][0], coldMedianMs: cold.median * 1000, coldRuns: cold.times.map(s => s * 1000), ...stats, peakMb, display: 'Xvfb 1440x900', renderer: 'lavapipe', budgets: { coldMedianMs: 300, p95: 16, peakMb: 100 } };
    fs.writeFileSync(path.join(EVIDENCE, 'performance.json'), JSON.stringify(report, null, 2));
    console.log(JSON.stringify(report, null, 2));
    console.log('REAL_PRESENTATION_MEASUREMENTS_RECORDED');
    assert.ok(report.coldMedianMs < 300, `cold median ${report.coldMedianMs}ms exceeds 300ms`);
    assert.ok(report.p95 < 16, `p95 ${report.p95}ms exceeds 16ms`);
    assert.ok(report.peakMb < 100, `peak ${report.peakMb}MB exceeds 100MB`);
    console.log('NATIVE_BUDGETS_OK');
  } else if (mode === 'parity') {
    const diffs = {};
    for (const theme of ['porcelain', 'graphite', 'amoled', 'dia']) {
      for (const screen of ['inbox', 'board', 'task']) {
        const name = `${screen}-${theme}`;
        const native = path.join(tmp, `${name}.png`);
        const live = path.join(tmp, `${name}.live.png`);
        fs.copyFileSync(path.join(EVIDENCE, `${name}.png`), native);
        fs.copyFileSync(path.join(ROOT, 'evidence/live', `${name}.png`), live);
        assertNonblank(native);
        if (screen !== 'inbox') for (const file of [native, live]) execFileSync('convert', [file, '-fill', 'black', '-draw', 'rectangle 1020,0 1440,900', file]);
        const result = await compare(live, native, path.join(tmp, `${name}.diff.png`), { antialiasing: true, threshold: 0.1 });
        diffs[name] = result.match ? 0 : result.diffPercentage ?? 100;
        assert.ok(diffs[name] <= 5, `${name}: ${diffs[name]}% exceeds 5%`);
      }
    }
    fs.writeFileSync(path.join(EVIDENCE, 'pixel-diff.json'), JSON.stringify(diffs, null, 2));
    console.log('TWELVE_NATIVE_PIXEL_DIFFS_BELOW_FIVE_PERCENT');
  } else if (mode === 'measurements') {
    const report = JSON.parse(fs.readFileSync(path.join(EVIDENCE, 'performance.json'), 'utf8'));
    assert.equal(report.binarySha256, createHash('sha256').update(fs.readFileSync(APP)).digest('hex'));
    assert.ok(report.tasks >= 2000);
    assert.equal(report.coldRuns.length, 15);
    assert.equal(report.n, 300);
    for (const metric of ['coldMedianMs', 'p50', 'p95', 'max', 'peakMb']) assert.ok(Number.isFinite(report[metric]) && report[metric] > 0);
    assert.ok(report.max >= report.p95 && report.p95 >= report.p50);
    console.log('REAL_INPUT_AND_LIFECYCLE_MEASURED');
  } else { throw new Error('usage: node crates/ht-desktop/verify.mjs shots|edits|performance|measurements|parity'); }
} finally {
  if (active && active.exitCode === null) await stop(active);
  xvfb.kill();
}
