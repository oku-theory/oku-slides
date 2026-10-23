// Éditeur de thème (t-554) : onglet « Thème » du mode pack, un jeton changé se voit sur la scène,
// et ✓ enregistre le brouillon en variante du thème de base (elle l'étend), choisie pour le deck ;
// le panneau montre ce que le thème étend et les surcharges du deck mis de côté, ses parents s'ajoutent,
// se déplacent et se retirent (⌘Z compris) ; le Thème d'un pack
// de widgets montre un aperçu par widget.
// Contre `oku-slides serve fixtures/roadmap.json` (thème Papier). Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { activeSnapshot, post } = require('./lib/http.cjs');
const { stageSvg } = require('./lib/stage.cjs');
const { untilValue } = require('./lib/until.cjs');
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
  // Le thème se voit sur des slides : l'atelier d'Essentiel (pack de layouts). L'agent ouvrirait le
  // pack propre du deck, s'il en a un : Essentiel s'ouvre par la session, après le pack laissé ouvert.
  const api = (req) => post(p, '/api', { tab: 't-1', req });
  let snap = await api({ cmd: 'snapshot' });
  if (snap.editing) {
    if (snap.dirty) await api({ cmd: 'save_pack', as_new: false });
    snap = await api({ cmd: 'close_pack' });
  }
  // Un jeton posé sur le deck : l'éditeur le met de côté et le liste, sans l'appliquer.
  await api({ cmd: 'apply', label: 'Jeton', ops: [{ op: 'set_token', path: ['color', 'ink'], value: '#202020' }], merge: null });
  assert.equal((await api({ cmd: 'edit_pack', pack: snap.layouts.essentiel.id })).editing?.kind, 'layouts');
  await p.reload();
  const tab = p.getByRole('tab', { name: 'Thème', exact: true });
  await tab.click();
  await p.waitForSelector(`${SURFACE} .a2ui-disclosure`);
  // La slide du layout `title`, qui emploie l'accent.
  await p.click('.thumb:has(.thumb-name:text-is("title"))');
  await p.waitForSelector('.stage img');
  assert.doesNotMatch(await stageSvg(p), RED);

  // Le thème édité étend Papier ; la surcharge du deck est listée.
  const panel = p.locator(SURFACE);
  await untilValue(p, () => panel.locator('[data-component-id="extends"] [data-component-id$=":name"]').first().innerText().catch(() => ''), (t) => t === 'Papier', "le panneau n'indique pas que le thème étend Papier");
  assert.match(await panel.locator('[data-component-id="deck-overrides"]').innerText(), /color\.ink/);

  // Les parents du brouillon (t-1116) : Pro ajouté, monté, retiré, rendu par ⌘Z, puis retiré pour de bon.
  // Chaque ligne a ses boutons libellés en français ; le thème édité n'est pas proposé à l'ajout.
  const parents = () => api({ cmd: 'snapshot' }).then((s) => s.editing.theme.extends.map((t) => t.name));
  const parentRow = (ns) => panel.locator(`[data-component-id="extends:${ns}"]`);
  const button = (ns, name) => parentRow(ns).getByRole('button', { name, exact: true });
  const addParent = panel.locator('[data-component-id="extends-add"] select');
  const editedName = (await api({ cmd: 'snapshot' })).editing.theme.name;
  assert.equal(await panel.getByText('Ajouter un thème à étendre', { exact: true }).count(), 1, 'menu « Ajouter un thème à étendre » absent');
  const offered = await addParent.locator('option').allTextContents();
  assert.ok(offered.includes('Pro'), `Pro n'est pas proposé : ${offered}`);
  assert.ok(!offered.includes(editedName), `le thème édité « ${editedName} » est proposé : ${offered}`);
  await addParent.selectOption({ label: 'Pro' });
  await untilValue(p, parents, (l) => JSON.stringify(l) === '["Papier","Pro"]', 'Pro ne devient pas un parent');
  await button('pro', 'Monter').waitFor();
  assert.equal(await button('pro', 'Retirer').count(), 1);
  assert.equal(await button('papier', 'Descendre').count(), 1);
  assert.equal(await button('papier', 'Retirer').count(), 1);
  await button('pro', 'Monter').click();
  await untilValue(p, parents, (l) => JSON.stringify(l) === '["Pro","Papier"]', 'Pro ne monte pas');
  await button('pro', 'Retirer').click();
  await untilValue(p, parents, (l) => JSON.stringify(l) === '["Papier"]', "Pro n'est pas retiré");
  await parentRow('pro').waitFor({ state: 'detached' });
  await p.evaluate(() => document.activeElement instanceof HTMLElement && document.activeElement.blur());
  await p.keyboard.press('Control+z');
  await untilValue(p, parents, (l) => JSON.stringify(l) === '["Pro","Papier"]', '⌘Z ne rend pas Pro');
  await button('pro', 'Retirer').click();
  await untilValue(p, parents, (l) => JSON.stringify(l) === '["Papier"]', "Pro n'est pas retiré une seconde fois");
  await parentRow('pro').waitFor({ state: 'detached' });

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
  // Le brouillon est enregistré dans la bibliothèque sous « Papier (variante) », thème du deck, qui étend Papier.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await p.waitForSelector('#theme-panel', { state: 'detached' });
  const deckTheme = async () => (await activeSnapshot(p)).theme_name;
  await untilValue(p, deckTheme, (name) => name === 'Papier (variante)', 'le thème du deck n\'est pas « Papier (variante) »');
  const after = await activeSnapshot(p);
  assert.deepEqual(after.theme_pins[after.theme].extends, ['papier'], 'la variante n\'étend pas Papier');
  // Hors du mode pack, la scène garde l'accent rouge.
  await untilValue(p, () => stageSvg(p), (svg) => RED.test(svg), "hors du mode pack, la scène perd l'accent rouge");

  // Le thème d'un pack de widgets : une slide d'aperçu par widget, hors de l'onglet seulement le vide.
  const widgetPacks = await api({ cmd: 'packs', kind: 'widgets' });
  const base = widgetPacks.find((pack) => pack.namespace === 'base');
  assert.ok(base, 'le pack de widgets base manque');
  const widgetPack = await api({ cmd: 'edit_pack', pack: base.id });
  assert.equal(widgetPack.editing?.kind, 'widgets');
  const previews = widgetPack.slides.map((slide) => slide.id);
  assert.ok(previews.length > 0 && previews.every((id) => id.startsWith('widget-')), `aperçus attendus, pas ${previews}`);
  await p.reload();
  // Hors de l'onglet « Thème », la scène invite à ouvrir un widget.
  await p.waitForSelector('.stage-wrap.empty');
  await p.getByRole('tab', { name: 'Thème', exact: true }).click();
  await p.waitForSelector('.stage img');
  // Une slide d'aperçu par widget d'élément : la flèche suivante les parcourt toutes.
  const next = p.getByRole('button', { name: 'Slide suivante', includeHidden: true });
  for (let seen = 1; seen < previews.length; seen++) {
    await next.click();
    await p.waitForSelector('.stage img');
  }
  assert.equal(await next.isDisabled(), true, 'il reste des aperçus après le dernier');
  assert.equal((await api({ cmd: 'snapshot' })).slides.length, previews.length);
  await api({ cmd: 'close_pack' });
  // Le jeton posé sur le deck pour la liste des surcharges est retiré.
  await api({ cmd: 'apply', label: 'Jeton', ops: [{ op: 'unset_token', path: ['color', 'ink'] }], merge: null });
  assert.deepEqual(errors, []);
  console.log('éditeur de thème ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
