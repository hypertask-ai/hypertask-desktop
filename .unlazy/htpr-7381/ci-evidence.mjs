import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
const base = 'https://api.github.com/repos/hypertask-ai/hypertask-desktop';
async function get(path) {
  const response = await fetch(base + path, {headers:{'User-Agent':'HTPR-7381-public-CI-evidence'},signal:AbortSignal.timeout(30000)});
  assert.equal(response.status,200);
  return response.json();
}
const head = execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
const run = await get('/actions/runs/40029947557');
assert.equal(run.head_sha,head);
const {jobs} = await get('/actions/runs/40029947557/jobs');
assert.deepEqual(jobs.map(j=>j.name).sort(),['macos-15','ubuntu-24.04','windows-2025','windows-arm64']);
for (const job of jobs) {
  console.log(`${job.name}: ${job.conclusion ?? job.status}`);
  assert.equal(job.status,'completed');
  assert.equal(job.conclusion,'success');
}
assert.equal(run.status,'completed');
assert.equal(run.conclusion,'success');
const linux = jobs.find(j=>j.name==='ubuntu-24.04');
for (const name of ['Run cargo test --workspace --release','Layout dump contract','Upload Rust AI request fixture']) {
  assert.equal(linux.steps.find(s=>s.name===name)?.conclusion,'success');
}
const pr = await get('/pulls/3');
assert.equal(pr.head.sha,head);
assert.equal(pr.draft,false);
console.log(`${head} ${run.html_url}`);
console.log('ALL_CI_JOBS_GREEN_AND_PR_READY');
