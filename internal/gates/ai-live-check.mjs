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
const mode = process.argv[2];
const composeMode = mode === '--compose' || mode === '--discover' || mode === '--seed' || mode === '--diagnose';
const qaEmail = composeMode ? 'valentin+qa-owner@hypertask.ai' : 'valentin@hypertask.ai';
const qaUserId = composeMode ? 4036 : 985;
const fixture = (composeMode ? process.argv[3] : process.argv[2]) ?? '/home/valentin/projects/hypertask-desktop-wt-ai/target/ai-live-request.json';
let browser, server, timer;

try {
  // Read the CI-generated Rust body unchanged: no parallel JavaScript payload builder.
  const body = composeMode ? null : await fs.readFile(fixture, 'utf8');
  const request = composeMode ? {} : JSON.parse(body);
  for (const key of (composeMode ? [] : ['session_id', 'user_message_id', 'assistant_message_id', 'stream_id'])) {
    assert.match(request[key], /^[0-9a-f-]{36}$/i);
  }
  let storageState = JSON.parse(await fs.readFile(path.join(os.homedir(), '.config/ht-qa/state-owner.json'), 'utf8'));
  const savedUser = state => JSON.parse(decodeURIComponent(state.cookies.find(cookie => cookie.name === 'nookies_user').value));
  const suppliedUser = savedUser(storageState);
  if (!composeMode && suppliedUser.id === 4036 && suppliedUser.email === 'valentin+qa-owner@hypertask.ai') {
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
  console.log('identity 200 {qa:true,human:true}');
  const headers = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
  if (mode === '--seed') {
    const defaults = await fetch(`${origin}/api/tasks/createGlobally?projectId=7283&position=top`, {headers,redirect:'error'});
    console.log(`defaults ${defaults.status}`); assert.equal(defaults.status,200);
    const response = await fetch(`${origin}/api/mcp/comments`, {method:'POST',headers,redirect:'error',body:JSON.stringify({task_id:60253,text:'<p>HTPR-7381 temporary desktop contract probe</p>',kind:'qa',content_type:'html'})});
    console.log(`seed ${response.status}`); assert(response.ok);
    const value=await response.json(); console.log(`seed ${response.status} {commentId:${value.comment.id}}`);
  } else if (mode === '--discover') {
    const response = await fetch(`${origin}/api/mcp/comments?task_id=60253&limit=5&sort_order=desc`, {headers, redirect:'error'});
    console.log(`comments ${response.status}`);
    assert.equal(response.status,200);
    const value = await response.json();
    console.log(`comments ${response.status} {count:${value.comments.length},firstId:${value.comments[0]?.id}}`);
  } else if (mode === '--compose' || mode === '--diagnose') {
    const fixtures = JSON.parse(await fs.readFile(fixture,'utf8'));
    const run = async (name, request) => {
      assert.equal(new URL(request.path,origin).origin,origin);
      const response = await fetch(`${origin}${request.path}`, {
        method:request.method,headers,redirect:'error',signal:AbortSignal.timeout(310_000),
        ...(request.method === 'GET' ? {} : {body:JSON.stringify(request.body)}),
      });
      const text=await response.text();
      let value; try {value=JSON.parse(text);} catch {value=text;}
      const shape=typeof value==='string' ? 'text' : Array.isArray(value) ? `array(${value.length})` : `{${Object.keys(value).slice(0,6).join(',')}}`;
      console.log(`${name} ${response.status} ${shape}`);
      if (mode !== '--diagnose') assert(response.ok);
      return {status:response.status,value};
    };
    assert.equal(fixtures.writer.body.projectId,7283);
    assert.equal(fixtures.star.body.taskId,60253);
    assert.equal(fixtures.star.body.projectId,7283);
    assert.equal(fixtures.pin.body.type,'Public');
    assert.equal(fixtures.star.body.type,'Private');
    if (mode === '--diagnose') {
      const results=[];
      for (const name of ['defaults','writer','mentions','star','pin']) results.push(await run(name,fixtures[name]));
      assert(results.every(result=>result.status>=200 && result.status<300));
    } else {
    const defaults=await run('defaults',fixtures.defaults);
    assert(Number.isInteger(defaults.value.sectionId) && typeof defaults.value.ranking==='string');
    const mentions=await run('mentions',fixtures.mentions);
    assert(Array.isArray(mentions.value));
    const writer=await run('writer',fixtures.writer);
    assert(typeof writer.value==='string' ? writer.value.includes('ai-generated-task-title') && !/^event:\s*(?:error|done)\b/m.test(writer.value) : Array.isArray(writer.value.tasks) && writer.value.tasks.length>0);
    // Begin from a known empty QA-only saved state; cleanup also runs after a failed acknowledgement.
    try {
      await run('unstar-before',fixtures.unstar);
      await run('unpin-before',fixtures.unpin);
      const star=await run('star',fixtures.star);
      assert.equal(star.status,200); assert(star.value.saved);
      const pin=await run('pin',fixtures.pin);
      assert.equal(pin.status,200); assert(pin.value.saved);
    } finally {
      const star=await run('unstar',fixtures.unstar);
      const pin=await run('unpin',fixtures.unpin);
      assert.equal(star.status,201); assert.equal(pin.status,201);
    }
    console.log('compose-contract 200 {routes:4,tasksCreated:0,cleaned:true}');
    }
  } else {
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
  }
} catch {
  // Never dump browser URLs, exchange bodies, exception objects, codes, cookies, or tokens.
  process.exitCode = 1;
} finally {
  clearTimeout(timer);
  await browser?.close();
  if (server) await new Promise(resolve => server.close(resolve));
}
