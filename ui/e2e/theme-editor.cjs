// Éditeur de thème (t-554) : onglet « Thème » du mode modèles, un jeton changé se voit sur la scène,
// et ✓ enregistre le brouillon en copie du thème de base, choisie pour le deck.
// Contre `oku-slides serve fixtures/roadmap.json` (thème Papier). Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { stageSvg } = require('./stage.cjs');
const { untilValue } = require('./until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const SURFACE = '#theme-panel [data-surface-id="host-theme"]';
// La ligne d'un jeton : le composant qui la porte a pour id le chemin du jeton suivi de `:row`.
const row = (token) => `${SURFACE} [data-component-id="${token}:row"]`;
const RED = /#ff0000/i;

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('dialog', (d) => {
    errors.push(`dialogue inattendu : ${d.message()}`);
    void d.dismiss();
  });
  await p.goto(URL);
  // Un test précédent peut avoir laissé l'onglet en mode modèles (sous-onglet d'un widget ouvert) :
  // on y entre s'il le faut.
  await p.waitForSelector('.stage img, .widget-pane', { state: 'attached' });
  const tab = p.getByRole('tab', { name: 'Thème', exact: true });
  if (!(await tab.isVisible())) {
    await p.getByRole('button', { name: 'Menu' }).click();
    await p.getByRole('menuitem', { name: 'Atelier' }).click();
  }
  await tab.click();
  await p.waitForSelector(`${SURFACE} .a2ui-disclosure`);
  // Le premier modèle (couverture), qui emploie l'accent.
  await p.click('.thumb >> nth=0');
  await p.waitForSelector('.stage img');
  assert.doesNotMatch(await stageSvg(p), RED);

  // Le filtre, validé par Entrée, ne garde que les jetons « accent » : color.ink disparaît, color.accent reste.
  const filter = p.locator(SURFACE).getByRole('textbox').first();
  await filter.fill('accent');
  await filter.press('Enter');
  await p.waitForSelector(row('color.ink'), { state: 'detached' });
  const picker = p.locator(`${row('color.accent')} input[type="color"]`);

  // Le sélecteur natif du ColorField : `fill` émet input puis change, comme sa fermeture.
  await picker.fill('#ff0000');
  await untilValue(p, () => stageSvg(p), (svg) => RED.test(svg), 'la scène ne montre pas le nouvel accent');
  // La surface relue garde la valeur envoyée.
  await untilValue(p, () => picker.inputValue(), (v) => v === '#ff0000', "la surface relue n'a pas gardé l'accent envoyé");

  // ✓ tout de suite, sans attendre l'enregistrement automatique : il part d'abord, et aucune
  // confirmation « quitter sans enregistrer » ne s'ouvre (l'instantané lu est celui de l'enregistrement).
  // Le brouillon est enregistré dans la bibliothèque sous « Papier (copie) », thème du deck.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await p.waitForSelector('#theme-panel', { state: 'detached' });
  await p.getByRole('button', { name: 'Menu' }).click();
  const themes = p.locator('[role="menu"]').getByRole('combobox', { name: 'Thème', exact: true });
  await themes.waitFor();
  const selected = () => themes.evaluate((s) => s.selectedOptions[0]?.textContent ?? '');
  await untilValue(p, selected, (name) => name === 'Papier (copie)', 'le thème du deck n\'est pas « Papier (copie) »');
  await p.keyboard.press('Escape');
  // Hors du mode modèles, la scène garde l'accent rouge.
  await untilValue(p, () => stageSvg(p), (svg) => RED.test(svg), "hors du mode modèles, la scène perd l'accent rouge");

  assert.deepEqual(errors, []);
  console.log('éditeur de thème ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
