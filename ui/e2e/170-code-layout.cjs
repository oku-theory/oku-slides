// Code d'un widget sous l'aperçu, à droite des réglages, élargi au séparateur, retrouvé après
// rechargement ; dans une fenêtre étroite, les réglages gardent leur place minimale, dans l'onglet de pack d'Essentiel.
// Writes: Essentiel (the `cadran` widget). Leaves: no pack tab. Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openEssentiel } = require('./lib/atelier.cjs');
const { untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = '.widget-tab:not([hidden])';

/** Boîtes de l'aperçu, des réglages et du code du sous-onglet visible. */
const boxes = (p) => p.evaluate((sel) => {
  const tab = document.querySelector(sel);
  const r = (el) => { const b = el.getBoundingClientRect(); return { left: b.left, right: b.right, top: b.top, bottom: b.bottom, width: b.width }; };
  return {
    preview: r(tab.querySelector('.widget-tab-preview')),
    side: r(tab.querySelector('.widget-tab-side')),
    code: r(tab.querySelector('.widget-tab-code')),
    overflow: tab.scrollWidth - tab.clientWidth,
  };
}, TAB);

/** Boîte du séparateur, relue tant qu'elle est nulle : il se remonte quand la disposition change (t-1108). */
const dividerBox = (p) =>
  untilValue(p, () => p.locator(`${TAB} .code-divider`).boundingBox(), (box) => box !== null, 'séparateur du code sans boîte');

/** Réponse de l'hôte à la sauvegarde de l'état d'interface dont la largeur du code vérifie `ok` :
 *  la sauvegarde part après le rendu et peut tarder sur une machine chargée (t-668). */
const saved = (p, ok) =>
  p.waitForResponse((res) => {
    const q = res.request();
    if (q.method() !== 'POST') return false;
    try {
      const { req } = JSON.parse(q.postData() ?? '');
      return req?.cmd === 'ui' && ok(req.code_width);
    } catch {
      return false;
    }
  });

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');
  await openEssentiel(p);
  await p.getByRole('button', { name: 'Nouveau widget' }).click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('cadran');
  await name.press('Enter');
  await p.waitForSelector(`${TAB} .widget-tab-preview`);

  // Code affiché : l'aperçu en haut sur toute la largeur ; dessous, les réglages puis le code à leur droite.
  await p.getByRole('button', { name: 'Code source' }).click();
  await p.waitForSelector(`${TAB}[data-code="shown"] .code-divider[aria-orientation="vertical"]`);
  let r = await boxes(p);
  assert.ok(r.code.top >= r.preview.bottom && r.side.top >= r.preview.bottom, `pas sous l'aperçu : ${JSON.stringify(r)}`);
  assert.ok(r.code.left >= r.side.right, `code pas à droite des réglages : ${JSON.stringify(r)}`);
  assert.ok(r.preview.right >= r.code.right - 1, `l'aperçu ne couvre pas la largeur : ${JSON.stringify(r)}`);

  // Glisser le séparateur de 60 px vers la gauche élargit le code d'autant (les réglages gardent leurs 240 px).
  const before = r.code.width;
  const d = await dividerBox(p);
  await p.mouse.move(d.x + d.width / 2, d.y + d.height / 2);
  await p.mouse.down();
  await p.mouse.move(d.x + d.width / 2 - 60, d.y + d.height / 2, { steps: 6 });
  const widened = saved(p, (w) => w !== null && Math.abs(w - before - 60) <= 2);
  await p.mouse.up();
  r = await boxes(p);
  assert.ok(Math.abs(r.code.width - before - 60) <= 2, `largeur ${r.code.width}, attendue ${before + 60}`);
  const wide = r.code.width;
  // Recharger avant que l'hôte ait gardé la largeur la perdrait.
  assert.ok((await widened).ok(), 'largeur non gardée par l\'hôte');

  // Rechargé : même largeur.
  await p.reload();
  await p.waitForSelector(`${TAB}[data-code="shown"] .code-divider`);
  r = await boxes(p);
  assert.ok(Math.abs(r.code.width - wide) <= 2, `largeur reprise ${r.code.width}, attendue ${wide}`);

  // Fenêtre étroite, code tiré au plus large : les réglages gardent leurs 240 px (SIDE_MIN), rien ne déborde.
  await p.setViewportSize({ width: 1100, height: 900 });
  const n = await dividerBox(p);
  await p.mouse.move(n.x + n.width / 2, n.y + n.height / 2);
  await p.mouse.down();
  await p.mouse.move(0, n.y + n.height / 2, { steps: 6 });
  await p.mouse.up();
  r = await boxes(p);
  assert.ok(r.side.width >= 240 - 1, `réglages trop étroits : ${r.side.width}`);
  assert.ok(r.overflow <= 0, `le sous-onglet déborde de ${r.overflow} px`);
  // Retour à la proportion par défaut, pour les tests suivants.
  const reset = saved(p, (w) => w === null);
  await p.locator(`${TAB} .code-divider`).dblclick();
  assert.ok((await reset).ok(), 'proportion par défaut non gardée par l\'hôte');

  await closePackTabs(p);
  assert.deepEqual(errors, []);
  console.log('disposition du code ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
