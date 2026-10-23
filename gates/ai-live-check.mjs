#!/usr/bin/env node
// Run only the production contract probe here; Rust fixture generation belongs to GitHub CI.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire('/home/valentin/projects/hypertask-desktop/package.json');
const { chromium } = require('playwright');
const origin = 'https://app.hypertask.ai';
const qaEmail = 'valentin@hypertask.ai';
const qaUserId = 985;
const fixture = process.argv[2] ?? '/home/valentin/projects/hypertask-desktop-wt-ai/target/ai-live-request.json';
let browser, server, timer;

try {
  // Read the CI-generated Rust body unchanged: no parallel JavaScript payload builder.
  const body = await fs.readFile(fixture, 'utf8');
  const request = JSON.parse(body);
  for (const key of ['session_id', 'user_message_id', 'assistant_message_id', 'stream_id']) {
    assert.match(request[key], /^[0-9a-f-]{36}$/i);
  }
  let storageState = JSON.parse(await fs.readFile(path.join(os.homedir(), '.config/ht-qa/state-owner.json'), 'utf8'));
  const savedUser = state => JSON.parse(decodeURIComponent(state.cookies.find(cookie => cookie.name === 'nookies_user').value));
  const suppliedUser = savedUser(storageState);
  if (suppliedUser.id === 4036 && suppliedUser.email === 'valentin+qa-owner@hypertask.ai') {
    // The supplied team-owner QA fixture is not the server's OWNER_AND_QA identity (985).
    // Use the existing flag-QA state in memory; never overwrite either shared state file.
    storageState = JSON.parse(await fs.readFile(path.join(os.homedir(), '.config/hypertask-videos/storageState-qa.json'), 'utf8'));
  }
  assert.equal(savedUser(storageState).id, qaUserId);
  assert.equal(savedUser(storageState).email, qaEmail);

  let resolveCode, rejectCode;
  const callbackCode = new Promise((resolve, reject) => { resolveCode = resolve; rejectCode = reject; });
  // Attach a rejection handler immediately, including during browser launch/navigation.
  callbackCode.catch(() => {});
  server = http.createServer((req, res) => {
    const url = new URL(req.url, 'http://localhost');
    if (req.method !== 'GET' || url.pathname !== '/callback') {
      res.writeHead(404).end();
    } else if (!url.searchParams.get('code')) {
      res.writeHead(400).end();
    } else {
      res.writeHead(200, { 'Content-Type': 'text/plain' }).end('Return to Hypertask.');
      resolveCode(url.searchParams.get('code'));
    }
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  timer = setTimeout(() => rejectCode(new Error('Callback timed out')), 180_000);
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ storageState, serviceWorkers: 'block' });
  const identity = await context.request.get(`${origin}/api/users/getById`);
  assert.equal(identity.status(), 200);
  assert.equal((await identity.json()).id, qaUserId);
  const page = await context.newPage();
  page.setDefaultTimeout(60_000);
  await page.goto(`${origin}/cli-auth?port=${server.address().port}`, { waitUntil: 'domcontentloaded' });
  await page.getByRole('button', { name: /Allow and continue/i }).click();
  const sameDevice = page.getByRole('button', { name: /same device/i });
  const needsClick = await Promise.race([
    callbackCode.then(() => false),
    sameDevice.waitFor({ state: 'visible' }).then(() => true),
  ]);
  if (needsClick) await sameDevice.click();
  const code = await callbackCode;
  clearTimeout(timer);
  await browser.close();
  browser = undefined;

  // Native fetch is separate from the browser context: no cookies and no sidebar-project header.
  const exchange = await fetch(`${origin}/api/cli/token-exchange`, {
    method: 'POST', redirect: 'error', signal: AbortSignal.timeout(30_000),
    headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ code }),
  });
  console.log(`exchange ${exchange.status}`);
  assert.equal(exchange.status, 200);
  const { token } = await exchange.json();
  assert.equal(typeof token, 'string');
  // Identity guard only; production validates the signature and authorization.
  const claims = JSON.parse(Buffer.from(token.split('.')[1], 'base64url').toString('utf8'));
  assert.equal(claims.sub, qaEmail);
  assert.equal(claims.userId, qaUserId);
  assert(!claims.agentId);
  const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
  const response = await fetch(`${origin}/api/ai/chat/stream`, {
    method: 'POST', redirect: 'error', signal: AbortSignal.timeout(310_000),
    headers: { ...headers, Accept: 'text/event-stream' }, body,
  });
  console.log(`stream ${response.status}`);
  assert.equal(response.status, 200);
  assert.match(response.headers.get('content-type'), /^text\/event-stream\b/);
  const reader = response.body.getReader();
  const decoder = new TextDecoder('utf-8', { fatal: true });
  let buffer = '', reply = '', done = false, failed = false;
  const events = new Set();
  try {
    while (!done) {
      const chunk = await reader.read();
      buffer += decoder.decode(chunk.value, { stream: !chunk.done });
      buffer = buffer.replace(/\r\n/g, '\n');
      let boundary;
      while ((boundary = buffer.indexOf('\n\n')) !== -1) {
        const frame = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + 2);
        const lines = frame.split('\n');
        const event = lines.find(line => line.startsWith('event:'))?.slice(6).trim();
        const data = lines.filter(line => line.startsWith('data:')).map(line => line.slice(5).replace(/^ /, '')).join('\n');
        if (!event || !data) continue;
        assert(['status', 'content', 'title', 'agent', 'error', 'done', 'thinking', 'complete'].includes(event));
        if (!events.has(event)) {
          events.add(event);
          console.log(`event ${event}`);
        }
        const payload = JSON.parse(data);
        if (event === 'content') reply = Array.from(reply + (payload.content ?? '')).slice(0, 80).join('');
        if (event === 'error') failed = true;
        if (event === 'done' || event === 'complete') {
          done = true;
          failed ||= payload.status !== 'complete' || payload.cancelled === true;
          break;
        }
      }
      assert(buffer.length <= 1024 * 1024);
      if (chunk.done) break;
    }
  } finally {
    await reader.cancel();
  }
  console.log(`reply ${reply.replace(/[\r\n\t]/g, ' ')}`);
  const cancel = await fetch(`${origin}/api/ai/chat/cancel`, {
    method: 'POST', redirect: 'error', signal: AbortSignal.timeout(30_000), headers,
    body: JSON.stringify({
      session_id: request.session_id,
      assistant_message_id: request.assistant_message_id,
      stream_id: request.stream_id,
    }),
  });
  console.log(`cancel ${cancel.status}`);
  assert([200, 409].includes(cancel.status));
  if (cancel.status === 200) assert.equal((await cancel.json()).success, true);
  assert(done && !failed && events.has('content') && reply.length > 0);
} catch {
  // Never dump browser URLs, exchange bodies, exception objects, codes, cookies, or tokens.
  process.exitCode = 1;
} finally {
  clearTimeout(timer);
  await browser?.close();
  if (server) await new Promise(resolve => server.close(resolve));
}
