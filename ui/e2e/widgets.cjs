// Réglages d'essai d'un widget au dock : changer un paramètre change le contenu de l'aperçu.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const IMG = '.widget-tab:not([hidden]) .widget-tab-side img';
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
  await p.getByRole('menuitem', { name: 'Éditer les widgets' }).click();
  await p.getByRole('button', { name: 'Nouveau widget' }).click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('carte');
  await name.press('Enter');
  await p.waitForSelector(IMG);
  // L'aperçu se stabilise (rendu initial, puis valeurs d'essai) avant la mesure.
  let before = await svgOf(p);
  let settled = false;
  for (let again = 0; again < 20 && !settled; again++) {
    await p.waitForTimeout(400);
    const now = await svgOf(p);
    settled = now === before;
    before = now;
  }
  assert.ok(settled, "l'aperçu ne se stabilise pas");
  // Sous l'aperçu du widget, pas au dock de l'agent.
  assert.equal(await p.locator('.widget-tab:not([hidden]) .widget-tab-side [data-surface-id="host-widget"]').count(), 1);
  assert.equal(await p.locator('.notes [data-surface-id="host-widget"]').count(), 0);
  const field = p.locator('[data-surface-id="host-widget"] input').first();
  await field.fill('Météo du jour');
  await field.press('Enter');
  await p.waitForFunction(async ([sel, was]) => {
    const img = document.querySelector(sel);
    return img && (await (await fetch(img.src)).text()) !== was;
  }, [IMG, before]);
  assert.equal(await p.locator('[data-surface-id="host-widget"] input').first().inputValue(), 'Météo du jour');
  assert.deepEqual(errors, []);
  console.log('réglages du widget ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
