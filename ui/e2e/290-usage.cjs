// Agent usage: a counted turn (the fake agent's tokens and cost) enters the deck's ledger, and the
// thread's `show_usage` box shows it. Other specs already played turns on t-1, so only the deltas
// of the totals row between two boxes are checked.
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", log in E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, logSince, waitForTool } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
/** French column headers (session.ftl `usage-col-turns`, `-input`, `-cost`, Task 7). */
const HEAD = { turns: 'Tours', input: 'Entrée', cost: 'Coût' };

const prompt = (p, text) => post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text, ui: {} });
/** A cell's number: digits and dot kept, 0 for "–". */
const num = (text) => Number(text.replace(/[^\d.]/g, '') || '0');
/** The thread's turns holding `text`. */
const turnsOf = (p, text) => p.locator('.agent-turn', { hasText: text });

/** Plays a prompt to the end of its turn: its reply shows in the thread, in a new turn. */
async function turn(p, text, reply) {
  const before = await turnsOf(p, text).count();
  const from = logOffset(LOG);
  await prompt(p, text);
  await until(p, async () => logSince(LOG, from).some((m) => m.method === 'session/prompt'), `"${text}" must reach the fake agent`, 15000);
  await turnsOf(p, text).nth(before).getByText(reply).waitFor();
}

/** Asks for the usage box; returns its totals row as `{ column header: cell }` and whether it fits. */
async function usageBox(p) {
  const asked = 'combien a coûté ce deck';
  const before = await turnsOf(p, asked).count();
  const from = logOffset(LOG);
  await prompt(p, asked);
  const result = await waitForTool(p, from, 'show_usage');
  const { shown } = JSON.parse(result.content[0].text);
  const box = p.locator(`[data-surface-id="${shown}"]`);
  await box.locator('table').waitFor();
  await turnsOf(p, asked).nth(before).getByText("Voici l'usage.").waitFor();
  const head = await box.locator('table th').allInnerTexts();
  const cells = await box.locator('table tr').last().locator('td').allInnerTexts();
  assert.equal(cells.length, head.length, `totals row: ${cells.join(' | ')}`);
  const totals = Object.fromEntries(head.map((h, i) => [h.trim(), cells[i]]));
  for (const h of Object.values(HEAD)) assert.ok(h in totals, `missing column ${h}: ${head.join(' | ')}`);
  const fits = await box.evaluate((el) => {
    const list = el.closest('.agent-turns');
    return list === null || list.scrollWidth <= list.clientWidth;
  });
  return { totals, fits };
}

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

  // 1. A first cost-bearing turn: the session's cost base is now 0.1, resumed or not.
  await turn(p, 'pose la base de coût', 'Base posée.');

  // 2. First box: the starting totals.
  const a = await usageBox(p);

  // 3. The counted turn (cumulative cost 0.4321), then the second box.
  await turn(p, 'compte les tokens', 'Compté.');
  const bx = await usageBox(p);

  // 4. Deltas: box A's own turn (no tokens) and the counted one; 4321 input, 0.4321 - 0.1 USD.
  const delta = (h) => num(bx.totals[h]) - num(a.totals[h]);
  const shown = `${JSON.stringify(a.totals)} -> ${JSON.stringify(bx.totals)}`;
  assert.equal(delta(HEAD.turns), 2, `turns: ${shown}`);
  assert.equal(delta(HEAD.input), 4321, `input: ${shown}`);
  assert.ok(bx.totals[HEAD.cost].includes('USD'), `cost: ${shown}`);
  assert.ok(Math.abs(delta(HEAD.cost) - 0.3321) < 1e-9, `cost: ${shown}`);

  // 5. The nine-column table scrolls inside its box: the thread does not widen.
  assert.ok(bx.fits, 'the usage box widens the thread');

  assert.deepEqual(errors, []);
  console.log('usage : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
