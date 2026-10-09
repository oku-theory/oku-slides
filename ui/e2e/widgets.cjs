// Réglages d'essai d'un widget au dock : changer un paramètre change le contenu de l'aperçu.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { until, settled } = require('./until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const IMG = '.widget-tab:not([hidden]) .widget-tab-preview img';
const svgOf = (p) => p.evaluate(async (sel) => {
  const img = document.querySelector(sel);
  return img ? (await fetch(img.src)).text() : null;
}, IMG);
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Atelier' }).click();
  await p.getByRole('button', { name: 'Nouveau widget' }).click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('carte');
  await name.press('Enter');
  await p.waitForSelector(IMG);
  // Les réglages d'essai sont chargés (leur surface est là), puis l'aperçu se stabilise (rendu initial,
  // puis valeurs d'essai) avant la mesure : relevés à 400 ms, plus que l'écart entre ces deux rendus.
  await p.waitForSelector('[data-surface-id="host-widget"] input');
  const before = await settled(p, () => svgOf(p), "l'aperçu ne se stabilise pas", 10000, 400);
  // Sous l'aperçu du widget, pas au dock de l'agent.
  assert.equal(await p.locator('.widget-tab:not([hidden]) .widget-tab-side [data-surface-id="host-widget"]').count(), 1);
  assert.equal(await p.locator('.notes [data-surface-id="host-widget"]').count(), 0);
  const field = p.locator('[data-surface-id="host-widget"] input').first();
  await field.fill('Météo du jour');
  await field.press('Enter');
  await until(p, async () => {
    const now = await svgOf(p);
    return now !== null && now !== before;
  }, "l'aperçu ne change pas après le réglage");
  assert.equal(await p.locator('[data-surface-id="host-widget"] input').first().inputValue(), 'Météo du jour');
  assert.deepEqual(errors, []);
  console.log('réglages du widget ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
