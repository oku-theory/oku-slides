// Atelier d'un pack de layouts (mode pack) : la grille de `layout-blank` changée par la carte de la
// slide (son exemple), enregistrée dans un nouveau pack, se retrouve en le rouvrant. L'atelier se
// demande à l'agent (t-988). Au passage : pas de sous-onglet
// « Modèles » (sa scène est l'onglet « Layouts »), une miniature ramène la scène, l'onglet « Thème »
// montre les jetons à droite.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e (serveur neuf, port 4872).
// À la main : bun run e2e dans ui/, E2E_URL = adresse du serveur (défaut port 4870).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openAtelier } = require('./lib/atelier.cjs');
const { post } = require('./lib/http.cjs');
const { until, untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = 't-1';
const api = (p, req) => post(p, '/api', { tab: TAB, req });
const TARGET = '.settings-band [data-surface-id="host-target"]';
const thumb = (name) => `.thumb:has(.thumb-name:text-is("${name}"))`;
/** Valeurs des steppers de la carte de la slide (colonnes, lignes de sa grille). */
const grid = (p) => p.evaluate((sel) => [...document.querySelectorAll(`${sel} .stepper-value`)].map((e) => Number(e.textContent)), TARGET);
const stageHidden = (p) => p.evaluate(() => document.getElementById('stage-panel').hidden);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('dialog', (d) => d.accept());
  await p.goto(URL);
  await p.waitForSelector('.stage img');

  // 1. L'atelier s'ouvre en le demandant à l'agent (t-988), sur Essentiel (pack de layouts épinglé par le deck).
  await openAtelier(p);
  await until(p, async () => (await api(p, { cmd: 'snapshot' })).editing?.kind === 'layouts', "l'atelier ne s'ouvre pas sur un pack de layouts");
  const essentiel = (await api(p, { cmd: 'snapshot' })).editing.pack;
  await p.waitForSelector(thumb('blank'));
  assert.equal(await p.getByRole('tab', { name: 'Modèles', exact: true }).count(), 0, 'plus de sous-onglet Modèles');
  assert.equal(await p.getByRole('button', { name: /Nouvelle slide/ }).count(), 0, "pas d'ajout de slide en mode pack");
  assert.equal(await p.locator('.thumb-remove').count(), 0, 'pas de suppression de slide en mode pack');

  // Un widget ouvert, une miniature cliquée : la scène revient, sur cette slide.
  await p.click('.pile-widgets .pile-section-head .token');
  await p.fill('.pile-widgets-name .select', 'essai-pile');
  await p.keyboard.press('Enter');
  await p.waitForSelector('.widget-tab:not([hidden])');
  assert.equal(await stageHidden(p), true);
  await p.click(thumb('title'));
  await untilValue(p, () => stageHidden(p), (hidden) => hidden === false, 'la scène ne revient pas');
  assert.equal(await p.locator(thumb('title')).getAttribute('aria-current'), 'true');
  await p.click('.pile-widgets-row:has-text("essai-pile") .pile-widgets-remove');
  await p.waitForSelector('.pile-widgets-row:has-text("essai-pile")', { state: 'detached' });

  // Onglet « Thème » : les jetons à droite, à la place du fil ; une miniature cliquée le garde.
  await p.getByRole('tab', { name: 'Thème', exact: true }).click();
  await p.waitForSelector('#theme-panel [data-surface-id="host-theme"] .a2ui-disclosure');
  assert.equal(await p.locator('.notes #theme-panel').count(), 1);
  assert.equal(await stageHidden(p), false);
  await p.click(thumb('blank'));
  await untilValue(p, () => p.locator(thumb('blank')).getAttribute('aria-current'), (v) => v === 'true', 'la miniature cliquée ne devient pas la courante');
  // négatif : un retour fautif à la scène seule aurait le temps d'arriver.
  await p.waitForTimeout(300);
  assert.equal(await p.getByRole('tab', { name: 'Thème', exact: true }).getAttribute('aria-selected'), 'true');
  // Le menu ⋯ du mode pack n'offre que l'aide : le reste se demande à l'agent (t-988).
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.waitForSelector('[role="menu"]');
  assert.deepEqual(await p.locator('[role="menu"] [role="menuitem"] > span').allTextContents(), ['Aide']);
  await p.keyboard.press('Escape');
  // Retour à la scène par l'onglet « Layouts » : le Thème rend la colonne de droite.
  await p.getByRole('tab', { name: 'Layouts', exact: true }).click();
  await p.waitForSelector('#theme-panel', { state: 'detached' });

  // 2-3. La slide `layout-blank` : sa grille passe à 4 colonnes par la carte de la slide (son exemple).
  await p.click(thumb('blank'));
  await p.waitForSelector(`${TARGET} .stepper-value`);
  const rows = (await grid(p))[1];
  const fewer = p.locator(TARGET).getByRole('button', { name: 'Colonnes : une de moins' });
  for (let cols = (await grid(p))[0]; cols > 4; cols--) {
    await fewer.click();
    await untilValue(p, () => grid(p), (g) => g[0] === cols - 1, `la grille ne passe pas à ${cols - 1} colonnes`);
  }
  assert.deepEqual(await grid(p), [4, rows]);

  // 4. Enregistrer une copie (plus au menu : l'agent, outil `save_pack`, t-988 ; ici par la session) :
  // un nouveau pack, que l'atelier édite désormais.
  await api(p, { cmd: 'save_pack', as_new: true });
  const copy = await untilValue(p, async () => (await api(p, { cmd: 'snapshot' })).editing, (e) => !!e && e.pack !== essentiel && !e.builtin, "la copie n'est pas éditée");

  // 5. Terminer.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await until(p, async () => (await api(p, { cmd: 'snapshot' })).editing === null, "l'atelier ne se ferme pas");
  await p.waitForSelector('.stage img');

  // 6. Rouvrir le nouveau pack (aucun deck ne l'épingle : par la session), puis recharger la page.
  const reopened = await api(p, { cmd: 'edit_pack', pack: copy.pack });
  assert.equal(reopened.editing?.pack, copy.pack, `le nouveau pack ne se rouvre pas : ${JSON.stringify(reopened).slice(0, 200)}`);
  await p.reload();

  // 7. `layout-blank` garde ses 4 colonnes.
  await p.click(thumb('blank'));
  await untilValue(p, () => grid(p), (g) => g[0] === 4 && g[1] === rows, 'la grille de layout-blank ne garde pas ses 4 colonnes');
  await p.getByRole('button', { name: 'Terminer' }).click();
  await until(p, async () => (await api(p, { cmd: 'snapshot' })).editing === null, "l'atelier rouvert ne se ferme pas");
  assert.deepEqual(errors, []);
  console.log('atelier de layouts ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
