// The agent's undo, redo and focus-without-slide tools (t-999): a turn retitles the deck, a later
// turn's undo takes that change back (an undo inside the same turn is the session's unit test), the
// one after redoes it, then a focus with no slide deselects the user's element and keeps the shown slide.
// Last, `undo_turn` and `pack_update` refuse an unknown turn or pack, and `window` is refused by the
// dev server, which has no app windows: each new tool answers end to end through a scripted turn.
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", log in E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, mcpResults, waitForTool } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const TAB = 't-1';

const prompt = (p, text) => post(p, '/api/agent', { cmd: 'prompt', tab: TAB, text, ui: {} });
/** The deck title of tab t-1, as its snapshot gives it. */
const title = async (p) => (await post(p, '/api', { tab: TAB, req: { cmd: 'snapshot' } })).title;
/** The thread's turns holding `text`. */
const turnsOf = (p, text) => p.locator('.agent-turn', { hasText: text });

/** Plays `text` to the end of its turn; returns the decoded result of its `tool` call. */
async function turn(p, text, tool, reply) {
  const before = await turnsOf(p, text).count();
  const from = logOffset(LOG);
  await prompt(p, text);
  const result = await waitForTool(p, from, tool);
  await turnsOf(p, text).nth(before).getByText(reply).waitFor();
  return JSON.parse(result.content[0].text);
}

/** Plays `text` to the end of its turn; returns the text of its `tool` call, which must be refused. */
async function refused(p, text, tool, reply) {
  const before = await turnsOf(p, text).count();
  const from = logOffset(LOG);
  await prompt(p, text);
  await turnsOf(p, text).nth(before).getByText(reply).waitFor();
  const [out] = mcpResults(LOG, from, tool);
  assert.ok(out !== undefined, `${tool} was not called`);
  const result = JSON.parse(out);
  assert.equal(result.isError ?? result.is_error, true, `${tool} should be refused`);
  return (result.content ?? []).map((c) => c.text ?? '').join('\n');
}

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: TAB });
  await p.reload();
  await p.waitForSelector('.stage img');

  const original = await title(p);
  await turn(p, 'retitre le deck pour annuler', 'apply', 'Titre changé.');
  await until(p, async () => (await title(p)) === 'Titre à annuler', 'the agent turn should retitle the deck');

  // The next turn's undo takes back the previous turn's change.
  assert.deepEqual(await turn(p, 'annule le dernier changement', 'undo', 'Changement annulé.'), { done: 1 });
  await until(p, async () => (await title(p)) === original, 'undo should bring the title back');

  assert.deepEqual(await turn(p, 'rétablis le changement', 'redo', 'Changement rétabli.'), { done: 1 });
  await until(p, async () => (await title(p)) === 'Titre à annuler', 'redo should retitle the deck again');

  // The agent shows a slide but never selects: the user selects, then a focus with no slide
  // deselects and keeps the shown slide.
  await turn(p, 'montre la slide libre', 'focus', 'Slide montrée.');
  await p.locator('.overlay .target').first().click({ position: { x: 4, y: 4 } });
  await p.waitForSelector('.target.selected');
  const shown = () => p.evaluate(() => [...document.querySelectorAll('.thumb')].findIndex((t) => t.getAttribute('aria-current') === 'true'));
  const before = await shown();
  assert.deepEqual(await turn(p, 'désélectionne tout', 'focus', 'Sélection levée.'), { ok: true });
  await p.waitForSelector('.target.selected', { state: 'detached' });
  assert.equal(await shown(), before, 'the deselect should keep the shown slide');

  // The other new tools, each through one scripted turn.
  assert.ok((await refused(p, 'annule un tour inconnu', 'undo_turn', 'Tour non annulé.')).length > 0);
  assert.ok((await refused(p, 'mets à jour un pack inconnu', 'pack_update', 'Pack non mis à jour.')).length > 0);
  // The dev server has no app windows: `window` answers with its refusal.
  assert.ok((await refused(p, "rattache l'agent", 'window', 'Agent non rattaché.')).length > 0);

  assert.deepEqual(errors, []);
  console.log('undo-tools: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
