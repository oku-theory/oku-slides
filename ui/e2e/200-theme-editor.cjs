// Theme editor (t-554): the "Thème" sub-tab of Essentiel's pack tab, a changed token shows on its stage, and ✓
// saves the draft as a variant of the base theme (it extends it) in the library, without switching the deck's
// theme; the panel shows what the theme extends and the deck's overrides, its parents are added, moved and
// removed (⌘Z included); the closed pack tab's update offer is dismissed; the Thème of a widget pack shows one
// preview per widget. Writes: Essentiel's theme sub-tab (a pack version and a theme version). Leaves: no pack tab,
// `t-1`'s theme and pins unchanged.
// Contre `oku-slides serve fixtures/roadmap.json` (thème Papier). Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openPackTab: openFrom } = require('./lib/atelier.cjs');
const { post } = require('./lib/http.cjs');
const { stageSvg } = require('./lib/stage.cjs');
const { untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const SURFACE = '#theme-panel [data-surface-id="host-theme"]';
// La ligne d'un jeton : le composant qui la porte a pour id le chemin du jeton suivi de `:row`.
const row = (token) => `${SURFACE} [data-component-id="${token}:row"]`;
const RED = /#ff0000/i;
const api = (p, tab, req) => post(p, '/api', { tab, req });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
/** Opens pack `pack` from `t-1` over HTTP; returns its pack tab. */
const openPackTab = (p, pack, widget = null) => openFrom(p, 't-1', pack, widget);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  const dialogs = [];
  // The only dialog: the update offer of the closed pack tab, dismissed.
  p.on('dialog', (d) => {
    if (/^Mettre à jour « \u2068.+\u2069 » avec cette version du pack « \u2068Essentiel\u2069 » \?$/.test(d.message())) dialogs.push('offer');
    else errors.push(`dialogue inattendu : ${d.message()}`);
    void d.dismiss();
  });
  await p.goto(URL);
  await closePackTabs(p);
  // Le thème se voit sur des slides : Essentiel (pack de layouts), dans son onglet de pack ouvert par l'espace de travail.
  let snap = await api(p, 't-1', { cmd: 'snapshot' });
  // Un jeton posé sur le deck : l'éditeur le met de côté et le liste, sans l'appliquer.
  await api(p, 't-1', { cmd: 'apply', label: 'Jeton', ops: [{ op: 'set_token', path: ['color', 'ink'], value: '#202020' }], merge: null });
  const packTab = await openPackTab(p, snap.layouts.essentiel.id);
  assert.equal((await api(p, packTab, { cmd: 'snapshot' })).editing?.kind, 'layouts');
  await p.reload();
  assert.equal(await p.locator('.tab.active .tab-badge').innerText(), 'Pack', "l'onglet de pack n'a pas son badge");
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
  const parents = () => api(p, packTab, { cmd: 'snapshot' }).then((s) => s.editing.theme.extends.map((t) => t.name));
  const parentRow = (ns) => panel.locator(`[data-component-id="extends:${ns}"]`);
  const button = (ns, name) => parentRow(ns).getByRole('button', { name, exact: true });
  const addParent = panel.locator('[data-component-id="extends-add"] select');
  const editedName = (await api(p, packTab, { cmd: 'snapshot' })).editing.theme.name;
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

  // The pack tab's preview renders the variant.
  await untilValue(p, () => stageSvg(p), (svg) => RED.test(svg), "l'aperçu de l'onglet de pack perd l'accent rouge");

  // ✓ tout de suite, sans attendre l'enregistrement automatique : la fermeture enregistre d'abord, et aucune
  // confirmation « quitter sans enregistrer » ne s'ouvre ; l'offre de mise à jour du deck est refusée.
  // Le brouillon est enregistré dans la bibliothèque en variante qui étend Papier ; le thème du deck ne change pas.
  const deckBefore = await api(p, 't-1', { cmd: 'snapshot' });
  await p.getByRole('button', { name: 'Terminer' }).click();
  await untilValue(p, () => view(p), (v) => !v.windows[0].tabs.some((t) => t.id === packTab) && v.windows[0].active === 't-1', "l'onglet de pack ne se ferme pas");
  await p.waitForSelector('#theme-panel', { state: 'detached' });
  const after = await untilValue(
    p,
    () => api(p, 't-1', { cmd: 'snapshot' }),
    (s) => s.themes.some((th) => th.name === 'Papier (variante)' && th.extends.includes('papier')),
    "la bibliothèque n'a pas la variante « Papier (variante) » qui étend Papier",
  );
  assert.deepEqual(dialogs, ['offer']);
  assert.equal(after.theme, deckBefore.theme, 'le thème du deck a changé');
  assert.deepEqual([after.layouts, after.widgets], [deckBefore.layouts, deckBefore.widgets], 'les épingles du deck ont changé');
  // Le deck garde son thème : sa scène n'a pas l'accent rouge.
  await p.waitForSelector('.stage img');
  assert.doesNotMatch(await untilValue(p, () => stageSvg(p), (svg) => svg !== '', 'scène vide'), RED, 'la scène du deck prend l\'accent rouge');

  // Le thème d'un pack de widgets : une slide d'aperçu par widget, hors de l'onglet seulement le vide.
  const widgetPacks = await api(p, 't-1', { cmd: 'packs', kind: 'widgets' });
  const base = widgetPacks.find((pack) => pack.namespace === 'base');
  assert.ok(base, 'le pack de widgets base manque');
  const widgetTab = await openPackTab(p, base.id);
  const widgetPack = await api(p, widgetTab, { cmd: 'snapshot' });
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
  assert.equal((await api(p, widgetTab, { cmd: 'snapshot' })).slides.length, previews.length);
  await post(p, '/api/workspace', { cmd: 'close', tab: widgetTab, discard: true });
  // Le jeton posé sur le deck pour la liste des surcharges est retiré.
  await api(p, 't-1', { cmd: 'apply', label: 'Jeton', ops: [{ op: 'unset_token', path: ['color', 'ink'] }], merge: null });
  assert.deepEqual(errors, []);
  console.log('éditeur de thème ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
