// Theme editor (t-554, t-1156): first the deck's agent opens Papier in its own tab (`open_theme`, the fake
// agent's `ouvre le thème avec une demande` turn, t-1191), an active theme tab whose composer holds the passed
// request as a draft after a reload, and no prompt was sent for it (t-1277); closed again. Then "Ouvrir" on the theme
// row of `t-1`'s "Packs du deck" opens Papier in its own theme tab, right after `t-1`, marked `data-kind="theme"`, with Essentiel's specimens in its pile; its parents are
// added, moved and removed (⌘Z included), a changed token shows on the `title` specimen, and "Terminer" closes the
// tab without a dialog, back on `t-1`, after saving the draft as a variant that extends Papier, without
// switching the deck's theme. Opening Papier again gives a new theme tab. "Nouveau thème" in the main menu opens
// an empty theme in a theme tab (`data-kind="theme"`), dropped from the library when closed untouched.
// Writes: a new library theme "Papier (variante)" extending Papier. Leaves: no pack or theme tab, `t-1`'s theme
// and pins unchanged.
// Contre `oku-slides serve fixtures/roadmap.json` (thème Papier). Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, closePacksPanel, openThemeTab } = require('./lib/atelier.cjs');
const { logOffset, logSince, waitForTool } = require('./lib/agentLog.cjs');
const { post } = require('./lib/http.cjs');
const { assertNoQuestion } = require('./lib/question.cjs');
const { stageSvg } = require('./lib/stage.cjs');
const { SLOW, untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const SURFACE = '#theme-panel [data-surface-id="host-theme"]';
// La ligne d'un jeton : le composant qui la porte a pour id le chemin du jeton suivi de `:row`.
const row = (token) => `${SURFACE} [data-component-id="${token}:row"]`;
const RED = /#ff0000/i;
const api = (p, tab, req) => post(p, '/api', { tab, req });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
const tabIds = async (p) => (await view(p)).windows[0].tabs.map((t) => t.id);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  // A theme tab closes without any question: no "quit without saving", no opener offer.
  p.on('dialog', (d) => {
    errors.push(`dialogue inattendu : ${d.message()}`);
    void d.dismiss();
  });
  await p.goto(URL);
  await closePackTabs(p);
  const deckBefore = await api(p, 't-1', { cmd: 'snapshot' });
  assert.equal(deckBefore.theme, 'papier', 't-1 ne commence pas sur Papier');

  // The deck's agent opens the theme (open_theme): an active theme tab on Papier, closed before the UI steps.
  const LOG = process.env.E2E_AGENT_LOG;
  assert.ok(LOG, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const from = logOffset(LOG);
  const asked = await post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text: 'ouvre le thème avec une demande', ui: {} });
  assert.equal(asked.error, undefined, `consigne « ouvre le thème avec une demande » refusée : ${asked.error}`);
  await waitForTool(p, from, 'open_theme');
  const activeThemeTab = async () => {
    const win = (await view(p)).windows[0];
    const active = win.tabs.find((t) => t.id === win.active);
    return active && active.id !== 't-1' && active.document.kind === 'theme' ? active.id : null;
  };
  const agentTab = await untilValue(p, activeThemeTab, (t) => t !== null, "l'agent n'ouvre pas d'onglet de thème actif", SLOW);
  const agentTheme = await api(p, agentTab, { cmd: 'snapshot' });
  assert.equal(agentTheme.theming?.theme.base.id, deckBefore.theme_pins.papier.id, "l'onglet ouvert par l'agent n'édite pas Papier");
  // The request waits in the theme tab's composer, replayed after a reload; nothing was prompted for it (t-1277).
  const REQUEST = 'Passe les titres en bleu.';
  await p.reload();
  const draft = p.locator('.notes .draft .draft-note');
  await draft.waitFor({ state: 'visible', timeout: SLOW });
  await untilValue(p, () => draft.innerText(), (t) => t === REQUEST, "le brouillon de l'onglet de thème ne porte pas la demande");
  const prompts = logSince(LOG, from).filter((m) => m.method === 'session/prompt');
  assert.equal(prompts.length, 1, `seule la consigne du deck devait partir : ${prompts.length} prompts`);
  assert.ok(!JSON.stringify(prompts).includes(REQUEST), "la demande est partie à l'agent sans que l'utilisateur l'envoie");
  // The turn may still stream: wait for its end (the fallback receipt end_turn puts in t-1's thread) before closing.
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  const agentTurn = p.locator('.agent-turn', { hasText: 'ouvre le thème' }).last();
  await agentTurn.locator('.a2ui-receipt .a2ui-title').waitFor({ timeout: SLOW });
  assert.equal(await agentTurn.locator('.a2ui-receipt .a2ui-title').innerText(), 'Thème ouvert.', "le tour « ouvre le thème » ne finit pas sur son reçu");
  await closePackTabs(p);

  // "Ouvrir" on the theme row: a theme tab right after t-1, of kind theme, named "…, thème".
  const themeTab = await openThemeTab(p, 't-1');
  const ids = await tabIds(p);
  assert.equal(ids.indexOf(themeTab), ids.indexOf('t-1') + 1, `l'onglet de thème n'est pas juste après t-1 : ${ids}`);
  assert.equal(await p.locator('.tab.active').getAttribute('data-kind'), 'theme', "l'onglet de thème n'a pas data-kind=theme");
  assert.match(await p.locator('.tab.active').getAttribute('aria-label'), /, thème$/, "l'onglet de thème n'est pas nommé « …, thème »");
  const opened = await api(p, themeTab, { cmd: 'snapshot' });
  assert.equal(opened.editing, null, "l'onglet de thème édite un pack");
  assert.equal(opened.theming?.theme.base.id, deckBefore.theme_pins.papier.id, "l'onglet de thème n'édite pas Papier");
  // Its pile: Essentiel's specimens, one per layout, fixed.
  assert.ok(opened.slides.length > 0 && opened.slides.every((s) => s.id.startsWith('layout-')), `spécimens attendus, pas ${opened.slides.map((s) => s.id)}`);
  assert.equal(await p.locator('.pile-column .thumb').count(), opened.slides.length);
  assert.equal(await p.getByRole('button', { name: /Nouvelle slide/ }).count(), 0, "pas d'ajout de slide dans un onglet de thème");
  assert.equal(await p.getByRole('tab', { name: 'Thème', exact: true }).count(), 0, 'un sous-onglet « Thème » reste');
  await p.waitForSelector(`${SURFACE} .a2ui-disclosure`);
  // Le spécimen du layout `title`, qui emploie l'accent.
  await p.click('.thumb:has(.thumb-name:text-is("title"))');
  await p.waitForSelector('.stage img');
  assert.doesNotMatch(await stageSvg(p), RED);

  // Le thème édité étend Papier.
  const panel = p.locator(SURFACE);
  await untilValue(p, () => panel.locator('[data-component-id="extends"] [data-component-id$=":name"]').first().innerText().catch(() => ''), (t) => t === 'Papier', "le panneau n'indique pas que le thème étend Papier");

  // Les parents du brouillon (t-1116) : Pro ajouté, monté, retiré, rendu par ⌘Z, puis retiré pour de bon.
  // Chaque ligne a ses boutons libellés en français ; le thème édité n'est pas proposé à l'ajout.
  const parents = () => api(p, themeTab, { cmd: 'snapshot' }).then((s) => s.theming.theme.extends.map((t) => t.name));
  const parentRow = (ns) => panel.locator(`[data-component-id="extends:${ns}"]`);
  const button = (ns, name) => parentRow(ns).getByRole('button', { name, exact: true });
  const addParent = panel.locator('[data-component-id="extends-add"] select');
  const editedName = opened.theming.theme.name;
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
  await untilValue(p, () => stageSvg(p), (svg) => RED.test(svg), 'le spécimen title ne montre pas le nouvel accent');
  // La surface relue garde la valeur envoyée.
  await untilValue(p, () => picker.inputValue(), (v) => v === '#ff0000', "la surface relue n'a pas gardé l'accent envoyé");

  // « Terminer » tout de suite, sans attendre l'enregistrement automatique : la fermeture enregistre d'abord, sans
  // dialogue, et rend t-1. Le brouillon est une variante qui étend Papier ; le thème du deck ne change pas.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await untilValue(p, () => view(p), (v) => !v.windows[0].tabs.some((t) => t.id === themeTab) && v.windows[0].active === 't-1', "l'onglet de thème ne se ferme pas");
  await p.waitForSelector('#theme-panel', { state: 'detached' });
  const after = await untilValue(
    p,
    () => api(p, 't-1', { cmd: 'snapshot' }),
    (s) => s.themes.some((th) => th.name === 'Papier (variante)' && th.extends.includes('papier')),
    "la bibliothèque n'a pas la variante « Papier (variante) » qui étend Papier",
  );
  assert.equal(after.theme, deckBefore.theme, 'le thème du deck a changé');
  assert.deepEqual(after.theme_pins, deckBefore.theme_pins, 'les épingles du thème du deck ont changé');
  assert.deepEqual([after.layouts, after.widgets], [deckBefore.layouts, deckBefore.widgets], 'les épingles du deck ont changé');
  // Le deck garde son thème : sa scène n'a pas l'accent rouge.
  await p.waitForSelector('.stage img');
  assert.doesNotMatch(await untilValue(p, () => stageSvg(p), (svg) => svg !== '', 'scène vide'), RED, 'la scène du deck prend l\'accent rouge');
  await closePacksPanel(p);

  // "Ouvrir" again: t-1 still pins Papier, so a new theme tab on Papier, never the closed one.
  const again = await openThemeTab(p, 't-1');
  assert.notEqual(again, themeTab, "l'onglet de thème fermé revient");
  const reopened = await api(p, again, { cmd: 'snapshot' });
  assert.deepEqual([reopened.theming?.theme.base.id, reopened.theming?.theme.name], [deckBefore.theme_pins.papier.id, 'Papier'], "le second onglet de thème n'est pas sur Papier");
  await p.getByRole('button', { name: 'Terminer' }).click();
  await untilValue(p, () => view(p), (v) => !v.windows[0].tabs.some((t) => t.id === again) && v.windows[0].active === 't-1', "le second onglet de thème ne se ferme pas");
  await closePacksPanel(p);

  // "Nouveau thème" from `t-1`'s main menu: an empty theme in a new theme tab. Closed untouched, it leaves the
  // library (read once back on `t-1`, whose next snapshot rereads the library index).
  const themesBefore = (await api(p, 't-1', { cmd: 'snapshot' })).themes.map((th) => th.namespace).sort();
  const count = (await tabIds(p)).length;
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Nouveau thème' }).click();
  await p.locator('.tab.active[data-kind="theme"]').waitFor();
  await p.waitForSelector('#theme-panel [data-surface-id="host-theme"]');
  const fresh = (await view(p)).windows[0].active;
  assert.notEqual(fresh, 't-1', "« Nouveau thème » n'ouvre pas d'onglet");
  assert.equal((await tabIds(p)).length, count + 1, '« Nouveau thème » n\'ouvre pas un seul onglet');
  await p.getByRole('button', { name: 'Terminer' }).click();
  await untilValue(p, () => view(p), (v) => !v.windows[0].tabs.some((t) => t.id === fresh) && v.windows[0].active === 't-1', "le nouveau thème ne se ferme pas");
  await untilValue(p, async () => (await api(p, 't-1', { cmd: 'snapshot' })).themes.map((th) => th.namespace).sort(),
    (l) => JSON.stringify(l) === JSON.stringify(themesBefore), 'le thème neuf fermé intact reste dans la bibliothèque');
  await assertNoQuestion(p);
  assert.deepEqual(errors, []);
  console.log('éditeur de thème ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
