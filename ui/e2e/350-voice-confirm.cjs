// Spoken answer to a danger confirmation (t-997): the fake agent calls `merge`; once the modal is
// armed, "oui" alone keeps it open and shows what was heard; "oui, fusionner" is the right answer,
// but `serve` only has the focus its pages report, which never confirms a danger (Q6): the modal
// says to click, and the click on the verb button merges.
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", E2E_AGENT_LOG,
// E2E_UTTERANCES, E2E_DECKS). The merged file is a copy of the deck, saved by this spec first in
// the run's decks dir, where the fixture's `${OKU_SLIDES_PATHS__DECKS}` path points. A copy shares
// t-1's history, so the merge brings nothing: t-1 stays as later specs expect.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { mkdirSync, rmSync } = require('node:fs');
const path = require('node:path');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { handsFree, status } = require('./lib/voiceBox.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const { LISTENING, prompted, say } = require('./lib/say.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const DECKS = process.env.E2E_DECKS;
const COPY_DIR = DECKS && path.join(DECKS, 'e2e-350');

const MODAL = '.agent-modal[role="alertdialog"]';
const HEARD = `${MODAL} [data-component-id="heard"]`;
const nothing = () => true;

/** The slide ids of t-1 and whether it is modified. */
async function deck(p) {
  const s = await post(p, '/api', { tab: 't-1', req: { cmd: 'snapshot' } });
  return { slides: s.slides.map((x) => x.id), dirty: s.dirty };
}

/** The `merge` results the fake agent logged since `from`. */
const merges = (from) => logSince(LOG, from).filter((m) => m.method === 'mcp/result' && m.params?.tool === 'merge');

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
  const before = await deck(p);
  let failed = null;
  try {
    const copied = await post(p, '/api', { tab: 't-1', req: { cmd: 'save_copy', path: `${COPY_DIR}/copie.oku-slides.pdf` } });
    assert.ok(!copied.error, `save_copy: ${JSON.stringify(copied).slice(0, 200)}`);
    await handsFree(p, true);
    await p.waitForSelector(LISTENING);

    // 1. The agent asks to merge: a danger modal whose button shows the verb.
    const from = logOffset(LOG);
    await say(p, 'fusionne la copie', { heard: prompted('fusionne la copie') });
    await p.waitForSelector(`${MODAL}[data-armed]`);
    assert.equal((await p.locator(`${MODAL} button[data-primary]`).textContent()).trim(), 'Fusionner');

    // 2. "oui" without the verb: still open, and the heard line says what was heard.
    await say(p, 'oui', { heard: nothing });
    await p.waitForSelector(`${HEARD}:has-text("Entendu : « oui »")`);
    assert.equal(merges(from).length, 0, 'a bare yes must not confirm a danger');

    // 3. "oui, fusionner": the right words, but the dev server's focus is page-reported (Q6).
    await say(p, 'oui, fusionner', { heard: nothing });
    await p.waitForSelector(`${HEARD}:has-text("Cliquez sur le bouton pour confirmer.")`);
    assert.equal(merges(from).length, 0, 'page-reported focus must not confirm a danger');

    // 4. The click on the verb button merges the copy.
    await p.locator(`${MODAL} button[data-primary]`).click();
    await p.waitForSelector(MODAL, { state: 'detached' });
    await until(p, async () => merges(from).length === 1, 'merge should run once confirmed');
    const out = String(merges(from)[0].params.out);
    const text = JSON.parse(out).content?.[0]?.text ?? '';
    assert.deepEqual(JSON.parse(text), { ok: true }, `merge result: ${out.slice(0, 200)}`);
    assert.deepEqual(await deck(p), before, 'merging a copy of t-1 should leave t-1 unchanged');
    await until(p, async () => logSince(LOG, from).some((m) => m.method === 'turn/end'), 'the turn should end');
  } catch (e) {
    failed = e;
  }
  // The server is shared: hands-free off again, listening stopped, the copy removed.
  try {
    if ((await status(p)).config.hands_free) await handsFree(p, false);
    await until(p, async () => (await status(p)).state === 'idle', 'listening should stop with the switch');
  } catch (e) {
    if (failed) console.error('restore:', e);
    else failed = e;
  }
  rmSync(COPY_DIR, { recursive: true, force: true });
  if (failed) throw failed;
  assert.deepEqual(errors, []);
  console.log('voice confirm: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
