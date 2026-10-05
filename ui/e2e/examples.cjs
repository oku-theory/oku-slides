// Atelier de widget (t-497) : un cas ajouté en pastille, un champ vidé sur la scène, resté vide
// (l'autre cas intact), les cas retrouvés après rechargement (ils sont dans le pack).
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

  // Le réglage du titre sous l'aperçu (libellé : le nom du paramètre), relu jusqu'à ce que `ok` le prenne :
  // la surface des réglages est reconstruite à chaque relecture.
  const title = p.locator(`${TAB} .host-settings`).getByRole('textbox', { name: 'title', exact: true });
  const titleUntil = async (ok, what) => {
    for (let t = 0; t < 50; t++) {
      const v = await title.inputValue({ timeout: 1000 }).catch(() => null);
      if (v !== null && ok(v)) return v;
      await p.waitForTimeout(100);
    }
    assert.fail(`réglage du titre : ${what} attendu, « ${await title.inputValue({ timeout: 1000 }).catch(() => '?')} »`);
  };
  await titleUntil((v) => v !== '', 'une valeur');

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

  // Le champ vidé le reste (pas de texte d'exemple) ; le cas « Exemple », lui, garde son titre.
  await titleUntil((v) => v === '', 'vide');
  await pills.getByRole('button', { name: 'Exemple', exact: true }).click();
  await pills.getByRole('button', { name: 'Exemple', pressed: true }).waitFor();
  await titleUntil((v) => v !== '', 'une valeur');

  // Rechargé : les deux cas sont toujours là, « Cas 2 » toujours vidé.
  await p.reload();
  await pills.getByRole('button', { name: 'Cas 2', exact: true }).waitFor();
  assert.equal(await pills.getByRole('button', { name: 'Exemple', exact: true }).count(), 1);
  await pills.getByRole('button', { name: 'Cas 2', exact: true }).click();
  await pills.getByRole('button', { name: 'Cas 2', pressed: true }).waitFor();
  await titleUntil((v) => v === '', 'vide');

  assert.deepEqual(errors, []);
  console.log('cas d\'exemple ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
