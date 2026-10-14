// Réglages du modèle au dock de l'agent (surface A2UI) : renommer un modèle et changer sa grille, puis annuler.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e (serveur neuf, port 4872).
// À la main : bun run e2e dans ui/, E2E_URL = adresse du serveur (défaut port 4870).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
// Modèle « libre » (6e) : sans blocs, il a une carte de grille (les blocs n'en ont pas, t-292).
const LIBRE = 5;
const state = (p) => p.evaluate((LIBRE) => ({
  cards: document.querySelectorAll('[data-surface-id="host-template"]').length,
  name: document.querySelector('[data-surface-id="host-template"] input')?.value,
  grid: [...document.querySelectorAll('[data-surface-id="host-template"] .stepper-value')].map((e) => Number(e.textContent)),
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
  // Hors des modèles, aucune carte.
  assert.equal((await state(p)).cards, 0);
  // Les modèles s'ouvrent depuis le menu ⋯.
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Atelier' }).click();
  await p.waitForSelector('[data-surface-id="host-template"]');
  // Sous la slide qu'ils règlent, pas au dock de l'agent.
  assert.equal(await p.locator('.templates-panel [data-surface-id="host-template"]').count(), 1);
  assert.equal(await p.locator('.notes [data-surface-id="host-template"]').count(), 0);
  await p.click(`.thumb >> nth=${LIBRE}`);
  await p.waitForSelector('[data-surface-id="host-template"] .stepper-value');
  const before = await state(p);
  assert.equal(before.cards, 1);
  // Renommer : la vignette suit, l'annulation porte le libellé.
  await p.locator('[data-surface-id="host-template"] input').first().fill('Texte A2UI');
  await p.keyboard.press('Enter');
  await untilValue(p, () => state(p), (s) => /Texte A2UI/.test(s.thumb), 'la vignette ne suit pas le nouveau nom');
  // Une colonne de moins.
  await p.getByRole('button', { name: 'Colonnes : une de moins' }).click();
  const fewer = JSON.stringify([before.grid[0] - 1, before.grid[1]]);
  await untilValue(p, () => state(p), (s) => JSON.stringify(s.grid) === fewer, 'la grille ne perd pas une colonne');
  // Deux annulations, dès le clic sur le stepper (le bouton garde le focus) : la grille revient,
  // puis le nom.
  await p.keyboard.press('Control+z');
  const regrid = await untilValue(p, () => state(p), (s) => JSON.stringify(s.grid) === JSON.stringify(before.grid), 'première annulation : la grille ne revient pas');
  assert.equal(regrid.name, 'Texte A2UI', 'la première annulation a défait le renommage');
  await p.keyboard.press('Control+z');
  await untilValue(
    p,
    () => state(p),
    (s) => s.name === before.name && JSON.stringify(s.grid) === JSON.stringify(before.grid),
    'seconde annulation : le nom ne revient pas (ou la grille a changé)',
  );
  // Un widget ouvert, une miniature cliquée : retour au sous-onglet Modèles, sur ce modèle.
  const panelHidden = () => p.evaluate(() => document.getElementById('templates-panel').hidden);
  // Créé, il s'ouvre dans son sous-onglet.
  await p.click('.pile-widgets .pile-section-head .token');
  await p.fill('.pile-widgets-name .select', 'essai-pile');
  await p.keyboard.press('Enter');
  await p.waitForSelector('.widget-tab:not([hidden])');
  assert.equal(await panelHidden(), true);
  await p.click('.thumb >> nth=0');
  await untilValue(p, panelHidden, (hidden) => hidden === false, 'le sous-onglet Modèles ne revient pas');
  assert.equal(await p.locator('.thumb >> nth=0').getAttribute('aria-current'), 'true');
  await p.click('.pile-widgets-row:has-text("essai-pile") .pile-widgets-remove');
  await p.waitForSelector('.pile-widgets-row:has-text("essai-pile")', { state: 'detached' });
  // Onglet « Thème » : les jetons à droite, à la place du fil ; aucun réglage du modèle sous la scène.
  await p.getByRole('tab', { name: 'Thème', exact: true }).click();
  await p.waitForSelector('#theme-panel [data-surface-id="host-theme"] .a2ui-disclosure');
  assert.equal(await p.locator('.notes #theme-panel').count(), 1);
  assert.equal(await p.locator('[data-surface-id="host-template"]').count(), 0);
  assert.equal(await panelHidden(), false);
  assert.equal(await p.getByRole('combobox', { name: 'Thème' }).inputValue(), '2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31');
  // Le filtre de la surface est relu par la session : moins de sections, toutes dépliées.
  const sections = () => p.locator('[data-surface-id="host-theme"] .a2ui-disclosure').count();
  const all = await sections();
  await p.locator('[data-surface-id="host-theme"] input').first().fill('accent');
  await p.keyboard.press('Enter');
  await p.waitForFunction((n) => document.querySelectorAll('[data-surface-id="host-theme"] .a2ui-disclosure').length < n, all);
  // Une miniature cliquée garde l'onglet « Thème ».
  await p.click('.thumb >> nth=1');
  // Point de synchronisation : le clic a bien été traité (la miniature est la courante).
  await untilValue(p, () => p.locator('.thumb >> nth=1').getAttribute('aria-current'), (v) => v === 'true', 'la miniature cliquée ne devient pas la courante');
  // négatif : un retour fautif au sous-onglet Modèles aurait le temps d'arriver.
  await p.waitForTimeout(300);
  assert.equal(await p.getByRole('tab', { name: 'Thème', exact: true }).getAttribute('aria-selected'), 'true');
  // Le menu ⋯ du mode modèles n'a plus de sélecteur de thème.
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.waitForSelector('[role="menu"]');
  assert.equal(await p.locator('[role="menu"] option', { hasText: /^Papier$/ }).count(), 0);
  await p.keyboard.press('Escape');
  await p.getByRole('tab', { name: 'Modèles', exact: true }).click();
  await p.waitForSelector('.templates-panel [data-surface-id="host-template"]');
  assert.equal(await p.locator('#theme-panel').count(), 0);
  // En quittant les modèles, les cartes disparaissent.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await untilValue(p, () => state(p), (s) => s.cards === 0, 'les cartes ne disparaissent pas hors des modèles');
  assert.deepEqual(errors, []);
  console.log('réglages du modèle ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
