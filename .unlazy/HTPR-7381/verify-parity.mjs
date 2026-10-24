import assert from 'node:assert/strict';
import fs from 'node:fs';
const base = '/home/valentin/projects/hypertask-desktop-wt-public/internal';
const round = fs.readFileSync('.unlazy/HTPR-7381/current-round', 'utf8').trim();
const screen = name => {
  const load = dir => new Map(JSON.parse(fs.readFileSync(`${base}/${dir}/${name}.json`, 'utf8')).elements.map(e => [e.key, e]));
  return [load('live-graphite-1920'), load(round)];
};
function compare(name, keys, fields = ['x', 'y', 'w', 'h']) {
  const [live, native] = screen(name);
  for (const key of keys) {
    assert(native.has(key), `${name}/${key} missing`);
    for (const field of fields) {
      const a = live.get(key)[field], b = native.get(key)[field];
      if (typeof a === 'number') assert(Math.abs(a - b) <= 1.5, `${name}/${key}.${field}: live ${a}, native ${b}`);
      else assert.equal(b, a, `${name}/${key}.${field}`);
    }
    console.log(`${name}/${key}: ${fields.map(f => `${f}=${native.get(key)[f]}`).join(' ')}`);
  }
}
switch (process.argv[2]) {
  case 'board':
    compare('board', ['board.header', 'board.column.0', 'board.column.1', 'board.column.2', 'board.card.0', 'board.card.1']);
    break;
  case 'panel':
    for (const name of ['inbox', 'board', 'task']) compare(name, ['rightpanel'], ['x', 'y', 'w', 'h', 'bg']);
    break;
  case 'task':
    compare('task', ['task.header', 'task.title', 'task.description', 'task.composer', ...Array.from({length:6}, (_, i) => `task.meta.${i}`)]);
    compare('task', ['task.description'], ['radius']);
    break;
  case 'inbox':
    compare('inbox', ['inbox.tabs', 'inbox.section', 'inbox.row.0', 'inbox.row.0.actions']);
    compare('inbox', Array.from({length:5}, (_, i) => `inbox.tab.${i}`), ['y', 'h', 'color', 'fontWeight']);
    break;
  case 'small':
    compare('board', ['board.title'], ['fontWeight']);
    compare('board', ['board.count'], ['color']);
    for (const name of ['inbox', 'board', 'task']) compare(name, Array.from({length:5}, (_, i) => `rail.icon.${i}`));
    compare('board', ['rail.icon.3'], ['color']);
    compare('inbox', ['rail.icon.2'], ['color']);
    break;
  default: throw Error('unknown acceptance gate');
}
console.log(`PARITY_ACCEPTED ${process.argv[2]} ${round}`);
