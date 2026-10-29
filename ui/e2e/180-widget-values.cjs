// Atelier de widget (t-497, t-541) : un champ vidé sur la scène de l'aperçu reste vide dans les
// réglages du widget, et le reste après rechargement (ses valeurs d'exemple sont dans le pack).
// In Essentiel's pack tab. Writes: Essentiel (the `fiche` widget and its example values). Leaves: no pack tab.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openEssentiel, packTabShown } = require('./lib/atelier.cjs');
const { untilValue } = require('./lib/until.cjs');
const { newPackItem, pileThumb } = require('./lib/widget.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = '.widget-tab';

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');
  await openEssentiel(p);
  // The item is selected once created (newPackItem waits for its thumbnail, current).
  await newPackItem(p, 'fiche');

  // Le réglage du titre sous l'aperçu (libellé : le nom du paramètre), relu jusqu'à ce que `ok` le prenne :
  // la surface des réglages est reconstruite à chaque relecture.
  const title = p.locator(`${TAB} .host-settings`).getByRole('textbox', { name: 'title', exact: true });
  const titleUntil = (ok, what) => untilValue(
    p,
    () => title.inputValue({ timeout: 1000 }).catch(() => null),
    (v) => v !== null && ok(v),
    `réglage du titre : ${what} attendu`,
  );
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

  // Le champ vidé le reste dans les réglages (pas de texte d'exemple).
  await titleUntil((v) => v === '', 'vide');

  // Rechargé : l'élément reste choisi, le titre toujours vidé.
  await p.reload();
  await packTabShown(p);
  await pileThumb(p, 'fiche', true).waitFor();
  await titleUntil((v) => v === '', 'vide');

  await closePackTabs(p);
  assert.deepEqual(errors, []);
  console.log('valeurs d\'exemple ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
