// Deux colonnes : la feuille (les miniatures en bande dessous) et le fil de l'agent avec la dictée ;
// aucune ne chevauche sa voisine, et le fil (surfaces a2ui) occupe une large part de la table.
// Lancer : just e2e (serveur neuf, port 4872). À la main : bun e2e/090-layout.cjs, E2E_URL = adresse du serveur.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until, stable } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const COLUMNS = ['.slide-strip', '.stage', '.notes', '.device'];
const hostNotes = async (p) => (await post(p, '/api', { tab: 't-1', req: { cmd: 'snapshot' } })).ui.notes; // null : largeur par défaut
const columns = (p) => p.evaluate(() => {
  const box = (s) => document.querySelector(s).getBoundingClientRect();
  return { left: box('.table').left, strip: box('.slide-strip'), stage: box('.stage'), notes: box('.notes').left, notesWidth: box('.notes').width, device: box('.device').left };
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
    // La mise à l'échelle de la scène suit le rendu React, qui peut prendre plus de deux images quand le
    // fil de l'agent est chargé (specs 060 à 080) : `stable` seul la mesurerait encore à l'ancienne taille.
    await until(p, async () => {
      const c = await columns(p);
      return c.stage.left >= c.left && c.stage.right <= c.notes;
    }, `${width} px : la feuille ne suit pas la fenêtre`);
    await stable(p, COLUMNS);
    const c = await columns(p);
    assert.ok(c.stage.left >= c.left, `${width} px : la feuille déborde à gauche (${c.stage.left} < ${c.left})`);
    // La bande des miniatures sous la slide, dans la feuille.
    assert.ok(c.strip.top >= c.stage.bottom, `${width} px : la bande des miniatures chevauche la slide (${c.strip.top} < ${c.stage.bottom})`);
    assert.ok(c.strip.right <= c.notes, `${width} px : la bande des miniatures chevauche le fil (${c.strip.right} > ${c.notes})`);
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
    // Le fil monte jusqu'en haut de la table (t-810) ; le menu est dans la barre d'outils.
    const top = await p.evaluate(() => {
      const box = (s) => document.querySelector(s).getBoundingClientRect();
      return { table: box('.table').top, notes: box('.notes').top, menu: !!document.querySelector('.tools-tray > .action-menu') };
    });
    assert.ok(Math.abs(top.notes - top.table) <= 1, `${width} px : le fil ne monte pas en haut (${top.notes} ≠ ${top.table})`);
    assert.ok(top.menu, `${width} px : le menu n'est pas dans la barre d'outils`);
  }
  // Séparateurs : glisser celui du fil vers la gauche l'élargit ; la largeur est gardée par l'hôte
  // (rechargement) ; un double-clic la rend par défaut.
  await p.setViewportSize({ width: 1440, height: 900 });
  await stable(p, COLUMNS);
  const notesWidth = () => p.evaluate(() => document.querySelector('.notes').getBoundingClientRect().width);
  const before = await notesWidth();
  // Une bande de prise de 8 px, rien de plus (sans hériter des règles des colonnes) ; sans colonne
  // de gauche hors mode pack, seul le séparateur du fil se montre.
  const widths = await p.evaluate(() => [...document.querySelectorAll('.col-divider')].filter((d) => d.getClientRects().length > 0).map((d) => d.getBoundingClientRect().width));
  assert.deepEqual(widths, [8]);
  const handle = await p.locator('.col-divider[data-side="notes"]').boundingBox();
  const x = handle.x + handle.width / 2;
  const y = handle.y + handle.height / 2;
  await p.mouse.move(x, y);
  await p.mouse.down();
  await p.mouse.move(x - 50, y, { steps: 5 });
  await p.mouse.move(x - 100, y, { steps: 5 });
  await p.mouse.up();
  await stable(p, COLUMNS);
  assert.ok(Math.abs((await notesWidth()) - (before + 100)) <= 1, `fil pas élargi de 100 px (${before} → ${await notesWidth()})`);
  // L'hôte a gardé la largeur (sauvegarde asynchrone) avant qu'on recharge.
  await until(p, async () => Math.abs((await hostNotes(p)) - (before + 100)) <= 1, 'largeur du fil jamais gardée par l\'hôte');
  await p.reload();
  await p.waitForSelector('.stage img');
  // La largeur gardée est reprise de l'hôte après le premier rendu : attendre la reprise, pas une image.
  await until(p, async () => Math.abs((await notesWidth()) - (before + 100)) <= 1, 'largeur du fil non reprise au rechargement');
  assert.ok(Math.abs((await notesWidth()) - (before + 100)) <= 1, `largeur du fil perdue au rechargement (${await notesWidth()})`);
  await p.dblclick('.col-divider[data-side="notes"]');
  // L'hôte garde « par défaut » (null) avant qu'on termine : la sauvegarde ne doit pas arriver après la fermeture.
  await until(p, async () => (await hostNotes(p)) === null, 'largeur par défaut jamais gardée par l\'hôte (double-clic)');
  await stable(p, COLUMNS);
  assert.ok(Math.abs((await notesWidth()) - before) <= 1, `double-clic : fil pas revenu à ${before} (${await notesWidth()})`);
  // La bande des miniatures se cache depuis la barre d'outils : plus de bande, la slide garde ou gagne
  // de la place (fenêtre basse : la hauteur la borne) ; réaffichée pour les scripts suivants.
  await p.setViewportSize({ width: 1440, height: 760 });
  await stable(p, COLUMNS);
  const stageHeight = () => p.evaluate(() => document.querySelector('.stage').getBoundingClientRect().height);
  const shown = await stageHeight();
  await p.getByRole('button', { name: 'Masquer les miniatures', exact: true }).click();
  await p.waitForSelector('.slide-strip', { state: 'detached' });
  await until(p, async () => (await stageHeight()) > shown, `la slide ne gagne pas la place de la bande (${shown})`);
  await p.getByRole('button', { name: 'Afficher les miniatures', exact: true }).click();
  await p.waitForSelector('.slide-strip .thumb');
  assert.deepEqual(errors, []);
  console.log('colonnes ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
