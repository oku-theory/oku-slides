// Panneau de droite (surface A2UI) : renommer un modèle et changer sa grille, puis annuler.
// Contre `slider serve fixtures/roadmap.json`. Lancer : just e2e (serveur neuf, port 4872).
// À la main : bun run e2e dans ui/, E2E_URL = adresse du serveur (défaut port 4870).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const state = (p) => p.evaluate(() => ({
  cards: document.querySelectorAll('.notes .panel-card').length,
  name: document.querySelector('.notes input')?.value,
  grid: [...document.querySelectorAll('.notes .stepper-value')].map((e) => Number(e.textContent)),
  thumb: document.querySelector('.thumb')?.parentElement?.innerText,
}));
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('dialog', (d) => d.accept());
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  // Hors des modèles, le panneau est vide.
  assert.equal((await state(p)).cards, 0);
  await p.getByRole('button', { name: 'Modèles', exact: true }).click();
  await p.waitForSelector('.notes .panel-card');
  const before = await state(p);
  assert.equal(before.cards, 2);
  // Renommer : la vignette suit, l'annulation porte le libellé.
  await p.locator('.notes input').first().fill('Texte A2UI');
  await p.keyboard.press('Enter');
  await p.waitForTimeout(700);
  assert.match((await state(p)).thumb, /Texte A2UI/);
  // Une colonne de moins.
  await p.getByRole('button', { name: 'Colonnes : une de moins' }).click();
  await p.waitForTimeout(700);
  assert.deepEqual((await state(p)).grid, [before.grid[0] - 1, before.grid[1]]);
  // Deux annulations : tout revient.
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(700);
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(700);
  const after = await state(p);
  assert.deepEqual([after.name, after.grid], [before.name, before.grid]);
  // En quittant les modèles, le panneau se vide.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await p.waitForTimeout(700);
  assert.equal((await state(p)).cards, 0);
  assert.deepEqual(errors, []);
  console.log('panneau ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
