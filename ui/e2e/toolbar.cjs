// Barre d'outils : ajout d'éléments et suppression de slide, contre `slider serve fixtures/roadmap.json`.
// Lancer : bun run e2e dans ui/ (Chromium : bunx playwright install chromium). E2E_URL change l'adresse (défaut port 4870).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const state = (p) => p.evaluate(() => ({
  slides: document.querySelectorAll('.thumb').length,
  add: [...document.querySelectorAll('.add-card button')].map((b) => !b.disabled),
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
  // Suppression depuis la barre d'outils, puis annulation.
  const before = (await state(p)).slides;
  await p.getByRole('button', { name: 'Supprimer la slide' }).click();
  await p.waitForTimeout(700);
  assert.equal((await state(p)).slides, before - 1);
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(700);
  assert.equal((await state(p)).slides, before);
  // Plus de post-it d'ajout ni de suppression dans le panneau.
  assert.equal(await p.locator('.notes .add-note, .notes button.postit').count(), 0);
  // Fenêtre minimale (1024 px) : les outils ne chevauchent pas l'étiquette et ne s'écrasent pas.
  await p.setViewportSize({ width: 1024, height: 640 });
  await p.waitForTimeout(300);
  const fit = await p.evaluate(() => {
    const items = [...document.querySelectorAll('.tray > *')].map((e) => e.getBoundingClientRect());
    return {
      gap: Math.min(...items.map((r) => r.left)) - document.querySelector('.label').getBoundingClientRect().right,
      tokens: [...document.querySelectorAll('.tray .token')].map((e) => e.getBoundingClientRect().width),
    };
  });
  assert.ok(fit.gap > 0, `outils sur l'étiquette (${fit.gap} px)`);
  assert.ok(fit.tokens.every((w) => w === 44), `jetons écrasés : ${fit.tokens}`);
  assert.deepEqual(errors, []);
  console.log('barre d\'outils ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
