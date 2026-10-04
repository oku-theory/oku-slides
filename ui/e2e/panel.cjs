// Panneau de droite (surface A2UI) : renommer un modèle et changer sa grille, puis annuler.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e (serveur neuf, port 4872).
// À la main : bun run e2e dans ui/, E2E_URL = adresse du serveur (défaut port 4870).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
// Modèle « libre » (6e) : sans blocs, il a une carte de grille (les blocs n'en ont pas, t-292).
const LIBRE = 5;
const state = (p) => p.evaluate((LIBRE) => ({
  cards: document.querySelectorAll('.notes .a2ui-card').length,
  name: document.querySelector('.notes input')?.value,
  grid: [...document.querySelectorAll('.notes .stepper-value')].map((e) => Number(e.textContent)),
  thumb: document.querySelectorAll('.thumb')[LIBRE]?.parentElement?.innerText,
}), LIBRE);
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
  // Les modèles s'ouvrent depuis le menu ⋯.
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Modèles' }).click();
  await p.waitForSelector('.notes .a2ui-card');
  await p.click(`.thumb >> nth=${LIBRE}`);
  await p.waitForSelector('.notes .a2ui-card >> nth=1');
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
  // Un widget ouvert, une miniature cliquée : retour au sous-onglet Modèles, sur ce modèle.
  const panelHidden = () => p.evaluate(() => document.getElementById('templates-panel').hidden);
  // Créé, il s'ouvre dans son sous-onglet.
  await p.click('.pile-widgets-head button');
  await p.fill('.pile-widgets-name .select', 'essai-pile');
  await p.keyboard.press('Enter');
  await p.waitForSelector('.widget-tab:not([hidden])');
  assert.equal(await panelHidden(), true);
  await p.click('.thumb >> nth=0');
  await p.waitForTimeout(300);
  assert.equal(await panelHidden(), false);
  assert.equal(await p.locator('.thumb >> nth=0').getAttribute('aria-current'), 'true');
  await p.click('.pile-widgets-row:has-text("essai-pile") .pile-widgets-remove');
  await p.waitForSelector('.pile-widgets-row:has-text("essai-pile")', { state: 'detached' });
  // En quittant les modèles, le panneau se vide.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await p.waitForTimeout(700);
  assert.equal((await state(p)).cards, 0);
  assert.deepEqual(errors, []);
  console.log('panneau ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
