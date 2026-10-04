// Atelier de widget (t-497) : un cas ajouté en pastille, un champ vidé sur la scène, l'alerte
// « Champ vide » sur la planche, les cas retrouvés après rechargement (ils sont dans le pack).
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = '.widget-tab:not([hidden])';

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  // Un test précédent peut avoir laissé l'onglet en mode modèles : on y entre s'il le faut.
  await p.waitForSelector('.stage img, .widget-pane', { state: 'attached' });
  const add = p.getByRole('button', { name: 'Nouveau widget' });
  if (!(await add.isVisible())) {
    await p.getByRole('button', { name: 'Menu' }).click();
    await p.getByRole('menuitem', { name: 'Éditer les widgets' }).click();
  }
  await add.click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('fiche');
  await name.press('Enter');
  // Le sous-onglet s'ouvre après la création : on attend qu'il soit l'actif avant de viser ses pastilles.
  await p.getByRole('tab', { name: 'fiche', selected: true }).waitFor();

  // Un cas « Exemple » d'office ; « + Cas » en ajoute un, copie du courant, et l'affiche.
  const pills = p.locator(`${TAB} [role="group"][aria-label="Cas d'exemple"]`);
  await pills.getByRole('button', { name: 'Exemple', pressed: true }).waitFor();
  await pills.getByRole('button', { name: 'Ajouter un cas, copie du cas courant' }).click();
  await pills.getByRole('button', { name: 'Cas 2', pressed: true }).waitFor();

  // Scène : le titre se vide sur l'aperçu, comme un champ de slide.
  const field = p.locator(`${TAB} .stage-preview .field`).first();
  await field.waitFor();
  await field.dblclick();
  const editor = p.locator(`${TAB} .stage-preview .cm-content`);
  await editor.waitFor();
  await p.keyboard.press(process.platform === 'darwin' ? 'Meta+a' : 'Control+a');
  await p.keyboard.press('Backspace');
  await p.keyboard.press('Enter');
  await editor.waitFor({ state: 'detached' });

  // Planche : une vignette par cas ; « Cas 2 » signale son champ vide, « Exemple » n'a pas d'alerte.
  await p.getByRole('button', { name: 'Planche' }).click();
  const card = (n) => p.locator(`${TAB} .board-card`, { has: p.locator('.board-name', { hasText: new RegExp(`^${n}$`) }) });
  await card('Cas 2').locator('.board-badge', { hasText: 'Champ vide' }).waitFor();
  await card('Exemple').locator('.board-thumb').waitFor();
  assert.equal(await card('Exemple').locator('.board-badge').count(), 0);

  // Un clic sur une vignette ramène à la scène, sur ce cas.
  await card('Exemple').click();
  await pills.getByRole('button', { name: 'Exemple', pressed: true }).waitFor();
  await p.locator(`${TAB} .stage-preview`).waitFor();

  // Rechargé : les deux cas sont toujours là.
  await p.reload();
  await pills.getByRole('button', { name: 'Cas 2', exact: true }).waitFor();
  assert.equal(await pills.getByRole('button', { name: 'Exemple', exact: true }).count(), 1);

  assert.deepEqual(errors, []);
  console.log('cas d\'exemple ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
