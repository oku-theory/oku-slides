// Code d'un widget sous l'aperçu, à droite des réglages, élargi au séparateur, retrouvé après
// rechargement ; dans une fenêtre étroite, les réglages gardent leur place minimale. Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
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

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  // Le test précédent peut avoir laissé l'onglet en mode modèles : on y entre s'il le faut.
  await p.waitForSelector('.stage img, .widget-pane', { state: 'attached' });
  const add = p.getByRole('button', { name: 'Nouveau widget' });
  if (!(await add.isVisible())) {
    await p.getByRole('button', { name: 'Menu' }).click();
    await p.getByRole('menuitem', { name: 'Modèles et widgets' }).click();
  }
  await add.click();
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
  const d = await p.locator(`${TAB} .code-divider`).boundingBox();
  await p.mouse.move(d.x + d.width / 2, d.y + d.height / 2);
  await p.mouse.down();
  await p.mouse.move(d.x + d.width / 2 - 60, d.y + d.height / 2, { steps: 6 });
  await p.mouse.up();
  r = await boxes(p);
  assert.ok(Math.abs(r.code.width - before - 60) <= 2, `largeur ${r.code.width}, attendue ${before + 60}`);
  const wide = r.code.width;
  // La sauvegarde chez l'hôte part après le rendu.
  await p.waitForTimeout(500);

  // Rechargé : même largeur.
  await p.reload();
  await p.waitForSelector(`${TAB}[data-code="shown"] .code-divider`);
  r = await boxes(p);
  assert.ok(Math.abs(r.code.width - wide) <= 2, `largeur reprise ${r.code.width}, attendue ${wide}`);

  // Fenêtre étroite, code tiré au plus large : les réglages gardent leurs 240 px (SIDE_MIN), rien ne déborde.
  await p.setViewportSize({ width: 1100, height: 900 });
  const n = await p.locator(`${TAB} .code-divider`).boundingBox();
  await p.mouse.move(n.x + n.width / 2, n.y + n.height / 2);
  await p.mouse.down();
  await p.mouse.move(0, n.y + n.height / 2, { steps: 6 });
  await p.mouse.up();
  r = await boxes(p);
  assert.ok(r.side.width >= 240 - 1, `réglages trop étroits : ${r.side.width}`);
  assert.ok(r.overflow <= 0, `le sous-onglet déborde de ${r.overflow} px`);
  // Retour à la proportion par défaut, pour les tests suivants.
  await p.locator(`${TAB} .code-divider`).dblclick();
  await p.waitForTimeout(500);

  assert.deepEqual(errors, []);
  console.log('disposition du code ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
