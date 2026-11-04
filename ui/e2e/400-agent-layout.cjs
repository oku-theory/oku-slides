// The agent changes the editor layout through `set_interface` (t-1001): it folds the slide
// thumbnails and opens the code tab; the page saves both through its `ui` request, and they survive
// a reload. The fake agent's turns are in fixtures/agent/e2e.json.
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", log in E2E_AGENT_LOG).
// Restores what it changed: the thumbnails open again and the Settings tab, through the agent too.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, waitForTool } = require('./lib/agentLog.cjs');
const { uiSaved } = require('./lib/uiSaved.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;

const prompt = (p, text) => post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text, ui: {} });
const keptUi = async (p) => (await post(p, '/api', { tab: 't-1', req: { cmd: 'snapshot' } })).ui;

/** Plays `text`, whose turn calls `set_interface`; waits for its success and the turn's `reply`. */
async function play(p, text, reply) {
  const turns = p.locator('.agent-turn', { hasText: text });
  const before = await turns.count();
  const from = logOffset(LOG);
  await prompt(p, text);
  await waitForTool(p, from, 'set_interface');
  await turns.nth(before).getByText(reply).waitFor();
}

/** The toolbar's thumbnails toggle, pressed while the strip shows. */
const thumbsToggle = (p, open) => p.locator(`.thumbs-toggle[aria-pressed="${open}"]`);

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');
  const before = await keptUi(p);
  assert.ok(!before.folded.includes('slides') && before.code === false, `unexpected starting layout: ${JSON.stringify(before)}`);
  await thumbsToggle(p, true).waitFor();
  await p.locator('.slide-strip').waitFor();

  // 1. The agent folds the slides and opens the code tab: the page applies and saves both.
  const folded = uiSaved(p, (req) => req.folded.includes('slides') && req.code === true);
  await play(p, 'replie les diapositives et ouvre le code', 'Diapositives repliées.');
  assert.ok((await folded).ok(), 'the folded layout should be saved by the host');
  await thumbsToggle(p, false).waitFor();
  await p.locator('.slide-strip').waitFor({ state: 'detached' });

  // 2. A reload restores the kept layout: still folded, the code tab still chosen.
  // `code` has no visible effect on a deck tab: checked as kept state; 170-code-layout.cjs covers the Code tab.
  await p.reload();
  await p.waitForSelector('.stage img');
  await thumbsToggle(p, false).waitFor();
  await until(p, async () => (await p.locator('.slide-strip').count()) === 0, 'the strip should stay folded');
  const kept = await keptUi(p);
  assert.ok(kept.folded.includes('slides') && kept.code === true, `the layout should survive a reload: ${JSON.stringify(kept)}`);

  // 3. Restore: the agent opens the slides again and goes back to the Settings tab.
  const opened = uiSaved(p, (req) => !req.folded.includes('slides') && req.code === false);
  await play(p, 'déplie les diapositives et ferme le code', 'Diapositives dépliées.');
  assert.ok((await opened).ok(), 'the restored layout should be saved by the host');
  await p.locator('.slide-strip').waitFor();
  const after = await keptUi(p);
  assert.deepEqual({ folded: after.folded, code: after.code }, { folded: before.folded, code: before.code });

  assert.deepEqual(errors, []);
  console.log('agent layout: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
