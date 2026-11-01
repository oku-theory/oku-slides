// Agent guides in packs and themes (t-1267): a guide tab of a pack tab's Commun saves
// a use guide when its field is left; a deck pinning that pack shows the guide, folded, in "Packs du deck".
// A deck written by another installation brings a received pack with a guide: opening it asks, "Plus tard"
// keeps the guide inactive (badge "reçu", button "Approuver ce guide" in "Packs du deck"), the button
// approves it for good (reopened, the deck no longer asks), and the agent's first turn then holds the
// `<oku-slides-guides>` block with the guide. Two screenshots go to E2E_SHOTS (default .superpowers/sdd/2026-10-31-agent-guides).
// Runs against `oku-slides serve` started by `just e2e` (fake agent, log in E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { mkdirSync } = require('node:fs');
const path = require('node:path');
const { post } = require('./lib/http.cjs');
const { closePackTabs, openPackTab, packTabShown } = require('./lib/atelier.cjs');
const { createLibraryWidgetPack, deleteLibraryPack } = require('./lib/packs.cjs');
const { writeDeckBringingGuide } = require('./lib/foreignDeck.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const { until, untilValue } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = 't-1';
const PANEL = '.agent-dock [data-surface-id="host-packs"]';
const SHOTS = process.env.E2E_SHOTS ?? path.resolve(__dirname, '..', '..', '.superpowers', 'sdd', '2026-10-31-agent-guides');
const OWN_GUIDE = 'Poser une carte par idée, jamais deux titres.';
const BROUGHT_GUIDE = 'Citer la source sous chaque chiffre.';

const api = (p, tab, req) => post(p, '/api', { tab, req });
const snapshot = (p, tab) => api(p, tab, { cmd: 'snapshot' });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
const part = (p, id) => p.locator(`${PANEL} [data-component-id="${id}"]`);
/** Text of component `id` of the packs panel, fluent isolation marks dropped; `''` when absent. */
const text = (p, id) => part(p, id).first().innerText({ timeout: 1000 }).then((t) => t.replace(/[⁨⁩]/g, '').trim(), () => '');
const blur = (p) => p.evaluate(() => document.activeElement instanceof HTMLElement && document.activeElement.blur());
const dialog = (p) => p.getByRole('alertdialog', { name: 'Ce deck apporte des guides pour les agents' });

/** Activates `tab` and reloads the page (serve pushes no view). */
async function activate(p, tab) {
  const v = await post(p, '/api/workspace', { cmd: 'activate', tab });
  assert.equal(v.error, undefined, `activation de ${tab} refusée : ${v.error}`);
  await p.reload();
}

/** Opens "Packs du deck" from the toolbar of the active tab. */
async function openPanel(p) {
  await p.getByRole('button', { name: /^Packs du deck/ }).click();
  await part(p, 'title').waitFor();
}

/** Opens `file` in a new tab; returns its id with the page reloaded on it. */
async function openFile(p, file) {
  const v = await post(p, '/api/workspace', { cmd: 'open_tab', window: 'w-1', path: file });
  assert.equal(v.error, undefined, `open_tab refusé : ${v.error}`);
  const win = (await view(p)).windows[0];
  await p.reload();
  return win.active;
}

(async () => {
  const log = process.env.E2E_AGENT_LOG;
  assert.ok(log, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  mkdirSync(SHOTS, { recursive: true });
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');

  // 1. A pack of the library: its use guide is saved when the field is left, and shown by a deck pinning it.
  const own = await createLibraryWidgetPack(p, TAB, 'Guidé');
  try {
    const packTab = await openPackTab(p, TAB, own.id);
    await p.reload();
    await packTabShown(p);
    // The guides are tabs of Commun, shown even though the new pack has no shared file.
    await p.locator('.pack-pile-common .pack-pile-item').click();
    await p.getByRole('tab', { name: 'Comment utiliser' }).click();
    const section = p.locator('#widget-guide-use');
    const field = section.locator('.guide-field').locator('textarea');
    await field.fill(OWN_GUIDE);
    await blur(p);
    await untilValue(p, async () => (await snapshot(p, packTab)).guides?.guides.use, (u) => u === OWN_GUIDE, 'le guide ne s\'enregistre pas à la sortie du champ');
    await section.locator('.guides-badge', { hasText: 'Le vôtre' }).waitFor();
    await section.locator('.guide-count', { hasText: `${[...OWN_GUIDE].length} /` }).first().waitFor();
    await p.screenshot({ path: path.join(SHOTS, 'e2e-guides-section.png') });

    // The copy is saved by closing the tab; the deck then pins the library's version, guide included.
    await closePackTabs(p);
    await activate(p, TAB);
    await openPanel(p);
    const add = part(p, 'widgets-add').locator('select');
    await until(p, async () => (await add.locator('option', { hasText: 'Guidé' }).count()) === 1, 'Guidé absent de « Ajouter depuis la bibliothèque »');
    await add.selectOption({ label: 'Guidé' });
    const fold = `widgets/${own.namespace}/guide`;
    await part(p, fold).waitFor();
    assert.match(await text(p, fold), /\(le vôtre\)/, `résumé du guide : ${await text(p, fold)}`);
    assert.ok(!(await text(p, fold)).includes(OWN_GUIDE), 'le guide est déplié d\'office');
    await part(p, fold).getByRole('button').click();
    await untilValue(p, () => text(p, fold), (t) => t.includes(OWN_GUIDE), 'le texte du guide ne se montre pas une fois déplié');
    assert.equal(await part(p, `widgets/${own.namespace}/guide-approve`).count(), 0, 'un guide à vous demande une approbation');
    await blur(p);
    await p.keyboard.press('Control+z');
    await until(p, async () => (await snapshot(p, TAB)).widgets[own.namespace] === undefined, '⌘Z ne retire pas Guidé du deck');
    await part(p, 'close').getByRole('button').click();
    await p.locator(PANEL).waitFor({ state: 'detached' });
  } finally {
    await closePackTabs(p);
    await deleteLibraryPack(p, TAB, own.id);
  }

  // 2. A deck written elsewhere brings a pack with a use guide: the install dialog asks.
  const file = await writeDeckBringingGuide(b, 'Venu', BROUGHT_GUIDE);
  let received = null;
  try {
    received = await openFile(p, file);
    assert.notEqual(received, TAB, 'le deck reçu n\'ouvre pas d\'onglet');
    const ask = dialog(p);
    await ask.waitFor();
    await p.waitForSelector('.guides-install[data-armed]');
    await ask.getByText(BROUGHT_GUIDE, { exact: true }).waitFor();
    await ask.getByRole('heading', { name: 'Venu' }).waitFor();
    await p.screenshot({ path: path.join(SHOTS, 'e2e-guides-install.png') });

    // « Plus tard » : no more dialog, the guide stays inactive.
    await ask.getByRole('button', { name: 'Plus tard' }).click();
    await ask.waitFor({ state: 'detached' });
    assert.deepEqual((await snapshot(p, received)).guides_to_approve, [], 'le deck demande encore après « Plus tard »');
    await openPanel(p);
    const card = part(p, 'widgets/venu/guide');
    await card.waitFor();
    assert.match((await card.innerText()).replace(/[⁨⁩]/g, ''), /\(reçu\)/, 'le guide reçu n\'est pas marqué reçu');
    const approve = part(p, 'widgets/venu/guide-approve');
    await approve.getByRole('button', { name: 'Approuver ce guide' }).waitFor();

    // « Approuver ce guide » approves it for good: the button goes.
    await approve.getByRole('button').click();
    await approve.waitFor({ state: 'detached' });

    // 3. The agent's first turn in this tab carries the guides block with the guide.
    const from = logOffset(log);
    const sent = await post(p, '/api/agent', { cmd: 'prompt', tab: received, text: 'portée slides', ui: {} });
    assert.equal(sent.error, undefined, `consigne refusée : ${sent.error}`);
    await until(p, () => logSince(log, from).some((m) => m.method === 'session/prompt' && JSON.stringify(m).includes('<oku-slides-guides>')), 'le tour de l\'agent ne porte pas <oku-slides-guides>', 15_000);
    assert.ok(logSince(log, from).some((m) => m.method === 'session/prompt' && JSON.stringify(m).includes(BROUGHT_GUIDE)), 'le guide approuvé n\'arrive pas à l\'agent');

    // 4. Reopened, the deck asks no more.
    await post(p, '/api/workspace', { cmd: 'close', tab: received, discard: true });
    received = null;
    received = await openFile(p, file);
    await p.waitForSelector('.stage img');
    assert.equal(await dialog(p).count(), 0, 'le deck rouvert redemande l\'approbation');
    assert.deepEqual((await snapshot(p, received)).guides_to_approve, [], 'le deck rouvert liste encore des guides à approuver');
  } finally {
    if (received !== null) await post(p, '/api/workspace', { cmd: 'close', tab: received, discard: true });
    const left = (await api(p, TAB, { cmd: 'packs', kind: 'widgets' })).find((x) => x.name === 'Venu');
    if (left) await deleteLibraryPack(p, TAB, left.id);
  }
  await activate(p, TAB);
  assert.deepEqual(errors, []);
  console.log('guides ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
