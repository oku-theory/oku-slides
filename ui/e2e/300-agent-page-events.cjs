// Agent page events: the fake agent calls `present` (the deck shows in the window presenter, left
// with Escape) and `set_interface` (the scheme and the stage guides change in the page, t-1005).
// Runs against `oku-slides serve` started by `just e2e` (fake agent "Faux", log in E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, waitForTool } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;

const prompt = (p, text) => post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text, ui: {} });

/** Plays `text`, whose turn calls `tool`; waits for the tool's success and the turn's `reply`. */
async function play(p, text, tool, reply) {
  const turns = p.locator('.agent-turn', { hasText: text });
  const before = await turns.count();
  const from = logOffset(LOG);
  await prompt(p, text);
  await waitForTool(p, from, tool);
  await turns.nth(before).getByText(reply).waitFor();
}

/** The scheme the page shows (`appearance.ts` sets it on the root element). */
const scheme = (p) => p.evaluate(() => document.documentElement.dataset.scheme);
const guides = (p) => p.locator('.stage .stage-ghost-layer').count();

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 }, colorScheme: 'light' });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');

  // 1. `present` in window mode: the presenter covers the page with the slide; Escape leaves it.
  const presenter = p.locator('.presenter[role="dialog"]');
  assert.equal(await presenter.count(), 0);
  await play(p, 'présente en fenêtre', 'present', 'Présentation lancée.');
  await presenter.waitFor();
  await presenter.locator('img').waitFor();
  await p.keyboard.press('Escape');
  await presenter.waitFor({ state: 'detached' });
  await p.waitForSelector('.stage img');

  // 2. `set_interface` scheme: dark, then back to the system's (light here).
  assert.equal(await scheme(p), 'light');
  await play(p, 'passe en sombre', 'set_interface', 'Schéma sombre.');
  await until(p, async () => (await scheme(p)) === 'dark', 'the page must switch to the dark scheme');
  await play(p, 'remets le schéma du système', 'set_interface', 'Schéma du système.');
  await until(p, async () => (await scheme(p)) === 'light', "the page must return to the system's scheme");

  // 3. `set_interface` guides: the stage's ghost layer shows, then hides.
  assert.equal(await guides(p), 0);
  await play(p, 'affiche les repères', 'set_interface', 'Repères affichés.');
  await until(p, async () => (await guides(p)) === 1, 'the stage guides must show');
  await play(p, 'masque les repères', 'set_interface', 'Repères masqués.');
  await until(p, async () => (await guides(p)) === 0, 'the stage guides must hide');

  assert.deepEqual(errors, []);
  console.log('agent page events: ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
