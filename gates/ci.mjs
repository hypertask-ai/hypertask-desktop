import { execFileSync } from 'node:child_process';

const run = (cmd, args) => execFileSync(cmd, args, { encoding: 'utf8' }).trim();
const headSha = run('git', ['rev-parse', 'origin/main']);
const runs = JSON.parse(run('gh', ['run', 'list', '--workflow', 'ci.yml', '--commit', headSha, '--json', 'databaseId,status,conclusion']));
const finished = runs.find(r => r.status === 'completed');
if (!finished) {
  console.log(`no completed CI run for ${headSha}`);
  process.exit(1);
}
const { jobs } = JSON.parse(run('gh', ['run', 'view', String(finished.databaseId), '--json', 'jobs']));
const required = ['ubuntu-24.04', 'windows-2025', 'macos-15'];
const failing = required.filter(os => !jobs.some(j => j.name === os && j.conclusion === 'success'));
for (const j of jobs) console.log(`${j.name}: ${j.conclusion}`);
if (failing.length) {
  console.log(`not green: ${failing.join(', ')}`);
  process.exit(1);
}
console.log('CI_GREEN_OK');
