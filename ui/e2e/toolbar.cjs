// Barre d'outils (sous la feuille) : ajout d'éléments ; suppression de slide au coin de sa miniature.
// Contre `oku-slides serve fixtures/roadmap.json`.
// Lancer : just e2e (serveur neuf, port 4872). À la main : bun run e2e dans ui/, E2E_URL = adresse du serveur.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const state = (p) => p.evaluate(() => ({
  slides: document.querySelectorAll('.thumb').length,
  add: [...document.querySelectorAll('.add-group button')].map((b) => !b.disabled),
  undo: document.querySelector('[aria-label^="Annuler"]')?.getAttribute('aria-label'),
}));
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  // Slide 1 (titre) : rien ne s'ajoute.
  assert.deepEqual((await state(p)).add, [false, false, false, false]);
  // Slide 4 (contenu) : tout s'ajoute ; le graphique s'ajoute et s'annule par son libellé.
  await p.click('.thumb >> nth=3');
  await p.waitForTimeout(600);
  assert.deepEqual((await state(p)).add, [true, true, true, true]);
  await p.getByRole('group', { name: 'Ajouter sur la slide' }).getByRole('button', { name: 'Graphique' }).click();
  await p.waitForTimeout(700);
  assert.equal((await state(p)).undo, 'Annuler : Ajouter un graphique');
  // Suppression au coin de la miniature, puis annulation.
  const before = (await state(p)).slides;
  await p.locator('.thumb-slot').nth(3).getByRole('button', { name: 'Supprimer la slide' }).click();
  await p.waitForTimeout(700);
  assert.equal((await state(p)).slides, before - 1);
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(700);
  assert.equal((await state(p)).slides, before);
  // Plus de post-it d'ajout ni de suppression dans le panneau.
  assert.equal(await p.locator('.notes .add-note, .notes button.postit').count(), 0);
  // Fenêtre minimale (1024 px) : la barre reste sous la feuille, ses jetons ne s'écrasent pas.
  await p.setViewportSize({ width: 1024, height: 640 });
  await p.waitForTimeout(300);
  const fit = await p.evaluate(() => {
    const tray = document.querySelector('.tools-tray').getBoundingClientRect();
    const stage = document.querySelector('.stage').getBoundingClientRect();
    return {
      under: tray.top >= stage.bottom && tray.left >= stage.left && tray.right <= stage.right,
      tokens: [...document.querySelectorAll('.tools-tray .token')].map((e) => e.getBoundingClientRect().width),
    };
  });
  assert.ok(fit.under, 'barre d\'outils hors de la colonne de la feuille');
  assert.ok(fit.tokens.every((w) => w === 36), `jetons écrasés : ${fit.tokens}`);
  assert.deepEqual(errors, []);
  console.log('barre d\'outils ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
