// The agent's update tool on the dev server (t-999): `oku-slides serve` has no updater, so a check
// is refused (the app port is UnavailableApp) and the turn still ends with its reply.
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", log in E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, mcpResults } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const TAB = 't-1';

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

  const from = logOffset(LOG);
  await post(p, '/api/agent', { cmd: 'prompt', tab: TAB, text: 'cherche les mises à jour', ui: {} });
  await until(p, async () => mcpResults(LOG, from, 'update').length === 1, 'the update call should be logged');
  const result = JSON.parse(mcpResults(LOG, from, 'update')[0]);
  assert.equal(result.isError, true, `update should be refused on serve: ${JSON.stringify(result).slice(0, 300)}`);
  await p.locator('.agent-turn', { hasText: 'cherche les mises à jour' }).getByText('Mises à jour cherchées.').waitFor();

  assert.deepEqual(errors, []);
  console.log('update-serve: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
