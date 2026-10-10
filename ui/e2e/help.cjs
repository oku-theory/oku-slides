// Aide (t-469) : ⌘/ ouvre le deck d'aide en lecture seule, dans un onglet à
// lui, contre `oku-slides serve fixtures/roadmap.json`. Rien ne s'y édite :
// ni barre d'outils, ni agent, ni dictée, ni ajout de slide ; double-clic,
// Suppr et ⌘Z ne changent rien. Les flèches parcourent les slides, ⌘/ une
// seconde fois retrouve le même onglet, le bouton ✕ le ferme.
// Lancer : just e2e. À la main : bun e2e/help.cjs, E2E_URL = adresse du serveur.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { stageImage } = require('./stage.cjs');
const { settled, untilValue } = require('./until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
// Délai du premier rendu de l'aide (deck entier), en ms.
const FIRST_RENDER_MS = 120_000;
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  const count = (n) => p.waitForFunction((n) => document.querySelectorAll('[role="tab"]').length === n, n);
  const caption = (text) =>
    p.waitForFunction((text) => document.querySelector('.help-bar .caption')?.textContent?.startsWith(text), text);
  const tabs = await p.getByRole('tab').count();
  const mod = process.platform === 'darwin' ? 'Meta' : 'Control';

  await p.keyboard.press(`${mod}+Slash`);
  await count(tabs + 1);
  await p.waitForSelector('.help-bar');
  await caption('Slide 1 sur');
  // Premier rendu : les 32 slides de l'aide se compilent d'un coup. Il peut
  // dépasser les 30 s par défaut sur une machine chargée (juste après une
  // compilation, t-523) ; ensuite, chaque slide sort du cache.
  await p.waitForSelector('.stage img', { timeout: FIRST_RENDER_MS });
  // Lecture seule : ni outils, ni agent, ni dictée, ni ajout de slide.
  for (const sel of ['.tools-tray', '.notes', '.tray']) assert.equal(await p.locator(sel).isVisible(), false, sel);
  assert.equal(await p.getByRole('button', { name: 'Nouvelle slide' }).count(), 0);
  // Le bandeau couvre la rangée du titre : la pile commence dessous, sans être masquée (t-810).
  const bar = await p.evaluate(() => [document.querySelector('.help-bar').getBoundingClientRect().bottom, document.querySelector('.pile-column').getBoundingClientRect().top]);
  assert.ok(bar[1] >= bar[0], `bandeau de l'aide sur la pile (${bar[1]} < ${bar[0]})`);

  await p.keyboard.press('ArrowRight');
  await p.keyboard.press('ArrowRight');
  await p.keyboard.press('ArrowRight');
  await caption('Slide 4 sur');
  // Le contenu du rendu (SVG) de l'image visible, pas l'adresse de l'image : revenir sur la slide peut
  // en créer une autre. La légende peut devancer l'image : on attend que l'image visible soit celle de la
  // slide 4 (`alt`), puis deux lectures identiques (son adresse peut encore suivre) (t-750).
  const slide4 = (v) => v.alt === 'Slide 4' && v.svg !== '';
  await untilValue(p, () => stageImage(p), slide4, 'image de la slide 4');
  const shown = await settled(p, () => stageImage(p), 'rendu de la slide 4 stable');
  assert.ok(slide4(shown), `image de la slide 4 après stabilité : ${shown.alt}`);
  const before = shown.svg;
  const box = await p.locator('.stage img').filter({ visible: true }).boundingBox();
  await p.mouse.dblclick(box.x + box.width / 2, box.y + box.height / 2);
  await p.keyboard.press('Delete');
  await p.keyboard.press(`${mod}+z`);
  // négatif : en lecture seule, rien ne change ; une édition fautive aurait le temps de redessiner.
  await p.waitForTimeout(500);
  // Point de synchronisation : une flèche suit Suppr et ⌘Z, son effet prouve qu'elles sont traitées ;
  // retour sur la slide 4 pour relire son rendu.
  await p.keyboard.press('ArrowRight');
  await caption('Slide 5 sur');
  await p.keyboard.press('ArrowLeft');
  await caption('Slide 4 sur');
  // Une édition fautive persiste : le rendu ne revient jamais à celui d'avant, pas de faux vert.
  await untilValue(p, () => stageImage(p), (v) => v.alt === 'Slide 4' && v.svg === before, 'la slide de l\'aide a changé');

  // Recherche : ⌘F, un mot, Entrée montre la slide trouvée.
  await p.keyboard.press(`${mod}+f`);
  await p.keyboard.type('grille');
  await p.waitForSelector('.help-search-results [role="option"]');
  await p.keyboard.press('Enter');
  await caption('Slide 6 sur');
  await p.keyboard.press('Escape');
  await p.evaluate(() => document.activeElement?.blur());

  // ⌘/ dans l'aide : le même onglet, pas un second.
  await p.keyboard.press(`${mod}+Slash`);
  // négatif : un second onglet fautif aurait le temps de s'ouvrir.
  await p.waitForTimeout(300);
  await count(tabs + 1);
  // Point de synchronisation : la fermeture qui suit retire l'onglet.

  await p.getByRole('button', { name: "Fermer l'aide" }).click();
  await count(tabs);
  assert.deepEqual(errors, []);
  await b.close();
  console.log('help: ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
