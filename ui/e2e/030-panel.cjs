// Atelier d'un pack de layouts, dans son propre onglet de pack : la grille de `layout-blank` changée (`set_grid`,
// comme le fait l'agent : plus de carte de la slide), enregistrée dans un nouveau pack, se retrouve en le rouvrant.
// L'atelier se demande à l'agent (t-988). Au passage : pas de sous-onglet « Modèles » (sa scène est l'onglet
// « Layouts »), une miniature ramène la scène, l'onglet « Thème » montre les jetons à droite ; « Terminer » ferme
// l'onglet de pack et rend l'onglet du deck ; une frappe juste avant la croix est enregistrée (garde de fermeture).
// Writes: Essentiel (the `essai-pile` widget writes make a session version), a copy of Essentiel. Leaves: no pack
// tab; `t-1` pins an older Essentiel than the library's current one (its "Packs du deck" badge lit at its next snapshot).
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e (serveur neuf, port 4872).
// À la main : bun run e2e dans ui/, E2E_URL = adresse du serveur (défaut port 4870).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openAtelier, openPackTab: openFrom } = require('./lib/atelier.cjs');
const { post } = require('./lib/http.cjs');
const { until, untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const api = (p, tab, req) => post(p, '/api', { tab, req });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
const thumb = (name) => `.thumb:has(.thumb-name:text-is("${name}"))`;
/** La grille de `layout-blank` (colonnes, lignes), lue dans la session de `tab`. */
const grid = async (p, tab) => {
  const g = (await api(p, tab, { cmd: 'slide', id: 'layout-blank' })).slide?.params?.grid ?? {};
  return [g.columns, g.rows];
};
const stageHidden = (p) => p.evaluate(() => document.getElementById('stage-panel').hidden);
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
  // The only dialog: the widget-delete confirm of the pile (accepted). The copy is not pinned by `t-1`: no update offer.
  p.on('dialog', (d) => {
    const message = d.message();
    if (/^Supprimer le widget « essai-pile » \?$/.test(message)) {
      dialogs.push('widget-delete');
      void d.accept();
    } else {
      errors.push(`dialogue inattendu : ${message}`);
      void d.dismiss();
    }
  });
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');

  // 1. L'atelier s'ouvre en le demandant à l'agent (t-988), sur Essentiel (pack de layouts épinglé par le deck), dans son onglet.
  const packTab = await openAtelier(p, 't-1');
  assert.notEqual(packTab, 't-1', "l'atelier ne s'ouvre pas dans un onglet de pack");
  await until(p, async () => (await api(p, packTab, { cmd: 'snapshot' })).editing?.kind === 'layouts', "l'atelier ne s'ouvre pas sur un pack de layouts");
  const essentiel = (await api(p, packTab, { cmd: 'snapshot' })).editing.pack;
  assert.equal(await p.locator('.tab.active .tab-badge').innerText(), 'Pack', "l'onglet de pack n'a pas son badge");
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
  // Les sections : les jetons globaux (dont l'habillage), puis une par pack embarqué.
  const sections = await p.locator('#theme-panel [data-surface-id="host-theme"]').innerText();
  assert.match(sections, /Couleurs/);
  assert.match(sections, /Habillage/);
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

  // 2-3. La slide `layout-blank` : sa grille passe à 4 colonnes.
  await p.click(thumb('blank'));
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

  // 6. Rouvrir le nouveau pack (aucun deck ne l'épingle : par l'espace de travail), puis recharger la page.
  const reopened = await openPackTab(p, copy.pack);
  await p.reload();

  // 7. `layout-blank` garde ses 4 colonnes.
  await p.click(thumb('blank'));
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
  await p.click('.pile-widgets-row:has-text("garde") .pile-widgets-file');
  // The sub-tab's code shows on demand (its state is kept by the host).
  await p.waitForSelector('.widget-tab:not([hidden]) .widget-tab-preview');
  if ((await p.locator('.widget-tab:not([hidden])[data-code="shown"]').count()) === 0) await p.getByRole('button', { name: 'Code source' }).click();
  const editor = p.locator('.widget-tab:not([hidden]) .cm-content');
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
  assert.deepEqual(dialogs, ['widget-delete']);
  assert.deepEqual(errors, []);
  console.log('atelier de layouts ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
