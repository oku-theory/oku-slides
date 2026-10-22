// Barre d'outils (sous la feuille) : ajout d'éléments par la grille filtrable ; suppression de slide au coin de sa miniature.
// Contre `oku-slides serve fixtures/roadmap.json`.
// Lancer : just e2e (serveur neuf, port 4872). À la main : bun run e2e dans ui/, E2E_URL = adresse du serveur.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { until, stable } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const state = (p) => p.evaluate(() => ({
  slides: document.querySelectorAll('.thumb').length,
  add: document.querySelector('.add-button')?.disabled === false,
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
  assert.equal((await state(p)).add, false);
  // Slide 4 (contenu) : le bouton d'ajout ouvre la grille ; filtré sur la description du graphique,
  // Entrée l'ajoute et l'hôte annonce son annulation par un libellé.
  await p.click('.thumb >> nth=3');
  await until(p, async () => (await state(p)).add, 'slide 4 : le bouton d\'ajout ne s\'active pas');
  await p.getByRole('button', { name: 'Ajouter sur la slide' }).click();
  const picker = p.getByRole('dialog', { name: 'Ajouter sur la slide' });
  await picker.waitFor();
  const options = picker.getByRole('option');
  assert.deepEqual(await options.locator('.card-name').allTextContents(), ['Texte', 'Carte', 'Forme', 'Graphique', 'Tableau']);
  await picker.getByRole('combobox').fill('tiré');
  await until(p, async () => (await options.count()) === 1, 'le filtre sur la description ne garde pas le seul graphique');
  assert.equal(await options.locator('.card-name').textContent(), 'Graphique');
  await p.waitForSelector('.widget-picker[data-armed]');
  await p.keyboard.press('Enter');
  await picker.waitFor({ state: 'detached' });
  await until(p, async () => (await state(p)).undo === 'Annuler : Ajouter un graphique', 'le graphique n\'est pas ajouté (libellé d\'annulation)');
  assert.equal((await state(p)).undo, 'Annuler : Ajouter un graphique');
  // Suppression au coin de la miniature, puis annulation.
  const before = (await state(p)).slides;
  await p.locator('.thumb-slot').nth(3).getByRole('button', { name: 'Supprimer la slide' }).click();
  await until(p, async () => {
    const s = await state(p);
    return s.slides === before - 1 && s.undo === 'Annuler : Supprimer la slide';
  }, 'la slide n\'est pas supprimée (nombre de slides, libellé d\'annulation)');
  assert.equal((await state(p)).slides, before - 1);
  await p.keyboard.press('Control+z');
  await until(p, async () => (await state(p)).slides === before, 'la suppression n\'est pas annulée');
  assert.equal((await state(p)).slides, before);
  // Plus de carte d'ajout ni de suppression dans le panneau.
  assert.equal(await p.locator('.notes .add-note').count(), 0);
  // Fenêtre minimale (1024 px) : la barre, menu compris, reste sous la feuille, dans sa colonne, ses jetons ne s'écrasent pas.
  // La colonne, marges comprises (où passent les flèches slide précédente / suivante) : à 1024 px, le fil
  // (40 % de la table) ne laisse pas à la slide la largeur de la barre, qui la déborde, d'une marge au plus.
  await p.setViewportSize({ width: 1024, height: 640 });
  await stable(p, ['.tools-tray', '.stage', '.sheet-area']);
  const fit = await p.evaluate(() => {
    const tray = document.querySelector('.tools-tray').getBoundingClientRect();
    const stage = document.querySelector('.stage').getBoundingClientRect();
    const column = document.querySelector('.sheet-area').getBoundingClientRect();
    const gutter = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--space-6'));
    return {
      under: tray.top >= stage.bottom && tray.left >= column.left && tray.right <= column.right,
      boxes: `barre ${tray.left}–${tray.right} (haut ${tray.top}), colonne ${column.left}–${column.right}, slide bas ${stage.bottom}`,
      overhang: Math.max(stage.left - tray.left, tray.right - stage.right),
      gutter,
      tokens: [...document.querySelectorAll('.tools-tray .token')].map((e) => e.getBoundingClientRect().width),
    };
  });
  assert.ok(fit.under, `barre d'outils hors de la colonne de la feuille : ${fit.boxes}`);
  assert.ok(fit.overhang <= fit.gutter, `barre plus large que la slide de ${fit.overhang} px (marge ${fit.gutter})`);
  // Jetons réduits à 32 px en fenêtre étroite, tous égaux (aucun écrasé).
  assert.ok(fit.tokens.length > 0 && fit.tokens.every((w) => w === 32), `jetons écrasés : ${fit.tokens}`);
  assert.deepEqual(errors, []);
  console.log('barre d\'outils ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
