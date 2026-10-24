import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { chromium } from 'playwright';

// src/lib/fonts/ibmPlexSans.ts:3; src/styles/tailwindThemes/graphite.css:171-172.
const url = 'https://app.hypertask.ai/_next/static/media/26d4368bf94c0ec4-s.woff2';
const response = await fetch(url);
assert.equal(response.status, 200);
const font = Buffer.from(await response.arrayBuffer());
const digest = createHash('sha256').update(font).digest('hex');
assert.equal(digest, '056e4e2459f57a0033c8c9c844ff19d6e42ac8602027803d4345823bcc939818');
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1009 }, deviceScaleFactor: 1 });
  // BottomSettings_QuickTips.tsx:413-415; inboxSplit/SplitTitle.tsx:32-54;
  // tailwind.config.ts:167-178. No app, authenticated API or laptop is used.
  await page.setContent(`<!doctype html><style>
    @font-face { font-family: WebPlex; src: url(data:font/woff2;base64,${font.toString('base64')}); font-weight: 100 900; }
    body { font-family: WebPlex; font-size: 16px; line-height: 1.5; }
    #footer { display: inline-block; font-size: 14px; line-height: 21px; font-weight: 400; }
    .tab { display: inline-flex; height: 32px; padding-right: 15px; align-items: center; gap: 4px; font-size: 14px; }
    .label { display: flex; align-items: baseline; gap: 4px; }
    .selected { font-weight: 600; }
    p { font-size: 11px; font-weight: 400; margin: 0; }
  </style><span id="footer">Command center</span>
  <div id="important" class="tab"><div class="label selected"><span>Important</span><p>1</p></div></div>
  <div id="agents" class="tab"><div class="label"><span>Agents</span><p>1</p></div></div>`);
  await page.evaluate(async () => {
    await document.fonts.load('400 14px WebPlex');
    await document.fonts.load('600 14px WebPlex');
    await document.fonts.ready;
  });
  const web = await page.evaluate(() => Object.fromEntries(
    ['footer', 'important', 'agents'].map(id => [id, document.getElementById(id).getBoundingClientRect().width]),
  ));
  const diagnostics = await page.evaluate(async fonts => {
    for (const [family, weights] of Object.entries(fonts)) {
      for (const [weight, data] of Object.entries(weights)) {
        const face = new FontFace(family, `url(data:font/ttf;base64,${data})`, { weight });
        document.fonts.add(await face.load());
      }
    }
    const measure = () => Object.fromEntries(['footer', 'important', 'agents'].map(id => [id, document.getElementById(id).getBoundingClientRect().width]));
    const result = {};
    result.glyphs = {};
    const canvas = document.createElement('canvas').getContext('2d');
    for (const family of ['WebPlex', 'WebFullStatic', 'WebLatinStatic']) {
      document.body.style.fontFamily = family;
      result[family] = measure();
      result.glyphs[family] = {};
      for (const weight of [400, 600]) {
        canvas.font = `${weight} 14px ${family}`;
        result.glyphs[family][weight] = Object.fromEntries(
          [...new Set('Command centerImportantAgents')].map(char => [char, canvas.measureText(char).width]),
        );
      }
    }
    result.keycaps = {};
    for (const [name, family] of [
      ['embedded', 'WebMono'],
      ['source', 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace'],
    ]) {
      canvas.font = `600 12px ${family}`;
      result.keycaps[name] = Object.fromEntries(['CTRL', 'K', 'Enter', 'TAB', 'N / Shift+C'].map(text => [text, canvas.measureText(text).width]));
    }
    document.body.style.fontFamily = 'WebPlex';
    document.body.style.fontKerning = 'none';
    result.withoutKerning = measure();
    return result;
  }, { ...Object.fromEntries([
    ['WebFullStatic', 'evidence/web-fonts'],
    ['WebLatinStatic', 'evidence/web-fonts/latin-probe'],
  ].map(([family, path]) => [family, Object.fromEntries([400, 600].map(weight => [weight, readFileSync(`${path}/IBMPlexSans-${weight}.ttf`).toString('base64')]))])),
  WebMono: { 600: readFileSync('crates/ht-desktop/assets/fonts/LiberationMono-Bold.ttf').toString('base64') },
  });
  const inbox = JSON.parse(readFileSync('evidence/layout/parity/inbox.json', 'utf8'));
  const elements = Object.fromEntries(inbox.elements.map(e => [e.key, e]));
  const probes = [
    ['footer', 'footer.text', 106],
    ['important', 'inbox.tab.0', 96],
    ['agents', 'inbox.tab.1', 72],
  ].map(([id, key, frozenWidth]) => ({ key, webWidth: web[id], nativeWidth: elements[key].w, frozenWidth, fullStaticWidth: diagnostics.WebFullStatic[id], latinStaticWidth: diagnostics.WebLatinStatic[id], withoutKerningWidth: diagnostics.withoutKerning[id] }));
  const receipt = { font: url, digest, width: 1920, height: 1009, scale: 1, probes, diagnostics };
  writeFileSync('evidence/layout/font-metrics.json', JSON.stringify(receipt, null, 2) + '\n');
  for (const probe of probes) console.log(`FONT_METRIC ${JSON.stringify(probe)}`);
  for (const probe of probes) {
    assert.ok(Math.abs(probe.fullStaticWidth - probe.nativeWidth) <= 1.5, JSON.stringify(probe));
    // Equivalent full-face agreement cannot waive the actual live-web contract.
    assert.ok(Math.abs(probe.webWidth - probe.nativeWidth) <= 1.5, JSON.stringify(probe));
  }
  console.log('LAYOUT_WEB_FONT_METRICS_OK');
} finally {
  await browser.close();
}
