// Panneau « Packs du deck » (t-1116) : ouvert par la barre d'outils au dock, ses trois sections,
// les refus de retrait (base, pack employé), l'ajout d'un pack de widgets de la bibliothèque, qui
// devient une section de la grille d'ajout et s'annule par ⌘Z, une carte de base ajoutée avec son
// texte de départ, et le thème choisi dans le panneau, annulé par ⌘Z. "Nouveau pack de widgets" in the main menu
// opens an empty pack in a pack tab (`data-kind="pack"`, empty pile and text) offered in the add list; its preview
// theme changes silently (no unsaved mark, ⌘Z keeps it); closed untouched, it leaves the add list.
// Contre `oku-slides serve fixtures/roadmap.json` lancé par `just e2e` (thème Papier : 200 ne change plus celui de `t-1`).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { closePackTabs, packTabShown } = require('./lib/atelier.cjs');
const { createLibraryWidgetPack, deleteLibraryPack } = require('./lib/packs.cjs');
const { stageSvg } = require('./lib/stage.cjs');
const { until, untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = 't-1';
const PANEL = '.agent-dock [data-surface-id="host-packs"]';

const api = (p, req) => post(p, '/api', { tab: TAB, req });
const snapshot = (p) => api(p, { cmd: 'snapshot' });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
/** Le composant `id` du panneau. */
const part = (p, id) => p.locator(`${PANEL} [data-component-id="${id}"]`);
/** Le texte du composant `id`, sans les marques d'isolement de Fluent ; `''` s'il manque. */
const text = (p, id) => part(p, id).first().innerText({ timeout: 1000 }).then((t) => t.replace(/[⁨⁩]/g, '').trim(), () => '');
/** Le bouton de la barre d'outils, avec ou sans la pastille des mises à jour. */
const packsButton = (p) => p.getByRole('button', { name: /^Packs du deck/ });
/** Rend le focus au document : ⌘Z n'est pas pris par un menu du panneau. */
const blur = (p) => p.evaluate(() => document.activeElement instanceof HTMLElement && document.activeElement.blur());

/** Makes `tab` active over HTTP and reloads the page (serve pushes no view). */
async function activate(p, tab) {
  const v = await post(p, '/api/workspace', { cmd: 'activate', tab });
  assert.equal(v.error, undefined, `activation de ${tab} refusée : ${v.error}`);
  await p.reload();
}

/** The options of the panel's "Ajouter depuis la bibliothèque" for widget packs; none when it is absent. */
const addable = (p) => part(p, 'widgets-add').locator('option').allTextContents();

/** Ouvre le panneau par la barre d'outils et attend ses sections. */
async function openPanel(p) {
  await packsButton(p).click();
  await part(p, 'theme-pick').waitFor();
}

/** Sélectionne la première slide qui accepte `kind` ; attend le bouton d'ajout actif. */
async function selectSlideAccepting(p, kind) {
  const snap = await snapshot(p);
  const at = snap.slides.findIndex((s) => {
    const accepts = snap.free_slots[s.kind];
    return accepts !== undefined && (accepts === null || accepts.includes(kind));
  });
  assert.ok(at >= 0, `aucune slide n'accepte ${kind}`);
  await p.locator('.thumb').nth(at).click();
  await until(p, () => p.evaluate(() => document.querySelector('.add-button')?.disabled === false), "le bouton d'ajout ne s'active pas");
  return snap.slides[at].id;
}

/** Ouvre la grille d'ajout ; rend la fenêtre, armée. */
async function openGrid(p) {
  await p.getByRole('button', { name: 'Ajouter sur la slide' }).click();
  const picker = p.getByRole('dialog', { name: 'Ajouter sur la slide' });
  await picker.waitFor();
  await p.waitForSelector('.widget-picker[data-armed]');
  return picker;
}

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  // Le panneau ne vit qu'en mode deck : aucun onglet de pack laissé par un spec précédent.
  await closePackTabs(p);
  await p.waitForSelector('.stage img');

  // 1. Le bouton de la barre ouvre le panneau au dock : Layouts, Widgets, Thème.
  await openPanel(p);
  assert.equal(await text(p, 'title'), 'Packs du deck');
  assert.equal(await text(p, 'layouts-title'), 'Layouts');
  assert.equal(await text(p, 'widgets-title'), 'Widgets');
  assert.equal(await text(p, 'theme-title'), 'Thème');

  // 2. base ne se retire pas (son refus remplace le bouton) ; essentiel non plus, ses layouts sont employés.
  await untilValue(p, () => text(p, 'widgets/base/refusal'), (t) => t === 'Chaque deck garde le pack de widgets de base.', 'base : refus de retrait absent');
  assert.equal(await part(p, 'widgets/base/remove').count(), 0, 'base : bouton « Retirer » montré');
  const inUse = await text(p, 'layouts/essentiel/refusal');
  assert.match(inUse, /^Le pack .+ est encore utilisé par \d+ éléments?\.$/, `essentiel : refus inattendu « ${inUse} »`);
  assert.equal(await part(p, 'layouts/essentiel/remove').count(), 0, 'essentiel : bouton « Retirer » montré');

  // 3. Un pack de widgets de la bibliothèque, ajouté depuis le panneau, puis annulé.
  const pack = await createLibraryWidgetPack(p, TAB, 'Acme');
  try {
    // L'onglet de pack ouvert puis fermé par HTTP : la page relit la vue, le panneau se rouvre, la bibliothèque relue.
    await p.reload();
    await until(p, async () => (await snapshot(p)).editing === null, "l'onglet ne revient pas au deck");
    await packsButton(p).waitFor();
    await openPanel(p);
    const add = part(p, 'widgets-add').locator('select');
    await until(p, async () => (await add.locator('option', { hasText: 'Acme' }).count()) === 1, 'Acme absent de « Ajouter depuis la bibliothèque »');
    await add.selectOption({ label: 'Acme' });
    await until(p, async () => (await snapshot(p)).widgets[pack.namespace] !== undefined, "Acme n'est pas épinglé par le deck");
    await untilValue(p, () => text(p, `widgets/${pack.namespace}/name`), (t) => t === 'Acme', "Acme n'apparaît pas dans le panneau");
    // Sa propre section dans la grille d'ajout.
    await selectSlideAccepting(p, 'base/card');
    let picker = await openGrid(p);
    await picker.getByRole('group', { name: 'Acme', exact: true }).waitFor();
    await p.keyboard.press('Escape');
    await picker.waitFor({ state: 'detached' });
    // ⌘Z le retire du deck, du panneau et de la grille.
    await blur(p);
    await p.keyboard.press('Control+z');
    await until(p, async () => (await snapshot(p)).widgets[pack.namespace] === undefined, "⌘Z ne retire pas Acme du deck");
    await part(p, `widgets/${pack.namespace}`).waitFor({ state: 'detached' });
    picker = await openGrid(p);
    assert.equal(await picker.getByRole('group', { name: 'Acme', exact: true }).count(), 0, 'Acme reste dans la grille');
    await p.keyboard.press('Escape');
    await picker.waitFor({ state: 'detached' });
  } finally {
    await deleteLibraryPack(p, TAB, pack.id);
  }

  // 4. Une carte de base ajoutée depuis la grille porte son titre de départ.
  const slide = await selectSlideAccepting(p, 'base/card');
  const picker = await openGrid(p);
  await picker.getByRole('group', { name: 'Base', exact: true }).getByRole('option', { name: /^Carte/ }).click();
  await picker.waitFor({ state: 'detached' });
  const undoLabel = () => p.evaluate(() => document.querySelector('[aria-label^="Annuler"]')?.getAttribute('aria-label'));
  await untilValue(p, undoLabel, (l) => l === 'Annuler : Ajouter « Carte »', "la carte n'est pas ajoutée (libellé d'annulation)");
  const slideJson = async () => JSON.stringify(await api(p, { cmd: 'slide', id: slide }));
  const added = await slideJson();
  assert.ok(added.includes('"base/card"') && added.includes('Titre de la carte'), `carte sans son titre de départ : ${added.slice(0, 300)}`);
  await blur(p);
  await p.keyboard.press('Control+z');
  await until(p, async () => !(await slideJson()).includes('"base/card"'), "⌘Z ne retire pas la carte");

  // 5. Pro choisi dans le panneau change la scène ; ⌘Z rend Papier, le thème du deck.
  const start = (await snapshot(p)).theme_name;
  assert.match(start ?? '', /^Papier/, `le deck ne part pas de Papier : ${start}`);
  const paper = await untilValue(p, () => stageSvg(p), (svg) => svg !== '', 'scène vide');
  await part(p, 'theme-pick').locator('select').selectOption({ label: 'Pro' });
  await until(p, async () => (await snapshot(p)).theme_name === 'Pro', "le thème du deck ne passe pas à Pro");
  await untilValue(p, () => stageSvg(p), (svg) => svg !== '' && svg !== paper, 'la scène ne change pas avec Pro');
  await blur(p);
  await p.keyboard.press('Control+z');
  await until(p, async () => (await snapshot(p)).theme_name === start, `⌘Z ne rend pas ${start}`);
  await untilValue(p, () => text(p, 'theme-name'), (t) => t === start, `le panneau ne montre pas ${start}`);

  // 6. "Nouveau pack de widgets" from the main menu: an empty pack in a new pack tab.
  const count = (await view(p)).windows[0].tabs.length;
  await blur(p);
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Nouveau pack de widgets' }).click();
  await packTabShown(p);
  const fresh = (await view(p)).windows[0].active;
  assert.notEqual(fresh, TAB, "« Nouveau pack de widgets » n'ouvre pas d'onglet");
  assert.equal((await view(p)).windows[0].tabs.length, count + 1, "« Nouveau pack de widgets » n'ouvre pas un seul onglet");
  const freshSnap = () => post(p, '/api', { tab: fresh, req: { cmd: 'snapshot' } });
  const made = (await freshSnap()).editing;
  assert.equal(made?.kind, 'widgets', `le nouvel onglet n'édite pas un pack de widgets : ${JSON.stringify(made)}`);
  assert.equal(await p.locator('.tab.active').getAttribute('data-kind'), 'pack');
  assert.equal(await p.locator('.pack-pile .thumb').count(), 0, 'la pile du pack neuf a des éléments');
  await p.locator('.pack-pile-empty', { hasText: "Ce pack n'a pas encore de widget." }).waitFor();
  // While its tab is open, the new pack is in the library: `t-1`'s add list offers it.
  await activate(p, TAB);
  await openPanel(p);
  await untilValue(p, () => addable(p), (l) => l.includes(made.name), `« ${made.name} » absent de « Ajouter depuis la bibliothèque »`);

  // Its preview theme: the select changes the tab's theme, silently (clean tab, nothing for ⌘Z to undo).
  await activate(p, fresh);
  await packTabShown(p);
  const { theme: shown, themes } = await freshSnap();
  const other = themes.find((th) => th.namespace !== shown && th.name === 'Pro') ?? themes.find((th) => th.namespace !== shown);
  assert.ok(other, `aucun autre thème que ${shown}`);
  await p.getByRole('combobox', { name: 'Thème d’aperçu' }).selectOption(other.namespace);
  await untilValue(p, async () => (await freshSnap()).theme, (th) => th === other.namespace, `le thème d'aperçu ne passe pas à ${other.namespace}`);
  assert.equal((await freshSnap()).dirty, false, "le thème d'aperçu rend le pack modifié");
  assert.equal(await p.locator('.tab.active .tab-dirty').count(), 0, "l'onglet du pack neuf montre une modification");
  await blur(p);
  await p.keyboard.press('ControlOrMeta+z');
  // négatif : une annulation fautive aurait le temps d'arriver.
  await p.waitForTimeout(500);
  assert.equal((await freshSnap()).theme, other.namespace, "⌘Z annule le thème d'aperçu");

  // Closed untouched, the new pack leaves the library: back on `t-1` (its next snapshot rereads the library
  // index), the add list no longer offers it.
  await p.locator('.tab.active .tab-close').click();
  await untilValue(p, () => view(p), (v) => !v.windows[0].tabs.some((t) => t.id === fresh), "l'onglet du pack neuf ne se ferme pas");
  await activate(p, TAB);
  await snapshot(p);
  await openPanel(p);
  await untilValue(p, () => addable(p), (l) => !l.includes(made.name), `« ${made.name} » reste dans « Ajouter depuis la bibliothèque »`);
  assert.equal((await api(p, { cmd: 'packs', kind: 'widgets' })).some((pack) => pack.id === made.pack), false, 'le pack neuf fermé intact reste dans la bibliothèque');

  await part(p, 'close').getByRole('button').click();
  await p.locator(PANEL).waitFor({ state: 'detached' });
  assert.deepEqual(errors, []);
  console.log('packs du deck ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
