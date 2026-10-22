// Import PowerPoint dans l'app : « Ouvrir » un .pptx (alto) ouvre le dialogue d'import ; en copie
// fidèle avec le peaufinage, l'onglet importé s'ouvre et le faux agent joue son tour de
// fine-tuning (import_report, import_diff de la slide 7, report) ; l'atelier édite son pack de layouts
// du deck seul, et son thème du deck seul se reprend après Papier ; puis un import sur mes layouts
// (Essentiel par défaut, sans agent).
// Contre `oku-slides serve` lancé par `just e2e` : faux agent « Faux » choisi, son journal dans
// E2E_AGENT_LOG (les résultats MCP s'y lisent). Le témoin (office2pdf) tourne pour de vrai : attentes longues.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { openAtelier } = require('./lib/atelier.cjs');
const agentLog = require('./lib/agentLog.cjs');
const path = require('node:path');
const { post } = require('./lib/http.cjs');
const { until, untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const ALTO = path.resolve(__dirname, '..', '..', 'fixtures', 'import', 'alto-wealth.pptx');
/** Import et premier rendu d'alto, puis le tour de l'agent avec le témoin : bien plus que les 8 s par défaut. */
const SLOW = 180_000;
const PAPIER = '2a7d4c91-5e3b-4f08-b6a1-8c9e0d1f2a31';

const api = (p, tab, req) => post(p, '/api', { tab, req });

const logOffset = () => agentLog.logOffset(LOG);
const logSince = (from) => agentLog.logSince(LOG, from);

/** Les résultats MCP (`mcp/result`) notés pour `tool` depuis `from`, dans l'ordre. */
const mcpResults = (tool, from) =>
  logSince(from)
    .filter((m) => m.method === 'mcp/result' && m.params.tool === tool)
    .map((m) => m.params.out);

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
    (t) => !!t?.path && !t.dirty,
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
  const importTheme = snap.theme;
  assert.deepEqual(snap.deck_themes.map((th) => [th.id, th.version, th.deck_only]), [[importTheme.id, importTheme.version, true]]);
  await p.waitForSelector('.stage img');
  // Le deck importé est neuf : l'enregistrement automatique lui donne un fichier, au nom du .pptx.
  const hifiTab = await savedTab(p, hifi);
  assert.equal(hifiTab.title, 'alto-wealth');
  assert.match(hifiTab.path, /alto-wealth[^/]*\.oku-slides\.pdf$/);

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
  // L'atelier demandé à l'agent (t-988) ouvre le pack de layouts propre du deck.
  await openAtelier(p, hifi);
  await until(p, async () => (await api(p, hifi, { cmd: 'snapshot' })).editing?.deck_only === true, "l'atelier ne s'ouvre pas sur le pack du deck seul");
  await p.locator('.brand-note', { hasText: 'Enregistré dans cette présentation seulement' }).waitFor();
  assert.match(await p.locator('.label .brand').textContent(), / · pack de layouts$/);
  await p.locator('.doc-name-button').click();
  await p.getByRole('textbox', { name: 'Titre de la présentation' }).fill('Alto (layouts)');
  await p.keyboard.press('Enter');
  await until(p, async () => (await api(p, hifi, { cmd: 'snapshot' })).title === 'Alto (layouts)', "le pack n'est pas renommé");
  // ✓ : l'enregistrement automatique part d'abord, puis le deck revient.
  await p.getByRole('button', { name: 'Terminer' }).click();
  await until(p, async () => (await api(p, hifi, { cmd: 'snapshot' })).editing === null, "l'atelier ne se ferme pas");
  const back = await api(p, hifi, { cmd: 'snapshot' });
  assert.ok(back.deck_pack, 'le pack enregistré reste propre au deck');
  assert.equal(pinOf(back).id, packBefore.id);
  assert.notEqual(pinOf(back).version, packBefore.version, 'le deck passe à la version enregistrée');
  assert.deepEqual(await library(), packsBefore, 'la bibliothèque ne change pas');

  // Le thème du deck seul se reprend après Papier (t-903) ; l'agent choisit le thème par l'op set_theme (t-988).
  assert.deepEqual(back.theme, importTheme, "l'atelier garde le thème de l'import");
  const setTheme = (theme) => api(p, hifi, { cmd: 'apply', label: 'Thème', ops: [{ op: 'set_theme', theme }], merge: null });
  await setTheme({ id: PAPIER, version: back.themes.find((th) => th.id === PAPIER).version });
  const onPapier = await api(p, hifi, { cmd: 'snapshot' });
  assert.equal(onPapier.theme?.id, PAPIER, "Papier n'est pas choisi");
  assert.deepEqual(onPapier.deck_themes.map((th) => th.id), [importTheme.id], "le thème de l'import n'est plus offert");
  await setTheme(importTheme);
  const repicked = await api(p, hifi, { cmd: 'snapshot' });
  assert.equal(repicked.theme?.id, importTheme.id, "le thème de l'import ne revient pas");
  assert.equal(repicked.theme.version, importTheme.version, "le thème de l'import revient dans sa version d'origine");
  assert.deepEqual(repicked.themes, onPapier.themes, 'la bibliothèque des thèmes ne change pas');

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
  assert.notEqual(retrofitTab.path, hifiTab.path, 'chaque import a son propre fichier');

  assert.deepEqual(errors, []);
  await b.close();
  console.log('import : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
