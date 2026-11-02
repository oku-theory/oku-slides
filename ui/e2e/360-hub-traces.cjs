// An agent edit opens nothing and selects nothing; its trace in the receipt opens the data pane on the
// edited chart. The slide `s-ventes` and the chart `e-ventes` are the ones fixtures/agent/e2e.json edits for
// "modifie le graphique des ventes"; both are removed at the end, the suite keeps its layout and slides.
// Against `oku-slides serve fixtures/roadmap.json` with the fake agent. Run: just e2e
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { prompt } = require('./lib/agent.cjs');
const { logOffset, waitForTool } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;
const SLIDE = 's-ventes';
const CHART = 'e-ventes';
const TEXT = 'modifie le graphique des ventes';
const OPEN = '.inspector, .data-pane, .agent-popover';

const api = (p, req) => post(p, '/api', { tab: 't-1', req });
const apply = async (p, label, ops) => {
  const done = await api(p, { cmd: 'apply', label, merge: null, ops });
  assert.ok(!done.error, JSON.stringify(done));
};
/** The slide `s-ventes` with its chart, as 150-data-panel adds its own. */
const addChart = async (p) => {
  await apply(p, 'slide', [{ op: 'add_slide', defaults: true, slide: { id: SLIDE, kind: 'essentiel/content', params: { title: 'Ventes' } } }]);
  await apply(p, 'graphique', [{
    op: 'add_element', slot: { owner: { slide: SLIDE }, param: 'blocks' }, defaults: true,
    element: { id: CHART, kind: 'base/chart', params: { option: {
      dataset: { source: [['Mois', 'Ventes', 'Objectif'], ['Jan', 3, 4], ['Fév', 5, 6]] },
      xAxis: { type: 'category' }, series: [{ type: 'bar' }, { type: 'bar' }],
    } } },
  }]);
};
const removeSlide = (p) => apply(p, 'retrait', [{ op: 'remove_slide', slide: SLIDE }]);
/** Closes the pane and the inspector when open, with their own buttons (as 150-data-panel and 010-editor do). */
const closeInspectorAndPane = async (p) => {
  if (await p.locator('.data-pane').count()) {
    await p.locator('.data-pane').getByRole('button', { name: 'Fermer les données', exact: true }).click();
    await p.waitForSelector('.data-pane', { state: 'detached' });
  }
  if (await p.locator('.inspector').count()) {
    await p.locator('.inspector').getByRole('button', { name: "Fermer l'inspecteur", exact: true }).click();
    await p.waitForSelector('.inspector', { state: 'detached' });
  }
};

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG missing: run through just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await addChart(p);
  await p.reload();
  await p.waitForSelector('.stage img');
  assert.equal(await p.locator(OPEN).count(), 0, 'the inspector and the pane start closed');
  const selectedBefore = await p.locator('.overlay .target.selected').count();

  // 1. The agent edits the chart: its turn answers with a receipt, and nothing opens or gets selected.
  const from = logOffset(LOG);
  await prompt(p, TEXT);
  await waitForTool(p, from, 'apply');
  await waitForTool(p, from, 'report');
  // The traces row sits under the receipt card, in the same turn.
  const turn = p.locator('.agent-turn', { hasText: TEXT });
  await turn.locator('.a2ui-receipt').waitFor();
  await p.waitForTimeout(500);
  assert.equal(await p.locator(OPEN).count(), 0, 'nothing opens by itself');
  assert.equal(await p.locator('.overlay .target.selected').count(), selectedBefore, 'the selection is unchanged');

  // 2. Its trace shows the slide, selects the chart and opens the data pane on it.
  await turn.getByRole('button', { name: /données modifiées/ }).click();
  await p.locator('.data-pane [data-surface-id="host-data"] .chart-panel').waitFor({ timeout: 15000 });
  const targets = (await api(p, { cmd: 'slide', id: SLIDE })).targets;
  const at = targets.findIndex((t) => t.id === CHART);
  assert.ok(at >= 0, 'the chart is a target of its slide');
  await p.locator('.overlay .target.selected').waitFor();
  assert.equal(await p.locator('.overlay .target').nth(at).getAttribute('aria-selected'), 'true', 'the chart is selected');
  assert.equal(await p.locator('.overlay .target.selected').count(), 1, 'one target selected');

  // The pane and the inspector closed, then the slide removed: the suite starts from the original layout and slides.
  await closeInspectorAndPane(p);
  await removeSlide(p);
  assert.deepEqual(errors, []);
  console.log('hub-traces : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
