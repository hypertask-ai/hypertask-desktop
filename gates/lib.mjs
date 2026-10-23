import { execFileSync, spawn, spawnSync } from 'node:child_process';
import path from 'node:path';

export const ROOT = path.resolve(import.meta.dirname, '..');
export const APP = path.join(ROOT, 'target/release/ht-desktop');
export const CORE_CLI = path.join(ROOT, 'target/release/ht-core-cli');
export const DISPLAY = ':94';
export const GUI_ENV = { ...process.env, DISPLAY, WAYLAND_DISPLAY: '' };

export async function withXvfb(fn) {
  if (process.platform !== 'linux') return fn();
  const xvfb = spawn('Xvfb', [DISPLAY, '-screen', '0', '1440x900x24'], { stdio: 'ignore' });
  await new Promise(r => setTimeout(r, 1000));
  try {
    return await fn();
  } finally {
    xvfb.kill();
  }
}

export function buildRelease() {
  const cargo = ['cargo', 'build', '--release', '-p', 'ht-desktop', '-p', 'ht-core'];
  const [cmd, ...args] = process.platform === 'win32' ? cargo : ['nice', ...cargo];
  execFileSync(cmd, args, { cwd: ROOT, stdio: 'inherit' });
}

export function peakMemoryMb(command, args) {
  if (process.platform === 'darwin') {
    const run = spawnSync('/usr/bin/time', ['-l', command, ...args], { encoding: 'utf8', env: GUI_ENV });
    if (run.status !== 0) throw new Error(`${command} exited ${run.status}: ${run.stderr}`);
    return Number(run.stderr.match(/(\d+)\s+maximum resident set size/)[1]) / 1024 / 1024;
  }
  const run = spawnSync('/usr/bin/time', ['-v', command, ...args], { encoding: 'utf8', env: GUI_ENV });
  if (run.status !== 0) throw new Error(`${command} exited ${run.status}: ${run.stderr}`);
  return Number(run.stderr.match(/Maximum resident set size \(kbytes\): (\d+)/)[1]) / 1024;
}

export function seedLargeBoard(dbPath) {
  execFileSync(CORE_CLI, ['--db', dbPath, 'import-fixtures', path.join(ROOT, 'fixtures/demo'), '--synthetic-tasks', '2000'], { stdio: 'inherit' });
}

export function run(cmd, args, env = GUI_ENV) {
  return execFileSync(cmd, args, { encoding: 'utf8', env });
}
