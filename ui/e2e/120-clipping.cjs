// Ce que rognent les zones qui défilent : l'anneau du champ de nom d'un élément dans la pile d'un pack (widgets :
// la pile seule dans la colonne ; layouts : dans la section repliable),
// l'ombre du dock de l'agent dans le fil. Sans serveur : les vraies feuilles de styles sur un DOM
// reconstitué. Lancer : bun e2e/120-clipping.cjs.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const path = require('node:path');
const { APP_SHEETS, sheets } = require('./lib/css.cjs');

const css = sheets(...APP_SHEETS, 'glass.css', path.join('a2ui', 'a2ui.css'));
const html = `<!doctype html><style>${css}</style>
<div style="display:flex;gap:40px;height:600px">
  <div class="pile-column" id="widgets" style="width:236px;height:200px">
    <div class="pack-pile"><form class="pack-pile-name"><input class="select" value=""></form><nav class="pile"></nav></div>
  </div>
  <div class="pile-column" id="layouts" style="width:236px;height:200px">
    <section class="pile-section pile-thumbs" data-open="true"><div class="pile-section-head"><h3>Layouts</h3></div>
      <div class="pile-section-body"><div class="pack-pile"><form class="pack-pile-name"><input class="select" value=""></form><nav class="pile"></nav></div></div>
    </section>
  </div>
  <aside class="notes" style="width:480px;height:500px;position:relative">
    <section class="agent-thread"><section class="agent-dock agent-keys"><div style="height:120px"></div></section></section>
  </aside>
</div>`;

/** Étendue d'une ombre portée (hors `inset`) de chaque côté. */
function reach(boxShadow) {
  const ext = { top: 0, right: 0, bottom: 0, left: 0 };
  for (const layer of boxShadow.split(/,(?![^(]*\))/)) {
    if (/inset/.test(layer) || layer.trim() === 'none') continue;
    const [x, y, blur = 0, spread = 0] = (layer.replace(/\([^)]*\)/g, '').match(/-?[\d.]+px/g) ?? []).map(parseFloat);
    const r = blur + spread;
    ext.top = Math.max(ext.top, r - y);
    ext.bottom = Math.max(ext.bottom, r + y);
    ext.left = Math.max(ext.left, r - x);
    ext.right = Math.max(ext.right, r + x);
  }
  return ext;
}

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1000, height: 700 } });
  await p.setContent(html);
  /** The focused name field of column `column` and the clipping box (padding box) of that column. */
  const measure = (column) => p.evaluate((id) => {
    const box = (el) => {
      const r = el.getBoundingClientRect();
      return { top: r.top + el.clientTop, left: r.left + el.clientLeft, right: r.left + el.clientLeft + el.clientWidth, bottom: r.top + el.clientTop + el.clientHeight };
    };
    const rect = (el) => el.getBoundingClientRect().toJSON();
    const input = document.querySelector(`#${id} input`);
    input.focus();
    const dock = document.querySelector('.agent-dock');
    return {
      input: rect(input), inputShadow: getComputedStyle(input).boxShadow, body: box(document.getElementById(id)),
      dock: rect(dock), dockShadow: getComputedStyle(dock).boxShadow, notes: box(document.querySelector('.notes')),
    };
  }, column);
  const all = [];
  for (const column of ['widgets', 'layouts']) all.push([column, await measure(column)]);
  await b.close();

  for (const [column, c] of all) {
    const ring = reach(c.inputShadow);
    assert.ok(ring.top > 0, `${column} : champ sans anneau de focus : ${c.inputShadow}`);
    assert.ok(c.input.top - ring.top >= c.body.top - 0.5, `${column} : anneau du champ rogné en haut (${c.input.top - ring.top} < ${c.body.top})`);
    assert.ok(c.input.left - ring.left >= c.body.left - 0.5, `${column} : anneau du champ rogné à gauche (${c.input.left - ring.left} < ${c.body.left})`);
    assert.ok(c.input.right + ring.right <= c.body.right + 0.5, `${column} : anneau du champ rogné à droite (${c.input.right + ring.right} > ${c.body.right})`);
  }

  const m = all[0][1];
  const shade = reach(m.dockShadow);
  assert.ok(m.dock.left - shade.left >= m.notes.left - 0.5, `ombre du dock rognée à gauche (${m.dock.left - shade.left} < ${m.notes.left})`);
  assert.ok(m.dock.right + shade.right <= m.notes.right + 0.5, `ombre du dock rognée à droite (${m.dock.right + shade.right} > ${m.notes.right})`);
  console.log('clipping ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
