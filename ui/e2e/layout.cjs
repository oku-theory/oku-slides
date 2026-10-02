// Trois colonnes : pile, feuille, fil de l'agent avec la dictée ; aucune ne chevauche sa voisine,
// et le fil (surfaces a2ui) occupe une large part de la table.
// Lancer : just e2e (serveur neuf, port 4872). À la main : bun e2e/layout.cjs, E2E_URL = adresse du serveur.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const columns = (p) => p.evaluate(() => {
  const box = (s) => document.querySelector(s).getBoundingClientRect();
  return { pile: box('.pile').right, stage: box('.stage'), notes: box('.notes').left, notesWidth: box('.notes').width, device: box('.device').left };
});
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  // La scène est mise à l'échelle au chargement, puis la fenêtre rétrécit : elle doit suivre.
  for (const [width, height] of [[1440, 900], [1024, 640], [1000, 700]]) {
    await p.setViewportSize({ width, height });
    await p.waitForTimeout(300);
    const c = await columns(p);
    assert.ok(c.stage.left >= c.pile, `${width} px : la feuille chevauche la pile (${c.stage.left} < ${c.pile})`);
    assert.ok(c.stage.right <= c.notes, `${width} px : la feuille chevauche le fil (${c.stage.right} > ${c.notes})`);
    assert.ok(c.notesWidth >= Math.min(420, width * 0.4) - 1, `${width} px : fil trop étroit (${c.notesWidth})`);
    assert.ok(c.stage.right <= c.device, `${width} px : la feuille chevauche la dictée (${c.stage.right} > ${c.device})`);
    // Le titre du document : discret (ni fond ni ombre), centré au-dessus de la slide.
    const label = await p.evaluate(() => {
      const el = document.querySelector('.label');
      const r = el.getBoundingClientRect();
      const s = getComputedStyle(el);
      const st = document.querySelector('.stage').getBoundingClientRect();
      const name = el.querySelector('.doc-name');
      return {
        center: (r.left + r.right) / 2, stage: (st.left + st.right) / 2, shadow: s.boxShadow, bg: s.backgroundColor,
        clipped: name.scrollWidth > name.clientWidth,
      };
    });
    assert.ok(!label.clipped, `${width} px : titre rogné alors qu'il y a la place`);
    assert.ok(Math.abs(label.center - label.stage) <= 2, `${width} px : titre décentré (${label.center} ≠ ${label.stage})`);
    assert.equal(label.shadow, 'none');
    assert.equal(label.bg, 'rgba(0, 0, 0, 0)');
  }
  assert.deepEqual(errors, []);
  console.log('colonnes ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
