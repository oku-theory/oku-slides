// Code d'un widget à droite de l'aperçu, élargi au séparateur, retrouvé après rechargement ;
// faute de largeur, il repasse dessous. Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = '.widget-tab:not([hidden])';

/** Boîtes de l'aperçu et du code du sous-onglet visible, place et axe affichés. */
const boxes = (p) => p.evaluate((sel) => {
  const tab = document.querySelector(sel);
  const r = (el) => { const b = el.getBoundingClientRect(); return { left: b.left, right: b.right, top: b.top, bottom: b.bottom, width: b.width }; };
  return {
    place: tab.dataset.place,
    axis: getComputedStyle(tab).getPropertyValue('--code-axis').trim() || 'below',
    side: r(tab.querySelector('.widget-tab-side')),
    code: r(tab.querySelector('.widget-tab-code')),
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
    await p.getByRole('menuitem', { name: 'Éditer les widgets' }).click();
  }
  await add.click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('cadran');
  await name.press('Enter');
  await p.waitForSelector(`${TAB} .widget-tab-side`);

  // Code affiché, puis à droite : l'aperçu à gauche, le code à sa droite.
  await p.getByRole('button', { name: 'Code source' }).click();
  await p.getByRole('button', { name: 'Code à droite' }).click();
  await p.waitForSelector(`${TAB}[data-place="right"] .code-divider[data-axis="right"]`);
  let r = await boxes(p);
  assert.equal(r.axis, 'right');
  assert.ok(r.code.left >= r.side.right, `code pas à droite de l'aperçu : ${JSON.stringify(r)}`);

  // Glisser le séparateur de 120 px vers la gauche élargit le code d'autant.
  const before = r.code.width;
  const d = await p.locator(`${TAB} .code-divider`).boundingBox();
  await p.mouse.move(d.x + d.width / 2, d.y + d.height / 2);
  await p.mouse.down();
  await p.mouse.move(d.x + d.width / 2 - 120, d.y + d.height / 2, { steps: 6 });
  await p.mouse.up();
  r = await boxes(p);
  assert.ok(Math.abs(r.code.width - before - 120) <= 2, `largeur ${r.code.width}, attendue ${before + 120}`);
  const wide = r.code.width;
  // La sauvegarde chez l'hôte part après le rendu.
  await p.waitForTimeout(500);

  // Rechargé : même place, même largeur.
  await p.reload();
  await p.waitForSelector(`${TAB}[data-place="right"] .code-divider`);
  r = await boxes(p);
  assert.ok(Math.abs(r.code.width - wide) <= 2, `largeur reprise ${r.code.width}, attendue ${wide}`);

  // Fenêtre étroite : le choix reste « à droite », le code s'affiche dessous.
  await p.setViewportSize({ width: 1100, height: 900 });
  await p.waitForFunction((sel) => !getComputedStyle(document.querySelector(sel)).getPropertyValue('--code-axis').trim(), TAB);
  r = await boxes(p);
  assert.equal(r.place, 'right');
  assert.ok(r.code.top >= r.side.bottom, `code pas sous l'aperçu : ${JSON.stringify(r)}`);
  // L'axe du séparateur suit au redimensionnement suivant (ResizeObserver).
  await p.waitForSelector(`${TAB} .code-divider[aria-orientation="horizontal"]`, { timeout: 3000 });

  assert.deepEqual(errors, []);
  console.log('disposition du code ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
