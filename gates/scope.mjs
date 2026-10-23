import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).trim();
const coreCommit = git('log', '-1', '--format=%H', '--', 'crates/ht-core/src/generated.rs');
const owned = ['crates/ht-core', 'crates/ht-schema-gen', 'gates/core.mjs', 'gates/schema-fresh.mjs', 'gates/scope.mjs'];
const commits = git('log', '--format=%H', `${coreCommit}^..HEAD`, '--', ...owned).split('\n');
for (const commit of commits) {
  const message = git('show', '-s', '--format=%B', commit);
  assert.match(message, /^HTPR-7381 \[FEATURE\] /);
  assert.match(message, /^Co-Authored-By: Claude Opus 5\.5 \(1M context\) <noreply@anthropic\.com>$/m);
  assert.match(message, /^Claude-Session: https:\/\/claude\.ai\/code\/session_012WJcheVXWZUXRqoyPT1gbq$/m);
  const paths = git('diff-tree', '--no-commit-id', '--name-only', '-r', commit).split('\n');
  for (const path of paths) assert(/^(crates\/ht-core\/|crates\/ht-schema-gen\/|gates\/(core|schema-fresh|scope)\.mjs$|Cargo\.lock$|\.gitignore$)/.test(path), `Outside core scope: ${path}`);
}
const paths = git('diff-tree', '--no-commit-id', '--name-only', '-r', coreCommit).split('\n');
for (const path of ['crates/ht-core/src/generated.rs', 'gates/core.mjs', 'gates/schema-fresh.mjs', 'Cargo.lock']) assert(paths.includes(path), `Missing deliverable: ${path}`);
assert.equal(git('diff', '--name-only', 'HEAD', '--', ...owned), '', 'Core deliverables must match their commits');
assert.match(git('show', `${coreCommit}:.gitignore`), /^\.unlazy\/$/m);
assert.match(execFileSync('git', ['check-ignore', '.unlazy/core/GATES.md'], { cwd: root, encoding: 'utf8' }), /\.unlazy\/core\/GATES\.md/);
console.log('CORE_SCOPE_OK');
