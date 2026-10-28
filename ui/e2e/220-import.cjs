// PowerPoint import in the app: "Ouvrir" on a .pptx (alto) opens the import dialog; as a faithful copy
// with fine-tuning, the imported tab opens and the fake agent plays its fine-tuning turn (import_report,
// import_diff of slide 7, report); its deck-only layout pack is edited in a deck-pack tab linked to the
// deck (saves forwarded to the deck, undo refused there while it is open), its theme is picked again
// after Papier with its overrides kept, and opens in a theme tab ("Ouvrir" in "Packs du deck"), closed
// without writing; the deck's × closes its deck-pack tab with it; then an import on my layouts
// (Essentiel by default, no agent). Writes: the imported decks and their deck-only pack versions.
// Leaves: the retrofit tab, no pack, deck-pack or theme tab.
// Against `oku-slides serve` started by `just e2e`: fake agent "Faux" chosen, its log in E2E_AGENT_LOG
// (the MCP results are read there). The witness (office2pdf) really runs: long waits.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePacksPanel, openAtelier, openThemeTab } = require('./lib/atelier.cjs');
const agentLog = require('./lib/agentLog.cjs');
const path = require('node:path');
const { post } = require('./lib/http.cjs');
const { SLOW, until, untilValue } = require('./lib/until.cjs');
const fs = require('node:fs/promises');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const ALTO = path.resolve(__dirname, '..', '..', 'fixtures', 'import', 'alto-wealth.pptx');

const api = (p, tab, req) => post(p, '/api', { tab, req });

const logOffset = () => agentLog.logOffset(LOG);
const logSince = (from) => agentLog.logSince(LOG, from);

/** Les résultats MCP (`mcp/result`) notés pour `tool` depuis `from`, dans l'ordre. */
const mcpResults = (tool, from) => agentLog.mcpResults(LOG, from, tool);

/** Le dernier résultat MCP de `tool` noté depuis `from`, décodé (CallToolResult) ; attend qu'il soit noté. */
const lastResult = async (p, tool, from) => {
  const outs = await untilValue(p, () => mcpResults(tool, from), (o) => o.length > 0, `aucun résultat MCP pour ${tool}`, SLOW);
  const out = outs[outs.length - 1];
  try {
    return JSON.parse(out);
  } catch {
    assert.fail(`${tool} : résultat illisible : ${out.slice(0, 300)}`);
  }
};

/** « Ouvrir » du menu, le chemin donné à l'invite du mode web : le dialogue d'import, armé (`data-armed` : clics des 500 premières ms ignorés). */
const openImport = async (p, file) => {
  p.once('dialog', (d) => d.accept(file));
  await p.getByRole('button', { name: 'Menu' }).click();
  await p.getByRole('menuitem', { name: 'Ouvrir' }).click();
  const dialog = p.getByRole('dialog', { name: 'Importer une présentation PowerPoint' });
  await dialog.waitFor();
  await p.locator('.import-dialog[data-armed]').waitFor();
  return dialog;
};

/**
 * Clique « Importer » ; rend l'onglet importé, actif dans la fenêtre (réponse de `import_tab`), et
 * la place du journal du faux agent juste avant l'import (ce qui suit vient de cet onglet).
 */
const submitImport = async (p, dialog) => {
  const offset = logOffset();
  const reply = p.waitForResponse(
    (r) => r.url().endsWith('/api/workspace') && (r.request().postData() ?? '').includes('"import_tab"'),
    { timeout: SLOW },
  );
  await dialog.getByRole('button', { name: 'Importer', exact: true }).click();
  const response = await reply;
  const view = await response.json();
  assert.ok(response.ok(), `import refusé : ${JSON.stringify(view)}`);
  await dialog.waitFor({ state: 'detached', timeout: SLOW });
  return { tab: view.windows.find((w) => w.label === 'w-1').active, offset };
};

/** L'onglet `id` de la fenêtre, une fois enregistré par l'enregistrement automatique (un fichier, rien de modifié). */
const savedTab = (p, id) =>
  untilValue(
    p,
    async () => (await post(p, '/api/workspace', { cmd: 'view' })).windows.flatMap((w) => w.tabs).find((t) => t.id === id),
    (t) => !!t?.document?.path && !t.dirty,
    `onglet ${id} jamais enregistré`,
    SLOW,
  );

