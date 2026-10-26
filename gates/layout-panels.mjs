import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { chromium } from 'playwright';

assert.equal(process.env.GITHUB_ACTIONS, 'true', 'Panel measurement runs only on GitHub Actions');
const out = 'evidence/layout';
const source = readFileSync('gates/fixtures/layout-panels/source.html', 'utf8');
const font = readFileSync('crates/ht-desktop/assets/fonts/IBMPlexSans-400.ttf');
const medium = readFileSync('crates/ht-desktop/assets/fonts/IBMPlexSans-500.ttf');
const rows = JSON.parse(readFileSync('crates/ht-desktop/src/shell_shortcuts.json', 'utf8'));
const prompts = ['What should I work on next?', 'Summarize the last 3 days', "What's unassigned?", "What's blocking release?"];
const key = (name, tag, content, attrs = '') => `<${tag} data-key="${name}" ${attrs}>${content}</${tag}>`;
const svg = size => `<svg width="${size}" height="${size}"></svg>`;
const html = state => {
  if (state === 'shortcuts') {
    let group = '';
    return key('shell.shortcuts', 'aside', key('settings.shortcuts.search', 'div', 'Search', 'class="search"') + `<div class="shortcut-content">${rows.slice(0, 9).map((row, i) => {
      let heading = '';
      if (group !== row.group) { group = row.group; heading = key(`settings.shortcut.group.${i}`, 'h3', group, 'class="group"'); }
      return heading + key(`settings.shortcut.${i}`, 'div', key(`settings.shortcut.label.${i}`, 'p', row.label, 'class="label"') + key(`settings.shortcut.keys.${i}`, 'div', row.keys.map((value, j) => key(`settings.shortcut.key.${i}.${j}`, 'kbd', value ?? 'then')).join(''), 'class="keys"'), 'class="shortcut"');
    }).join('')}</div>`, 'class="shortcuts"');
  }
  const header = key('ai.header', 'header', `<div style="flex:1;min-width:0">${key('ai.history.toggle', 'button', key('ai.title', 'span', 'Alpha') + svg(18), 'class="title"')}</div><div class="header-actions">${key('ai.new', 'button', svg(16))}${key('ai.more', 'button', svg(16))}${key('ai.close', 'button', svg(16))}</div>`, 'class="header"');
  const transcript = state === 'ai-welcome' ? key('ai.welcome', 'div', `<div class="suggestions">${prompts.map((prompt, i) => key(`ai.suggestion.${i}`, 'button', prompt, 'class="suggestion"')).join('')}</div>`, 'class="welcome"') : `<div class="messages">${['Hello', 'Hello back'].map((text, i) => key(`ai.message.${i}`, 'div', key(`ai.message.${i}.body`, 'div', `<div class="content"><p>${text}</p></div>`, 'class="bubble"') + '<div class="actions"></div>', `class="message ${i === 0 ? 'human' : ''}"`)).join('')}</div>`;
  const composer = `<div class="composer-outer">${key('ai.composer', 'div', key('ai.input', 'div', '&nbsp;', 'class="input"') + key('ai.controls', 'div', '<span>Claude Sonnet</span><span class="scope">Alpha</span>' + key('ai.send', 'button', svg(16)), 'class="controls"'), 'class="composer"')}</div>`;
  return key('ai.panel', 'aside', `<div class="inner">${header}${key('ai.transcript', 'div', transcript, 'class="transcript"')}${composer}</div>`, 'class="panel"');
};
function compare(web, native) {
  const actual = new Map(native.map(element => [element.key, element]));
  const differences = [];
  for (const live of web) {
    const app = actual.get(live.key);
    if (!app) { differences.push({ key: live.key, property: 'presence', live, app: null }); continue; }
    const fields = ['x', 'y', 'w', 'h'];
    const box = fields.filter(field => Math.abs(live[field] - app[field]) > 1.5);
    if (box.length) differences.push({ key: live.key, property: 'box', fields: box, live, app });
    for (const field of ['color', 'bg', 'fontSize', 'fontWeight', 'lineHeight', 'radius', 'borderColor']) {
      if (live[field] == null || (app[field] == null && !['bg', 'borderColor'].includes(field))) continue;
      if (typeof live[field] === 'number' ? Math.abs(live[field] - app[field]) > 0.5 : live[field] !== app[field]) differences.push({ key: live.key, property: field, live: live[field], app: app[field] });
    }
  }
  return differences;
}
const browser = await chromium.launch({ headless: true });
const report = {};
mkdirSync(`${out}/panels-web`, { recursive: true });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  for (const state of ['ai-welcome', 'ai-messages', 'shortcuts']) {
    await page.setContent(source.replace('<body></body>', `<body>${html(state)}</body>`));
    await page.addStyleTag({ content: `@font-face { font-family:WebPlex;src:url(data:font/ttf;base64,${font.toString('base64')});font-weight:400; } @font-face { font-family:WebPlex;src:url(data:font/ttf;base64,${medium.toString('base64')});font-weight:500; }` });
    await page.evaluate(async () => { await document.fonts.load('400 16px WebPlex'); await document.fonts.load('500 14px WebPlex'); await document.fonts.ready; });
    const web = await page.evaluate(() => [...document.querySelectorAll('[data-key]')].map(element => {
      const rect = element.getBoundingClientRect(); const s = getComputedStyle(element);
      const hex = color => {
        if (color === 'rgba(0, 0, 0, 0)') return null;
        const rgb = color.match(/\d+/g); return '#' + rgb.slice(0, 3).map(n => Number(n).toString(16).padStart(2, '0')).join('');
      };
      return { key: element.dataset.key, x: rect.x, y: rect.y, w: rect.width, h: rect.height, color: hex(s.color), bg: hex(s.backgroundColor), fontSize: parseFloat(s.fontSize), fontWeight: Number(s.fontWeight), lineHeight: parseFloat(s.lineHeight), radius: parseFloat(s.borderTopLeftRadius), borderColor: parseFloat(s.borderLeftWidth) ? hex(s.borderLeftColor) : null, text: element.textContent.slice(0, 40) };
    }));
    const native = JSON.parse(readFileSync(`${out}/panels-native/${state}.json`, 'utf8'));
    assert.equal(native.width, 1440); assert.equal(native.height, 900);
    writeFileSync(`${out}/panels-web/${state}.json`, JSON.stringify({ width: 1440, height: 900, elements: web }, null, 2) + '\n');
    const differences = compare(web, native.elements);
    report[state] = { measured: web.length, differences: differences.length, records: differences };
    const missing = compare(web, native.elements.filter(element => element.key !== web[0].key));
    assert.ok(missing.some(record => record.key === web[0].key && record.property === 'presence'));
    const perturbed = structuredClone(web); perturbed[0].x += 4;
    assert.equal(compare(web, perturbed).length, 1);
    console.log(`LAYOUT_PANELS_NEGATIVE_CONTROLS_OK ${state}`);
    console.log(`PANEL ${state}: ${differences.length} differences; ${web.length} measured records`);
  }
} finally { await browser.close(); }
report.total = Object.values(report).reduce((sum, state) => sum + state.differences, 0);
report.sourceProjectionSha256 = createHash('sha256').update(source).digest('hex');
report.coverage = 'Graphite source projection, not authenticated capture. Welcome and delivered plain messages; first nine shortcut rows. Geometry/style only, not icon paths or full live parity.';
writeFileSync(`${out}/panel-comparison.json`, JSON.stringify(report, null, 2) + '\n');
writeFileSync(`${out}/compare-panels.txt`, Object.entries(report).filter(([, value]) => value?.differences !== undefined).map(([state, value]) => `${state} ${value.differences}`).join('\n') + `\nTOTAL ${report.total}\n`);
console.log(`LAYOUT_PANELS_TOTAL ${report.total}`);
