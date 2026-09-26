// Test de fumée de l'édition à la main, contre `oku-slides serve fixtures/roadmap.json` (E2E_URL, port 4870 par défaut).
// Lancer : bun ui/e2e/smoke.cjs (Playwright installé).
const { chromium } = require('playwright');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
(async () => {
  const b = await chromium.launch();
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', e => errors.push(e.message));
  p.on('console', m => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  await p.waitForTimeout(800);
  await p.screenshot({ path: '/tmp/e2e-1.png' });
  // Slide 7 : placement libre.
  await p.click('.thumb >> nth=6');
  await p.waitForTimeout(600);
  const t = p.locator('.target[aria-label="Texte"]').first();
  const bb = await t.boundingBox();
  await p.mouse.move(bb.x + 20, bb.y + 20);
  await p.mouse.down();
  await p.mouse.move(bb.x + 120, bb.y + 80, { steps: 5 });
  await p.mouse.up();
  await p.waitForTimeout(700);
  await p.screenshot({ path: '/tmp/e2e-2.png' });
  const bb2 = await p.locator('.target.selected').boundingBox();
  console.log('déplacement px', Math.round(bb2.x - bb.x), Math.round(bb2.y - bb.y));
  // Édition du titre.
  await p.dblclick('.target[aria-label="Titre"]');
  await p.fill('.editor .cm-content', 'Titre modifié à la main');
  await p.keyboard.press('Enter');
  await p.waitForTimeout(700);
  const snap = await p.evaluate(() => fetch('/api', {method:'POST', body: JSON.stringify({tab:'t-1', req:{cmd:'slide', id:'s-libre'}})}).then(r => r.json()));
  console.log('titre', snap.slide.title, '| cadre', JSON.stringify(snap.slide.elements.find(e => e.id==='e-texte').frame));
  await p.screenshot({ path: '/tmp/e2e-3.png' });
  // Annuler deux fois.
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(500);
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(700);
  const s2 = await p.evaluate(() => fetch('/api', {method:'POST', body: JSON.stringify({tab:'t-1', req:{cmd:'slide', id:'s-libre'}})}).then(r => r.json()));
  console.log('après annulation', s2.slide.title, JSON.stringify(s2.slide.elements.find(e => e.id==='e-texte').frame));
  // Thème pro.
  await p.selectOption('#theme', 'pro');
  await p.waitForTimeout(900);
  await p.screenshot({ path: '/tmp/e2e-4.png' });
  console.log('erreurs', errors);
  await b.close();
})();