/** Aucun tour ne part depuis `from` : ni prompt noté par le faux agent pendant `ms`, ni tour à l'écran. */
const noTurnSince = async (p, from, ms) => {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    const prompts = logSince(from).filter((m) => m.method === 'session/prompt');
    assert.deepEqual(prompts, [], 'aucun tour attendu sans peaufinage');
    await p.waitForTimeout(250);
  }
  assert.equal(await p.locator('.agent-turn').count(), 0, 'aucun tour attendu à l\'écran');
};

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');

  // 1. Copie fidèle (le défaut) avec le peaufinage (coché par défaut).
  const dialog = await openImport(p, ALTO);
  assert.equal(await dialog.locator('.import-file').innerText(), 'alto-wealth.pptx');
  assert.ok(await dialog.locator('input[name=import-mode][value=hifi]').isChecked(), 'copie fidèle par défaut');
  assert.equal(await dialog.locator('.import-target').count(), 0, 'pas de pack ni de thème en copie fidèle');
  const tune = dialog.getByRole('checkbox', { name: "Peaufiner avec l'agent" });
  assert.ok((await tune.isChecked()) && (await tune.isEnabled()), 'peaufinage coché et disponible');
  const { tab: hifi, offset: hifiLog } = await submitImport(p, dialog);
  assert.notEqual(hifi, 't-1');
  const snap = await api(p, hifi, { cmd: 'snapshot' });
  assert.equal(snap.slides.length, 8);
  assert.ok(snap.deck_pack, 'le pack généré est propre au deck');
  // Le thème de l'import est le thème intégré `neutre`, et le style généré tient dans `deck.tokens`.
  assert.equal(snap.theme, 'neutre', "l'import n'épingle pas neutre");
  assert.ok(snap.theme_pins.neutre, 'neutre n\'est pas épinglé par le deck');
  const importTheme = snap.theme_pins.neutre;
  await p.waitForSelector('.stage img');
  // Le deck importé est neuf : l'enregistrement automatique lui donne un fichier, au nom du .pptx.
  const hifiTab = await savedTab(p, hifi);
  assert.equal(hifiTab.title, 'alto-wealth');
  assert.match(hifiTab.document.path, /alto-wealth[^/]*\.oku-slides\.pdf$/);

  // Le tour de fine-tuning : la bulle du prompt court, puis le reçu du `report`.
  const turn = p.locator('.agent-turn', { hasText: 'Peaufiner la présentation importée' });
  await turn.locator('.a2ui-prompt').waitFor({ timeout: SLOW });
  const receipt = turn.locator('.a2ui-receipt');
  await receipt.locator('.a2ui-title').waitFor({ timeout: SLOW });
  assert.equal(await receipt.locator('.a2ui-title').innerText(), 'Import peaufiné');
  assert.equal(await receipt.locator('.a2ui-hint').innerText(), "slide 7 comparée à l'original");
  await receipt.locator('.a2ui-receipt-thumb').waitFor();
  await turn.locator('.a2ui-timeline-summary').click();
  await turn.locator('.a2ui-step-toggle').click();
  assert.deepEqual(await turn.locator('.a2ui-call-label').allInnerTexts(), ["Lire le rapport d'import", 'Comparer la slide 7']);

  // Ce que les outils ont rendu à l'agent : le rapport hi-fi d'alto, la slide 7 signalée (pire région non suspecte 15,71, seuil 7)…
  const report = await lastResult(p, 'import_report', hifiLog);
  assert.ok(!report.isError, `import_report en erreur : ${JSON.stringify(report).slice(0, 300)}`);
  const body = JSON.parse(report.content.find((c) => c.type === 'text').text);
  assert.equal(body.mode, 'hifi');
  const s7 = body.slides.find((s) => s.slide === 's7');
  assert.ok(s7, `s7 absente de la file : ${JSON.stringify(body.slides)}`);
  assert.equal(s7.state, 'pending');
  // … puis le diff de la slide 7 : deux PNG (la référence, le rendu Oku) et ses régions notées.
  const diff = await lastResult(p, 'import_diff', hifiLog);
  assert.ok(!diff.isError, `import_diff en erreur : ${JSON.stringify(diff).slice(0, 300)}`);
  const images = diff.content.filter((c) => c.type === 'image');
  assert.equal(images.length, 2);
  assert.ok(images.every((i) => i.mimeType === 'image/png' && i.data.length > 0), 'deux PNG non vides');
  const regions = JSON.parse(diff.content.find((c) => c.type === 'text').text).regions;
  assert.ok(regions.some((r) => r.score > body.threshold && !r.suspect), `aucune région au-delà du seuil : ${JSON.stringify(regions)}`);
  const receiptCall = await lastResult(p, 'report', hifiLog);
  assert.ok(!receiptCall.isError, `report en erreur : ${JSON.stringify(receiptCall).slice(0, 300)}`);

  // Le tour n'a rien changé : la seule annulation est celle de l'import, qui vide l'onglet.
  const after = await api(p, hifi, { cmd: 'snapshot' });
  assert.equal(after.slides.length, 8);
  await p.locator('.stage').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press('Control+z');
  await until(p, async () => (await api(p, hifi, { cmd: 'snapshot' })).slides.length === 0, "l'import ne s'annule pas en une étape");

  await p.keyboard.press('Control+Shift+z');
  await until(p, async () => (await api(p, hifi, { cmd: 'snapshot' })).slides.length === 8, "l'import ne se refait pas");
  // L'atelier édite le pack de layouts du deck seul : enregistré dans le deck, jamais dans la bibliothèque.
  const library = async () => [await api(p, hifi, { cmd: 'packs', kind: 'layouts' }), await api(p, hifi, { cmd: 'packs', kind: 'widgets' })];
  const packsBefore = await library();
  // Le deck épingle `base`, `essentiel` et son propre pack de layouts : celui-ci se lit par son espace de noms.
  const pinOf = (snap) => snap.layouts['deck-layouts'];
  const packBefore = pinOf(await api(p, hifi, { cmd: 'snapshot' }));
  assert.ok(packBefore, 'le deck importé épingle son pack de layouts');
  // The deck's overrides that the import wrote (deck.tokens), counted on the theme row of "Packs du deck".
  const PACKS = '[data-surface-id="host-packs"]';
  const deckOverrides = async () => {
    await p.getByRole('button', { name: /^Packs du deck/ }).click();
    const row = p.locator(`${PACKS} [data-component-id="theme-overrides"]`);
    await row.waitFor();
    return (await row.innerText()).replace(/[⁨⁩]/g, '').trim();
  };
  const overridesAtImport = await deckOverrides();
  assert.match(overridesAtImport, /^[1-9]\d* surcharges? du deck$/, `l'import n'écrit pas de jetons dans deck.tokens : « ${overridesAtImport} »`);
  // L'atelier demandé à l'agent (t-988) ouvre le pack de layouts propre du deck dans un onglet lié au deck, juste après lui.
  const tabsOf = async () => (await post(p, '/api/workspace', { cmd: 'view' })).windows[0];
  const countBefore = (await tabsOf()).tabs.length;
  const packTab = await openAtelier(p, hifi);
  const opened = await tabsOf();
  const order = (win) => win.tabs.map((t) => t.id);
  assert.equal(opened.tabs.length, countBefore + 1, "l'atelier n'ouvre pas un onglet de plus");
  assert.equal(opened.active, packTab, "l'onglet du pack n'est pas actif");
  assert.equal(order(opened).indexOf(packTab), order(opened).indexOf(hifi) + 1, `l'onglet du pack n'est pas juste après l'onglet importé : ${order(opened)}`);
  assert.deepEqual(opened.tabs.find((t) => t.id === packTab).document, { kind: 'deck_pack', deck_tab: hifi, pack: packBefore.id }, "l'onglet n'édite pas le pack du deck seul");
  const editing = await api(p, packTab, { cmd: 'snapshot' });
  assert.equal(editing.editing?.deck_only, true, "l'atelier ne s'ouvre pas sur le pack du deck seul");
  // A layouts pack: its layouts are its widgets. The first one, by its bare name, as open_pack takes it.
  const firstLayout = editing.pack_widgets[0].split('/').pop();
  await p.locator('.brand-note', { hasText: 'Enregistré dans cette présentation seulement' }).waitFor();
  assert.match(await p.locator('.label .brand').textContent(), / · pack de layouts$/);
  // The hifi layouts have no title parameter: the edit renames the pack. The autosave then forwards the version to the deck.
  await p.locator('.doc-name-button').click();
  await p.getByRole('textbox', { name: 'Titre de la présentation' }).fill('Alto (layouts)');
  await p.keyboard.press('Enter');
  await until(p, async () => (await api(p, packTab, { cmd: 'snapshot' })).title === 'Alto (layouts)', "le pack n'est pas renommé");
  const forwarded = await untilValue(
    p,
    async () => pinOf(await api(p, hifi, { cmd: 'snapshot' })),
    (pin) => pin?.id === packBefore.id && pin.version !== packBefore.version,
    "le deck n'épingle pas la version enregistrée dans l'onglet du pack",
    SLOW,
  );
  // Back on the deck: it pins that version, the library has no new pack.
  await post(p, '/api/workspace', { cmd: 'activate', tab: hifi });
  await p.reload();
  await p.waitForSelector('.stage img');
  const back = await api(p, hifi, { cmd: 'snapshot' });
  assert.ok(back.deck_pack, 'le pack enregistré reste propre au deck');
  assert.deepEqual(pinOf(back), forwarded, 'le deck passe à la version enregistrée');
  assert.deepEqual(await library(), packsBefore, 'la bibliothèque ne change pas');
  // An undo of the pack step in the deck is refused while the pack tab is open: the toast names the pack and the tab.
  const packName = (await api(p, packTab, { cmd: 'snapshot' })).editing.name;
  const tabTitle = (await tabsOf()).tabs.find((t) => t.id === packTab).title;
  const fr = await fs.readFile(path.resolve(__dirname, '..', '..', 'crates', 'oku-slides-i18n', 'src', 'locales', 'fr', 'session.ftl'), 'utf8');
  const template = fr.match(/^session-undo-in-pack-tab = (.*)$/m)?.[1];
  assert.ok(template, 'session-undo-in-pack-tab absent de fr/session.ftl');
  const refusal = template.replace('{ $pack }', packName).replace('{ $tab }', tabTitle);
  await p.locator('.stage').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press('Control+z');
  const toast = p.locator('.toast.error');
  await toast.waitFor();
  assert.equal((await toast.innerText()).replace(/[⁨⁩]/g, ''), refusal, "l'annulation n'est pas refusée par l'onglet du pack");
  assert.deepEqual(pinOf(await api(p, hifi, { cmd: 'snapshot' })), forwarded, "l'annulation refusée change l'épingle");
  // "Terminer" in the pack tab closes it, back on the deck; the pack step then undoes from the deck.
  await post(p, '/api/workspace', { cmd: 'activate', tab: packTab });
  await p.reload();
  await p.getByRole('button', { name: 'Terminer' }).click();
  await until(p, async () => {
    const win = await tabsOf();
    return win.active === hifi && !win.tabs.some((t) => t.id === packTab);
  }, "l'onglet du pack ne se ferme pas sur l'onglet importé");
  await p.waitForSelector('.stage img');
  await p.locator('.stage').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press('Control+z');
  await until(p, async () => pinOf(await api(p, hifi, { cmd: 'snapshot' }))?.version === packBefore.version, "l'étape du pack ne s'annule pas depuis le deck");
  await p.keyboard.press('Control+Shift+z');
  await until(p, async () => pinOf(await api(p, hifi, { cmd: 'snapshot' }))?.version === forwarded.version, "l'étape du pack ne se refait pas");

  // Reopened by the agent, then refocused by open_pack on one of its layouts: the same tab, no new one.
  const reopenedTab = await openAtelier(p, hifi);
  const tabCount = (await tabsOf()).tabs.length;
  const refocused = await post(p, '/api/workspace', { cmd: 'open_pack', tab: hifi, pack: packBefore.id, widget: firstLayout });
  assert.equal(refocused.windows?.[0].active, reopenedTab, `open_pack n'active pas l'onglet du pack : ${JSON.stringify(refocused).slice(0, 300)}`);
  assert.equal(refocused.windows[0].tabs.length, tabCount, 'open_pack ouvre un onglet');
  await post(p, '/api/workspace', { cmd: 'activate', tab: hifi });
  await p.reload();
  await p.waitForSelector('.stage img');

  // L'agent choisit le thème par l'op set_theme (t-988) : Papier, puis Neutre, sans perdre les jetons du deck.
  assert.equal(back.theme, 'neutre', "l'atelier garde le thème de l'import");
  const setTheme = (theme) => api(p, hifi, { cmd: 'apply', label: 'Thème', ops: [{ op: 'set_theme', theme }], merge: null });
  await setTheme('papier');
  const onPapier = await api(p, hifi, { cmd: 'snapshot' });
  assert.equal(onPapier.theme, 'papier', "Papier n'est pas choisi");
  await setTheme('neutre');
  const repicked = await api(p, hifi, { cmd: 'snapshot' });
  assert.equal(repicked.theme, 'neutre', 'Neutre ne revient pas');
  assert.equal(repicked.theme_pins.neutre.version, importTheme.version, 'Neutre revient dans une autre version');
  // The deck's overrides (deck.tokens) are kept by Neutre: the theme row of "Packs du deck" counts the same.
  await p.reload();
  const overridesAfterRepick = await deckOverrides();
  assert.equal(overridesAfterRepick, overridesAtImport, 'les jetons du deck ont changé avec le thème');
  // The deck's theme is edited in its own theme tab, by "Ouvrir" on that row (no workshop sub-tab): Neutre, as
  // the deck pins it, right after the imported tab; "Terminer" closes it clean, back on the imported tab, pins kept.
  const themeTab = await openThemeTab(p, hifi);
  const themed = await api(p, themeTab, { cmd: 'snapshot' });
  assert.deepEqual([themed.theming?.theme.namespace, themed.theming?.theme.base.id], ['neutre', repicked.theme_pins.neutre.id], "l'onglet de thème n'édite pas Neutre");
  const themeOrder = order(await tabsOf());
  assert.equal(themeOrder.indexOf(themeTab), themeOrder.indexOf(hifi) + 1, `l'onglet de thème n'est pas juste après l'onglet importé : ${themeOrder}`);
  await p.getByRole('button', { name: 'Terminer' }).click();
  await until(p, async () => {
    const win = (await post(p, '/api/workspace', { cmd: 'view' })).windows[0];
    return win.active === hifi && !win.tabs.some((t) => t.id === themeTab);
  }, "l'onglet de thème ne se ferme pas sur l'onglet importé");
  await closePacksPanel(p);
  assert.deepEqual((await api(p, hifi, { cmd: 'snapshot' })).theme_pins, repicked.theme_pins, "l'onglet de thème change les épingles du deck");
  assert.deepEqual(repicked.themes, onPapier.themes, 'la bibliothèque des thèmes ne change pas');

  // Closing the deck by its × closes its deck-pack tab with it; the dirty deck saves silently to its file.
  // Dirtied over HTTP with no reload: the App never sees it dirty, so only the close saves it.
  await setTheme('papier');
  const savedAt = (await fs.stat(hifiTab.document.path)).mtimeMs;
  const bar = await tabsOf();
  assert.ok(bar.tabs.some((t) => t.id === reopenedTab), "l'onglet du pack rouvert n'est plus là");
  await p.locator('[role="tab"]').nth(order(bar).indexOf(hifi)).locator('.tab-close').click();
  await until(p, async () => {
    const win = await tabsOf();
    return !win.tabs.some((t) => t.id === hifi || t.id === reopenedTab);
  }, "l'onglet importé et son onglet de pack ne se ferment pas ensemble");
  assert.ok((await fs.stat(hifiTab.document.path)).mtimeMs > savedAt, "le deck fermé n'est pas enregistré dans son fichier");

  // 2. Sur mes layouts : Essentiel coché par défaut, le thème du pack, sans agent.
  const second = await openImport(p, ALTO);
  await second.locator('input[name=import-mode][value=retrofit]').check();
  const checked = second.locator('fieldset.import-layouts input[type=checkbox]:checked');
  await until(p, async () => (await checked.count()) === 1, 'Essentiel n\'est pas coché par défaut');
  // Essentiel lui-même, pas « Essentiel (copie) · à moi » enregistré par 030.
  assert.equal(await second.getByRole('checkbox', { name: 'Essentiel', exact: true }).isChecked(), true);
  assert.equal(await second.locator('.import-target select').first().inputValue(), '');
  await second.getByRole('checkbox', { name: "Peaufiner avec l'agent" }).uncheck();
  const { tab: retrofit, offset: retrofitLog } = await submitImport(p, second);
  assert.ok(![hifi, 't-1'].includes(retrofit), `nouvel onglet attendu, pas ${retrofit}`);
  const snap2 = await api(p, retrofit, { cmd: 'snapshot' });
  assert.equal(snap2.slides.length, 8);
  // Sans écriture de widget, le deck n'a pas de pack propre : il reste sur les packs de la bibliothèque.
  assert.ok(!snap2.deck_pack, "le retrofit sans widget devrait garder les packs de la bibliothèque");
  await p.waitForSelector('.stage img');
  // Sans peaufinage, aucun tour ne part dans l'onglet (le prompt du peaufinage part dès l'onglet prêt).
  await noTurnSince(p, retrofitLog, 4000);
  const retrofitTab = await savedTab(p, retrofit);
  assert.notEqual(retrofitTab.document.path, hifiTab.document.path, 'chaque import a son propre fichier');

  assert.deepEqual(errors, []);
  await b.close();
  console.log('import : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
