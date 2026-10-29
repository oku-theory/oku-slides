// Workshop of a layout pack, in its own pack tab: the grid of `layout-blank` changed (`set_grid`, as the agent
// does: no slide card any more), saved as a new pack, is found again when reopening it. The workshop is asked of
// the agent (t-988). On the way: the pack tab carries `data-kind="pack"` and its "…, pack" name, no "Modèles" nor
// "Thème" sub-tab; a new layout is added to the pile, selected, then deleted through the confirmation dialog;
// "Terminer" closes the pack tab and gives the deck's tab back; `t-1`'s theme opens in a
// theme tab from "Packs du deck", its tokens on the right, closed clean; typing right before the × is saved (close guard).
// Writes: Essentiel (the `essai-pile` widget writes make a session version), a copy of Essentiel. Leaves: no pack
// or theme tab; `t-1` pins an older Essentiel than the library's current one (its "Packs du deck" badge lit at its next snapshot).
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e (serveur neuf, port 4872).
// À la main : bun run e2e dans ui/, E2E_URL = adresse du serveur (défaut port 4870).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openAtelier, openPackTab: openFrom, openThemeTab, closePacksPanel, packTabShown } = require('./lib/atelier.cjs');
const { post } = require('./lib/http.cjs');
const { until, untilValue } = require('./lib/until.cjs');
const { newPackItem, pileThumb, selectPackItem } = require('./lib/widget.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const api = (p, tab, req) => post(p, '/api', { tab, req });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
const thumb = (name) => `.thumb:has(.thumb-name:text-is("${name}"))`;
/** La grille de `layout-blank` (colonnes, lignes), lue dans la session de `tab`. */
const grid = async (p, tab) => {
  const g = (await api(p, tab, { cmd: 'slide', id: 'layout-blank' })).slide?.params?.grid ?? {};
  return [g.columns, g.rows];
};
/** Opens pack `pack` from `t-1` over HTTP; returns its pack tab. */
const openPackTab = (p, pack, widget = null) => openFrom(p, 't-1', pack, widget);
/** Waits until `tab` left the view and `t-1` is active again. */
const backToOpener = (p, tab, why) =>
  untilValue(p, () => view(p), (v) => !v.windows[0].tabs.some((t) => t.id === tab) && v.windows[0].active === 't-1', why);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const dialogs = [];
  p.on('pageerror', (e) => errors.push(e.message));
  // No browser dialog: the pile deletes through its own confirmation; the copy is not pinned by `t-1`: no update offer.
  p.on('dialog', (d) => {
    dialogs.push(d.message());
    void d.dismiss();
  });
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');

  // 1. L'atelier s'ouvre en le demandant à l'agent (t-988), sur Essentiel (pack de layouts épinglé par le deck), dans son onglet.
  const packTab = await openAtelier(p, 't-1');
  assert.notEqual(packTab, 't-1', "l'atelier ne s'ouvre pas dans un onglet de pack");
  await until(p, async () => (await api(p, packTab, { cmd: 'snapshot' })).editing?.kind === 'layouts', "l'atelier ne s'ouvre pas sur un pack de layouts");
  const essentiel = (await api(p, packTab, { cmd: 'snapshot' })).editing.pack;
  const tabEl = p.locator('.tab.active');
  assert.equal(await tabEl.getAttribute('data-kind'), 'pack', "l'onglet de pack n'a pas data-kind=pack");
  assert.match(await tabEl.getAttribute('aria-label'), /, pack$/, "l'onglet de pack n'est pas nommé « …, pack »");
  assert.equal(await p.locator('.tab-badge').count(), 0, 'un badge d\'onglet reste');
  await p.waitForSelector(thumb('blank'));
  assert.equal(await p.getByRole('tab', { name: 'Modèles', exact: true }).count(), 0, 'plus de sous-onglet Modèles');
  assert.equal(await p.getByRole('button', { name: /Nouvelle slide/ }).count(), 0, "pas d'ajout de slide en mode pack");
  // The pile's trash buttons delete items, each named after its own; none deletes a slide.
  assert.equal(await p.getByRole('button', { name: 'Supprimer la slide' }).count(), 0, 'pas de suppression de slide en mode pack');
  assert.equal(await p.locator('.pack-pile .thumb-remove').count(), await p.locator('.pack-pile .thumb').count(), 'une miniature sans corbeille');

  // A new layout in the pile, selected, its view in the sheet area; another thumbnail selects that item.
  await newPackItem(p, 'essai-pile');
  await p.waitForSelector('.widget-tab .widget-tab-preview');
  await selectPackItem(p, 'title');
  // Its trash button asks in the dialog (layout wording), armed after 500 ms; confirmed, the item leaves the pile.
  await pileThumb(p, 'essai-pile').locator('xpath=..').locator('.thumb-remove').click();
  const confirm = p.getByRole('alertdialog');
  assert.match(await confirm.locator('.confirm-message').innerText(), /^Supprimer le layout « \u2068?essai-pile\u2069? » \?$/);
  await p.waitForSelector('.confirm-dialog[data-armed]');
  await confirm.getByRole('button', { name: 'Supprimer', exact: true }).click();
  await confirm.waitFor({ state: 'detached' });
  await pileThumb(p, 'essai-pile').waitFor({ state: 'detached' });

  // No "Thème" sub-tab: a theme opens in its own tab (step 5b).
  assert.equal(await p.getByRole('tab', { name: 'Thème', exact: true }).count(), 0, 'un sous-onglet « Thème » reste');
  // Le menu ⋯ du mode pack n'offre que la création de packs et de thèmes, l'aide et « À propos » : le reste se demande
  // à l'agent (t-988, t-1236, t-1157).
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.waitForSelector('[role="menu"]');
  assert.deepEqual(await p.locator('[role="menu"] [role="menuitem"] > span').allTextContents(),
    ['Nouveau pack de widgets', 'Nouveau pack de layouts', 'Nouveau thème', 'Aide', 'À propos d’Oku Slides']);
  // « À propos » : la version ; derrière lui, Page↓ ne change pas de slide (t-1236).
  await p.getByRole('menuitem', { name: 'À propos d’Oku Slides' }).click();
  const about = p.getByRole('dialog', { name: 'À propos d’Oku Slides' });
  await p.waitForSelector('.about-dialog[data-armed]');
  assert.match(await about.locator('.about-version').innerText(), /^Version \u2068?\d+\.\d+\.\d+\u2069?$/);
  await p.keyboard.press('PageDown');
  await p.keyboard.press('Escape');
  await about.waitFor({ state: 'detached' });
  assert.equal(await p.locator(thumb('title')).getAttribute('aria-current'), 'true', 'Page↓ a changé de slide derrière « À propos »');

  // 2-3. La slide `layout-blank` : sa grille passe à 4 colonnes.
  await selectPackItem(p, 'blank');
  await api(p, packTab, { cmd: 'apply', label: 'Grille', ops: [{ op: 'set_grid', slide: 'layout-blank', columns: 4 }] });
  const [, rows] = await untilValue(p, () => grid(p, packTab), (g) => g[0] === 4, 'la grille ne passe pas à 4 colonnes');

  // 4. Enregistrer une copie (plus au menu : l'agent, outil `save_pack`, t-988 ; ici par la session) :
  // un nouveau pack, que l'onglet édite désormais.
  await api(p, packTab, { cmd: 'save_pack', as_new: true });
  const copy = await untilValue(p, async () => (await api(p, packTab, { cmd: 'snapshot' })).editing, (e) => !!e && e.pack !== essentiel && !e.builtin, "la copie n'est pas éditée");

  // The essai-pile writes made a session version of Essentiel, now the library's current one: `t-1` pins an older one.
  const current = (await api(p, 't-1', { cmd: 'packs', kind: 'layouts' })).find((pack) => pack.id === essentiel);
  assert.notEqual(current?.version, (await api(p, 't-1', { cmd: 'snapshot' })).layouts.essentiel.version, "t-1 épingle déjà l'Essentiel courant");

  // 5. Terminer : l'onglet de pack se ferme, l'onglet du deck revient.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await backToOpener(p, packTab, "l'onglet de pack ne se ferme pas");
  await p.waitForSelector('.stage img');

  // 5b. The theme of t-1 in its own tab, by "Ouvrir" in "Packs du deck": the tokens on the right, in place of the
  // thread (the global sections, wrapping included); a specimen click keeps the panel. Closed clean, nothing written.
  const themeTab = await openThemeTab(p, 't-1');
  await p.waitForSelector('#theme-panel [data-surface-id="host-theme"] .a2ui-disclosure');
  assert.equal(await p.locator('.notes #theme-panel').count(), 1);
  const sections = await p.locator('#theme-panel [data-surface-id="host-theme"]').innerText();
  assert.match(sections, /Couleurs/);
  assert.match(sections, /Habillage/);
  await p.click(thumb('blank'));
  await untilValue(p, () => p.locator(thumb('blank')).getAttribute('aria-current'), (v) => v === 'true', 'le spécimen cliqué ne devient pas le courant');
  // négatif : un retrait fautif du panneau aurait le temps d'arriver.
  await p.waitForTimeout(300);
  assert.equal(await p.locator('.notes #theme-panel').count(), 1, 'un clic sur un spécimen retire le panneau du thème');
  await p.getByRole('button', { name: 'Terminer' }).click();
  await backToOpener(p, themeTab, "l'onglet de thème ne se ferme pas");
  await p.waitForSelector('#theme-panel', { state: 'detached' });
  await closePacksPanel(p);

  // 6. Rouvrir le nouveau pack (aucun deck ne l'épingle : par l'espace de travail), puis recharger la page.
  const reopened = await openPackTab(p, copy.pack);
  await p.reload();
  await packTabShown(p);

  // 7. `layout-blank` garde ses 4 colonnes.
  await selectPackItem(p, 'blank');
  await untilValue(p, () => grid(p, reopened), (g) => g[0] === 4 && g[1] === rows, 'la grille de layout-blank ne garde pas ses 4 colonnes');
  await p.getByRole('button', { name: 'Terminer' }).click();
  await backToOpener(p, reopened, "l'onglet rouvert ne se ferme pas");

  // 8. Close guard: typing right before the × is saved. A new widget `garde`, from the copy's first widget's source (it compiles).
  const guarded = await openPackTab(p, copy.pack);
  const first = (await api(p, guarded, { cmd: 'widget_status' })).find((w) => w.name !== '_commun').name;
  const source = (await api(p, guarded, { cmd: 'widget_source', name: first })).replace(`widget("${first}"`, 'widget("garde"');
  const set = await api(p, guarded, { cmd: 'set_widget', name: 'garde', source });
  assert.ok(!set?.error, `set_widget garde refusé : ${JSON.stringify(set).slice(0, 300)}`);
  await p.reload();
  await packTabShown(p);
  await selectPackItem(p, 'garde');
  // The item's code shows on demand (its state is kept by the host).
  await p.waitForSelector('.widget-tab .widget-tab-preview');
  if ((await p.locator('.widget-tab[data-code="shown"]').count()) === 0) await p.getByRole('button', { name: 'Code source' }).click();
  const editor = p.locator('.widget-tab .cm-content');
  await editor.click();
  await p.keyboard.press('ControlOrMeta+End');
  await p.keyboard.press('Enter');
  await p.keyboard.type('// garde');
  const clicked = Date.now();
  await p.locator('.tab.active .tab-close').click();
  // A slow render under load may stall the guard: the notice shows (it clears after 6 s), a second × after 30 s closes.
  const gone = () => view(p).then((v) => !v.windows[0].tabs.some((t) => t.id === guarded));
  const notice = p.locator('.workspace-error', { hasText: 'Cet onglet enregistre encore' });
  let stalled = false;
  await until(p, async () => (stalled = await notice.isVisible()) || (await gone()), "l'onglet de pack gardé ne se ferme pas", 20_000);
  if (stalled) {
    await p.waitForTimeout(Math.max(0, clicked + 31_000 - Date.now()));
    await p.locator('.tab.active .tab-close').click();
    await until(p, gone, "l'onglet de pack gardé ne se ferme pas au second clic", 20_000);
  }
  const back = await openPackTab(p, copy.pack, 'garde');
  assert.match(await api(p, back, { cmd: 'widget_source', name: 'garde' }), /\/\/ garde/, "la frappe d'avant la croix est perdue");
  await post(p, '/api/workspace', { cmd: 'close', tab: back, discard: true });
  await p.reload();
  await p.waitForSelector('.stage img');

  assert.equal((await view(p)).windows[0].active, 't-1');
  assert.deepEqual(dialogs, []);
  assert.deepEqual(errors, []);
  console.log('atelier de layouts ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
