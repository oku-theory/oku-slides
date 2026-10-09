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
    // Slide précédente / suivante : dans la marge de part et d'autre de la scène, à 8 px au moins.
    const nav = await p.evaluate(() => {
      const box = document.querySelector('.stage-box').getBoundingClientRect();
      const next = document.querySelector('.slide-nav.next').getBoundingClientRect();
      const previous = document.querySelector('.slide-nav.previous').getBoundingClientRect();
      return { left: box.left, right: box.right, next: next.left, previous: previous.right };
    });
    assert.ok(nav.next >= nav.right + 8, `${width} px : « suivante » collée à la scène (${nav.next} < ${nav.right} + 8)`);
    assert.ok(nav.previous <= nav.left - 8, `${width} px : « précédente » collée à la scène (${nav.previous} > ${nav.left} - 8)`);
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
  // Séparateurs : glisser celui du fil vers la gauche l'élargit ; la largeur est gardée par l'hôte
  // (rechargement) ; un double-clic la rend par défaut.
  await p.setViewportSize({ width: 1440, height: 900 });
  await p.waitForTimeout(300);
  const notesWidth = () => p.evaluate(() => document.querySelector('.notes').getBoundingClientRect().width);
  const before = await notesWidth();
  // Une bande de prise de 8 px, rien de plus (sans hériter des règles des colonnes).
  const widths = await p.evaluate(() => [...document.querySelectorAll('.col-divider')].map((d) => d.getBoundingClientRect().width));
  assert.deepEqual(widths, [8, 8]);
  const handle = await p.locator('.col-divider[data-side="notes"]').boundingBox();
  const x = handle.x + handle.width / 2;
  const y = handle.y + handle.height / 2;
  await p.mouse.move(x, y);
  await p.mouse.down();
  await p.mouse.move(x - 50, y, { steps: 5 });
  await p.mouse.move(x - 100, y, { steps: 5 });
  await p.mouse.up();
  await p.waitForTimeout(300);
  assert.ok(Math.abs((await notesWidth()) - (before + 100)) <= 1, `fil pas élargi de 100 px (${before} → ${await notesWidth()})`);
  await p.reload();
  await p.waitForSelector('.stage img');
  await p.waitForTimeout(300);
  assert.ok(Math.abs((await notesWidth()) - (before + 100)) <= 1, `largeur du fil perdue au rechargement (${await notesWidth()})`);
  await p.dblclick('.col-divider[data-side="notes"]');
  await p.waitForTimeout(300);
  assert.ok(Math.abs((await notesWidth()) - before) <= 1, `double-clic : fil pas revenu à ${before} (${await notesWidth()})`);
  assert.deepEqual(errors, []);
  console.log('colonnes ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
