// Aide (t-469) : ⌘/ ouvre le deck d'aide en lecture seule, dans un onglet à
// lui, contre `oku-slides serve fixtures/roadmap.json`. Rien ne s'y édite :
// ni barre d'outils, ni agent, ni dictée, ni ajout de slide ; double-clic,
// Suppr et ⌘Z ne changent rien. Les flèches parcourent les slides, ⌘/ une
// seconde fois retrouve le même onglet, le bouton ✕ le ferme.
// Lancer : just e2e. À la main : bun e2e/help.cjs, E2E_URL = adresse du serveur.
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
  const count = (n) => p.waitForFunction((n) => document.querySelectorAll('[role="tab"]').length === n, n);
  const caption = (text) =>
    p.waitForFunction((text) => document.querySelector('.help-bar .caption')?.textContent?.startsWith(text), text);
  const tabs = await p.getByRole('tab').count();
  const mod = process.platform === 'darwin' ? 'Meta' : 'Control';

  await p.keyboard.press(`${mod}+Slash`);
  await count(tabs + 1);
  await p.waitForSelector('.help-bar');
  await caption('Slide 1 sur');
  // Lecture seule : ni outils, ni agent, ni dictée, ni ajout de slide.
  for (const sel of ['.tools-tray', '.notes', '.tray']) assert.equal(await p.locator(sel).isVisible(), false, sel);
  assert.equal(await p.getByRole('button', { name: 'Nouvelle slide' }).count(), 0);

  await p.keyboard.press('ArrowRight');
  await p.keyboard.press('ArrowRight');
  await p.keyboard.press('ArrowRight');
  await caption('Slide 4 sur');
  await p.waitForSelector('.stage img');
  const before = await p.locator('.stage img').getAttribute('src');
  const box = await p.locator('.stage img').boundingBox();
  await p.mouse.dblclick(box.x + box.width / 2, box.y + box.height / 2);
  await p.keyboard.press('Delete');
  await p.keyboard.press(`${mod}+z`);
  await p.waitForTimeout(500);
  assert.equal(await p.locator('.stage img').getAttribute('src'), before);
  await caption('Slide 4 sur');

  // ⌘/ dans l'aide : le même onglet, pas un second.
  await p.keyboard.press(`${mod}+Slash`);
  await p.waitForTimeout(300);
  await count(tabs + 1);

  await p.getByRole('button', { name: "Fermer l'aide" }).click();
  await count(tabs);
  assert.deepEqual(errors, []);
  await b.close();
  console.log('help: ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
