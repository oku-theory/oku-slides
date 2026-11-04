// The agent's app tools (t-999): `import` opens a .pptx in a new tab of the agent's window. The fake
// agent calls `import` on a copy of `fixtures/import/libreoffice.pptx`, saved by this spec in the run's
// decks dir, where the fixture's `${OKU_SLIDES_PATHS__DECKS}` path points. The import is a danger
// confirmation with the verb "Importer": said hands-free under `serve` (page-reported focus, Q6),
// the modal says to click; the click imports, and the new tab shows beside t-1. Then the agent's tab
// tools (t-1000): `tabs` lists both tabs; `tab close` on another, dirty blank tab asks "Abandonner"
// and, clicked, closes it right away; `tab activate` shows the imported one, whose own close asks
// with the server's verb; said, it does not confirm under `serve`; clicked, it waits for the turn's
// end (hands-free hears the imported tab, shown since the import), then the page redeems the
// decision and closes it. Last, `help` (t-1000): "ouvre l'aide sur la dictée", said from t-1, opens
// the help tab at the first hit, whose title the help strip's current thumbnail shows. Then, said
// in the help tab: `quit` answers Unavailable under `serve` (R15) and nothing closes;
// `new_conversation` starts the thread over once the turn ends: the next prompt opens a new session.
// The fixture's `by_prefix` turns put the tab id said after the prefix in the tool's args.
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", E2E_AGENT_LOG,
// E2E_UTTERANCES, E2E_DECKS). Restores what it changed: hands-free off, the new tabs closed, the copy
// and any autosave of the imported deck removed.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { copyFileSync, mkdirSync, rmSync } = require('node:fs');
const path = require('node:path');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { handsFree, status } = require('./lib/voiceBox.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const { LISTENING, prompted, say } = require('./lib/say.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const DECKS = process.env.E2E_DECKS;
const COPY_DIR = DECKS && path.join(DECKS, 'e2e-390');
const DRAFT = 'e2e-390-brouillon';
const SOURCE = path.resolve(__dirname, '..', '..', 'fixtures', 'import', 'libreoffice.pptx');

const MODAL = '.agent-modal[role="alertdialog"]';
const HEARD = `${MODAL} [data-component-id="heard"]`;
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
const nothing = () => true;

/** The ids of the first window's tabs, in bar order. */
const tabIds = async (p) => (await view(p)).windows[0].tabs.map((t) => t.id);

/** The results of `tool` the fake agent logged since `from`. */
const results = (from, tool) => logSince(LOG, from).filter((m) => m.method === 'mcp/result' && m.params?.tool === tool);

/** The `import` results the fake agent logged since `from`. */
const imports = (from) => results(from, 'import');

/** The JSON a logged tool result carries, or null. */
const resultOf = (m) => JSON.parse(JSON.parse(String(m.params.out)).content?.[0]?.text ?? 'null');

/**
 * A wait for tab `tab`'s accepted context report with the focus, registered before the reload:
 * hands-free hears the tab the page last reported focused.
 */
const focusReported = (p, tab) =>
  p.waitForResponse((r) => {
    if (!r.ok() || !r.url().endsWith('/api/agent')) return false;
    const req = JSON.parse(r.request().postData() ?? 'null');
    return req?.cmd === 'context' && req.tab === tab && req.has_focus === true;
  });

/** Waits for the turn that started at `from` to end. */
const turnEnded = (p, from) => until(p, async () => logSince(LOG, from).some((m) => m.method === 'turn/end'), 'the turn should end');

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
  assert.ok(DECKS, 'E2E_DECKS missing: run through just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');
  mkdirSync(COPY_DIR, { recursive: true });
  copyFileSync(SOURCE, path.join(COPY_DIR, 'e2e-390.pptx'));
  const before = await tabIds(p);
  let opened = null;
  let draft = null;
  let help = null;
  let failed = null;
  try {
    await handsFree(p, true);
    await p.waitForSelector(LISTENING);

    // 1. The agent asks to import: a danger modal whose button shows the verb, nothing opened yet.
    const from = logOffset(LOG);
    await say(p, 'importe la présentation', { heard: prompted('importe la présentation') });
    await p.waitForSelector(`${MODAL}[data-armed]`);
    assert.equal((await p.locator(`${MODAL} button[data-primary]`).textContent()).trim(), 'Importer');
    assert.match(await p.locator(MODAL).innerText(), /e2e-390[/\\]e2e-390\.pptx/, 'the modal should name the full path');
    assert.deepEqual(await tabIds(p), before, 'no tab before the confirmation');

    // 2. "oui, importer": the right words, but the dev server's focus is page-reported (Q6).
    await say(p, 'oui, importer', { heard: nothing });
    await p.waitForSelector(`${HEARD}:has-text("Cliquez sur le bouton pour confirmer.")`);
    assert.equal(imports(from).length, 0, 'page-reported focus must not confirm a danger');

    // 3. The click on the verb button imports: a new tab beside t-1.
    await p.locator(`${MODAL} button[data-primary]`).click();
    await p.waitForSelector(MODAL, { state: 'detached' });
    await until(p, async () => imports(from).length === 1, 'import should run once confirmed', 60_000);
    const out = String(imports(from)[0].params.out);
    const result = JSON.parse(JSON.parse(out).content?.[0]?.text ?? 'null');
    assert.equal(result?.ok, true, `import result: ${out.slice(0, 300)}`);
    opened = result.tab;
    const tabs = await tabIds(p);
    assert.deepEqual(tabs, [...before, opened], `the imported tab should follow the others: ${tabs}`);
    const tab = (await view(p)).windows[0].tabs.find((t) => t.id === opened);
    assert.equal(tab.document.kind, 'deck', 'the imported tab holds a deck');
    // serve pushes no view: the page shows the new tab once it reads the workspace again.
    await p.reload();
    const bar = p.locator('.tabs[role="tablist"] [role="tab"]');
    await bar.locator('.tab-title', { hasText: tab.title }).first().waitFor();
    assert.equal(await bar.count(), tabs.length, 'the tab bar should show the imported tab');
    await turnEnded(p, from);

    // 4. "liste les onglets": the agent reads both tabs of its window, the imported one by its title.
    const listed = logOffset(LOG);
    await say(p, 'liste les onglets');
    await until(p, async () => results(listed, 'tabs').length === 1, 'tabs should run');
    const listing = resultOf(results(listed, 'tabs')[0]);
    assert.deepEqual(listing.tabs.map((t) => t.tab), tabs, `tabs should list this window's tabs: ${JSON.stringify(listing)}`);
    assert.equal(listing.tabs.find((t) => t.tab === opened)?.title, tab.title);
    await turnEnded(p, listed);

    // 5. Closing another tab right away: a blank tab, made dirty while hidden (only the shown page
    // autosaves), asks with "Abandonner"; said, it does not confirm; clicked, the page closes it now.
    // The imported tab, shown since the import, stays the one hands-free hears: it asks.
    const created = await post(p, '/api/workspace', { cmd: 'new_tab', window: 'w-1' });
    draft = created.windows.find((w) => w.label === 'w-1').active;
    const ops = [{ op: 'set_deck_param', key: 'title', value: DRAFT }];
    const edited = await post(p, '/api', { tab: draft, req: { cmd: 'apply', label: 'Titre', ops, merge: null } });
    assert.equal(edited.error, undefined, `apply refused: ${edited.error}`);
    await post(p, '/api/workspace', { cmd: 'activate', tab: opened });
    await p.reload();
    await p.waitForSelector(LISTENING);
    const other = logOffset(LOG);
    await say(p, `ferme l'onglet ${draft}`);
    await p.waitForSelector(`${MODAL}[data-armed]`);
    assert.equal((await p.locator(`${MODAL} button[data-primary]`).textContent()).trim(), 'Abandonner');
    await say(p, 'oui, abandonner', { heard: nothing });
    await p.waitForSelector(`${HEARD}:has-text("Cliquez sur le bouton pour confirmer.")`);
    assert.ok((await tabIds(p)).includes(draft), 'a said confirmation must not close the tab');
    await p.locator(`${MODAL} button[data-primary]`).click();
    await p.waitForSelector(MODAL, { state: 'detached' });
    await until(p, async () => !(await tabIds(p)).includes(draft), 'the other tab should close', 30_000);
    assert.equal(resultOf(results(other, 'tab')[0])?.closing, 'now', 'another tab closes right away');
    assert.deepEqual(await tabIds(p), tabs, 'only the blank tab should close');
    assert.equal((await view(p)).windows[0].active, opened, 'the asking tab should stay shown');
    draft = null;
    await turnEnded(p, other);

    // 6. "active l'onglet <id>": the tool shows the imported tab, the one already shown (activating
    // another would leave hands-free on a hidden tab, whose tools refuse page events).
    const shown = logOffset(LOG);
    await say(p, `active l'onglet ${opened}`);
    await until(p, async () => (await view(p)).windows[0].active === opened, 'the imported tab should be active');
    await turnEnded(p, shown);

    // 7. The page shows the imported tab, which hands-free now hears: "ferme l'onglet <id>" is its
    // own close, a danger confirmation; said, it does not confirm. The verb is the server's own
    // (Fermer or Abandonner, as the Rust tests check); this step reads it rather than guess it.
    await p.reload();
    await p.waitForSelector(LISTENING);
    const closing = logOffset(LOG);
    await say(p, `ferme l'onglet ${opened}`);
    await p.waitForSelector(`${MODAL}[data-armed]`);
    const verb = (await p.locator(`${MODAL} button[data-primary]`).textContent()).trim();
    assert.ok(['Fermer', 'Abandonner'].includes(verb), `a close verb: ${verb}`);
    assert.ok((await p.locator(MODAL).innerText()).includes(tab.title), 'the modal should name the tab');
    await say(p, `oui, ${verb.toLowerCase()}`, { heard: nothing });
    await p.waitForSelector(`${HEARD}:has-text("Cliquez sur le bouton pour confirmer.")`);
    assert.deepEqual(await tabIds(p), tabs, 'a said confirmation must not close the tab');

    // 8. The click: once the turn ended, the page redeems the decision and runs closeTabFlow.
    await p.locator(`${MODAL} button[data-primary]`).click();
    await p.waitForSelector(MODAL, { state: 'detached' });
    await until(p, async () => JSON.stringify(await tabIds(p)) === JSON.stringify(before), 'the tab should close', 30_000);
    assert.equal(resultOf(results(closing, 'tab')[0])?.closing, 'after_turn', 'its own tab closes after the turn');
    await turnEnded(p, closing);

    // 9. "ouvre l'aide sur la dictée", said from t-1: the help tab opens at the query's first hit,
    // the dictation slide; the page shows its title. Hands-free hears the tab the page last
    // reported focused, the closed imported one until t-1's report lands: wait for it first.
    await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
    const reported = focusReported(p, 't-1');
    await p.reload();
    await reported;
    await p.waitForSelector(LISTENING);
    const asked = logOffset(LOG);
    await say(p, "ouvre l'aide sur la dictée");
    await until(p, async () => results(asked, 'help').length === 1, 'help should run');
    const found = resultOf(results(asked, 'help')[0]);
    assert.equal(found?.ok, true, `help result: ${JSON.stringify(found)}`);
    help = found.tab;
    const hit = found.hits?.[0]?.title ?? '';
    assert.match(hit, /Dictée/, `the first hit should be about dictation: ${JSON.stringify(found.hits)}`);
    assert.equal((await view(p)).windows[0].active, help, 'the help tab should be shown');
    await turnEnded(p, asked);
    const helpFocused = focusReported(p, help);
    await p.reload();
    await helpFocused;
    const current = p.locator('.slide-strip .thumb[aria-current="true"] img.thumb-img');
    await current.waitFor({ timeout: 120_000 });
    assert.equal(await current.getAttribute('alt'), hit, 'the help tab should show the first hit');

    // 10. "quitte l'application", said from t-1: the help tab has no dictation bar, so the user
    // clicks t-1's tab and hands-free hears it once its focused report lands. The dev server
    // cannot quit (R15): refused, nothing asked or closed.
    const deck = (await view(p)).windows[0].tabs.find((t) => t.id === 't-1');
    const deckFocused = focusReported(p, 't-1');
    await p.locator('.tabs[role="tablist"] [role="tab"]').filter({ has: p.locator('.tab-title', { hasText: deck.title }) }).first().click();
    await deckFocused;
    await p.waitForSelector(LISTENING);
    const quitting = logOffset(LOG);
    const kept = await tabIds(p);
    await say(p, "quitte l'application");
    await until(p, async () => results(quitting, 'quit').length === 1, 'quit should run');
    const quit = JSON.parse(String(results(quitting, 'quit')[0].params.out));
    assert.equal(quit.isError, true, `quit should be refused: ${JSON.stringify(quit)}`);
    assert.match(quit.content?.[0]?.text ?? '', /Seule l'app Oku Slides peut le faire/);
    await turnEnded(p, quitting);
    assert.equal(await p.locator(MODAL).count(), 0, 'no confirmation for a refused quit');
    assert.deepEqual(await tabIds(p), kept, 'a refused quit closes nothing');

    // 11. "repars de zéro": the thread starts over once the turn ends; the next prompt opens a new session.
    const restarting = logOffset(LOG);
    await say(p, 'repars de zéro');
    await until(p, async () => results(restarting, 'new_conversation').length === 1, 'new_conversation should run');
    assert.equal(resultOf(results(restarting, 'new_conversation')[0])?.new_conversation, 'after_turn');
    await turnEnded(p, restarting);
    const next = logOffset(LOG);
    await say(p, 'après le rechargement');
    await turnEnded(p, next);
    assert.ok(logSince(LOG, next).some((m) => m.method === 'session/new'), 'the next prompt should open a new session');
  } catch (e) {
    failed = e;
  }
  // The server is shared: hands-free off again, listening stopped, the new tab closed, files removed.
  try {
    // The switch is in the dictation bar, which a help tab lacks: back to t-1 first.
    await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
    await p.reload();
    await p.waitForSelector('.stage img');
    if ((await status(p)).config.hands_free) await handsFree(p, false);
    await until(p, async () => (await status(p)).state === 'idle', 'listening should stop with the switch');
    if (opened !== null && (await tabIds(p)).includes(opened)) {
      const closed = await post(p, '/api/workspace', { cmd: 'close', tab: opened, discard: true });
      assert.equal(closed.error, undefined, `closing ${opened}: ${closed.error}`);
    }
    if (help !== null && (await tabIds(p)).includes(help)) {
      await post(p, '/api/workspace', { cmd: 'close', tab: help, discard: true });
    }
    if (draft !== null && (await tabIds(p)).includes(draft)) {
      await post(p, '/api/workspace', { cmd: 'close', tab: draft, discard: true });
    }
    assert.deepEqual(await tabIds(p), before, 'the tabs should be as before');
  } catch (e) {
    if (failed) console.error('restore:', e);
    else failed = e;
  }
  rmSync(COPY_DIR, { recursive: true, force: true });
  rmSync(path.join(DECKS, 'e2e-390.oku-slides.pdf'), { force: true });
  rmSync(path.join(DECKS, `${DRAFT}.oku-slides.pdf`), { force: true });
  if (failed) throw failed;
  assert.deepEqual(errors, []);
  console.log('agent app tools: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
