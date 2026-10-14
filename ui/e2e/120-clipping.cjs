// Ce que rognent les zones qui défilent : l'anneau du champ de nom d'un widget dans la pile,
// l'ombre du dock de l'agent dans le fil. Sans serveur : les vraies feuilles de styles sur un DOM
// reconstitué. Lancer : bun e2e/120-clipping.cjs.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const css = ['styles.css', 'glass.css', path.join('a2ui', 'a2ui.css')]
  .map((f) => fs.readFileSync(path.join(__dirname, '..', 'src', f), 'utf8'))
  .join('\n');
const html = `<!doctype html><style>${css}</style>
<div style="display:flex;gap:40px;height:600px">
  <div class="pile-section pile-widgets" style="width:236px;height:200px">
    <div class="pile-section-body"><form class="pile-widgets-name"><input id="name" class="select" value=""></form><ul><li>Commun</li></ul></div>
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
  await p.focus('#name');
  const m = await p.evaluate(() => {
    const box = (el) => {
      const r = el.getBoundingClientRect();
      return { top: r.top + el.clientTop, left: r.left + el.clientLeft, right: r.left + el.clientLeft + el.clientWidth, bottom: r.top + el.clientTop + el.clientHeight };
    };
    const rect = (el) => el.getBoundingClientRect().toJSON();
    const input = document.querySelector('#name');
    const dock = document.querySelector('.agent-dock');
    return {
      input: rect(input), inputShadow: getComputedStyle(input).boxShadow, body: box(document.querySelector('.pile-section-body')),
      dock: rect(dock), dockShadow: getComputedStyle(dock).boxShadow, notes: box(document.querySelector('.notes')),
    };
  });
  await b.close();

  const ring = reach(m.inputShadow);
  assert.ok(ring.top > 0, `champ sans anneau de focus : ${m.inputShadow}`);
  assert.ok(m.input.top - ring.top >= m.body.top - 0.5, `anneau du champ rogné en haut (${m.input.top - ring.top} < ${m.body.top})`);
  assert.ok(m.input.left - ring.left >= m.body.left - 0.5, `anneau du champ rogné à gauche (${m.input.left - ring.left} < ${m.body.left})`);

  const shade = reach(m.dockShadow);
  assert.ok(m.dock.left - shade.left >= m.notes.left - 0.5, `ombre du dock rognée à gauche (${m.dock.left - shade.left} < ${m.notes.left})`);
  assert.ok(m.dock.right + shade.right <= m.notes.right + 0.5, `ombre du dock rognée à droite (${m.dock.right + shade.right} > ${m.notes.right})`);
  console.log('clipping ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
