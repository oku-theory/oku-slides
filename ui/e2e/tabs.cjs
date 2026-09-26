// Onglets : nouvel onglet, bascule, fermeture, question pour un onglet modifié,
// contre `oku-slides serve fixtures/roadmap.json`.
// Passe après toolbar.cjs et panel.cjs sur le même serveur : le premier onglet
// (le fichier) a déjà été modifié ; ce script ne ferme que ses propres onglets.
// Il laisse un onglet modifié ouvert : relancé à la main, il lui faut un
// serveur neuf. Lancer : just e2e. À la main : bun e2e/tabs.cjs, E2E_URL =
// adresse d'un serveur tout juste démarré.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  const tabs = p.getByRole('tab');
  const count = (n) => p.waitForFunction((n) => document.querySelectorAll('[role="tab"]').length === n, n);
  const selected = (i) =>
    p.waitForFunction((i) => document.querySelectorAll('[role="tab"]')[i]?.getAttribute('aria-selected') === 'true', i);
  const thumbs = (n) => p.waitForFunction((n) => document.querySelectorAll('.thumb').length === n, n);
  await count(1);
  const slides = await p.locator('.thumb').count();
  // Nouvel onglet A : un deck vierge (2 slides), actif.
  await p.getByRole('button', { name: 'Nouvel onglet' }).click();
  await count(2);
  await selected(1);
  await thumbs(2);
  // Retour au fichier : son deck, intact.
  await tabs.nth(0).click();
  await selected(0);
  await thumbs(slides);
  // Nouvel onglet B, modifié : sa fermeture pose la question ; refuser le garde.
  await p.getByRole('button', { name: 'Nouvel onglet' }).click();
  await count(3);
  await selected(2);
  await thumbs(2);
  await p.getByRole('button', { name: 'Supprimer la slide' }).click();
  await tabs.nth(2).locator('.tab-dirty').waitFor();
  const dialog = p.waitForEvent('dialog');
  await tabs.nth(2).locator('.tab-close').click();
  await (await dialog).dismiss();
  // Refus : aucune requête de fermeture ; la vue relue montre toujours B.
  await p.evaluate(() => fetch('/api/workspace', { method: 'POST', body: JSON.stringify({ cmd: 'view' }) }));
  await tabs.nth(2).locator('.tab-dirty').waitFor();
  assert.equal(await tabs.count(), 3);
  assert.equal(await tabs.nth(2).getAttribute('aria-selected'), 'true');
  // A, non modifié, se ferme sans question.
  await tabs.nth(1).locator('.tab-close').click();
  await count(2);
  assert.deepEqual(errors, []);
  await b.close();
  console.log('tabs: ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
