// A query data in the side panel's Data tab (surface `deck-data`) and the data pane (`data-sheet`):
// a manual data added, a query added from the tab, its SQL written over that data and applied, its
// result shown read-only; a chart over the query, selected: its data pane (`host-data`) keeps its
// settings, a series color changed there, and shows the query's result and SQL read-only; a cell of the input edited in its own sheet, opened from that
// pane's input link: the query's result, the chart's pane and the chart change. The data and the
// chart are removed at the end. Against `oku-slides serve fixtures/roadmap.json`. Run: just e2e
// (after 150-data-panel.cjs, before 160-widgets.cjs: the tab is still in deck mode).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LIST = '.side-body [data-surface-id="deck-data"]';
const SHEET = '.data-pane [data-surface-id="data-sheet"]';
const DATA = '.data-pane [data-surface-id="host-data"]';
const CHART = 'e2e-query-chart';
const INPUT = 'e2e_ventes';
const SQL = `SELECT a, b FROM ${INPUT} WHERE b > 0`;

const api = (p, req) => post(p, '/api', { tab: 't-1', req });
/** The components of a surface the session answers for `req`. */
const components = async (p, req) => ((await api(p, req))?.messages ?? []).flatMap((m) => m.updateComponents?.components ?? []);
/** The deck's data rows, as the Data tab lists them. */
const dataRows = async (p) => (await components(p, { cmd: 'data_list' })).find((c) => c.component === 'DataList')?.rows ?? [];
/** The `DataSheet` of data `id` from its first row. */
const sheetOf = async (p, id) => (await components(p, { cmd: 'data_sheet', data: id, offset: 0 })).find((c) => c.component === 'DataSheet');
/** The `ChartData` of the chart's data pane. */
const paneData = async (p) => (await components(p, { cmd: 'data_panel', slide: 's-constat', element: CHART }))
  .find((c) => c.component === 'ChartData');
/** The page hash of slide s-constat: it changes when its chart's rows do. */
const hash = async (p) => (await api(p, { cmd: 'snapshot' })).slides.find((s) => s.id === 's-constat').hash;

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.waitForSelector('.stage img');

  // 1. A manual data: `a` text, `b` number; one row above zero.
  const added = await api(p, {
    cmd: 'apply', label: 'données', merge: null,
    ops: [{ op: 'add_data', name: INPUT, table: { generation: 'g1',
      columns: [{ id: 'c1', name: 'a', type: 'text' }, { id: 'c2', name: 'b', type: 'number' }],
      rows: [{ id: 'r0', cells: ['x', 1] }, { id: 'r1', cells: ['y', -1] }] } }],
  });
  assert.ok(!added.error, JSON.stringify(added));
  const input = (await dataRows(p)).find((r) => r.name === INPUT);
  assert.equal(input?.kind, 'manual');

  // 2. A query added from the Data tab: listed as a query, with its state.
  const before = (await dataRows(p)).map((r) => r.id);
  await p.click('.side-rail .data-toggle');
  await p.locator(LIST).getByRole('button', { name: 'Ajouter une requête', exact: true }).click();
  let query;
  await until(p, async () => (query = (await dataRows(p)).find((r) => !before.includes(r.id))), "aucune requête n'est ajoutée");
  assert.equal(query.kind, 'query');
  const row = p.locator(`${LIST} [data-testid="data-row-${query.name}"]`);
  await row.locator('.data-row-state').waitFor({ timeout: 15000 });

  // 3. Its sheet: the SQL written over the manual data and applied; its result, read-only.
  await row.locator('.data-row-name').click();
  const sheet = p.locator(SHEET);
  await sheet.locator('.data-query-sql').fill(SQL);
  await sheet.locator('.data-query-apply').click();
  await until(p, async () => {
    const s = await sheetOf(p, query.id);
    return s?.query?.sql === SQL && s.query.state === 'ready' && s.total === 1;
  }, "la requête n'est pas appliquée");
  await until(p, async () => (await sheet.locator('.data-query-result tbody tr').count()) === 1, "le résultat ne s'affiche pas");
  assert.equal(await sheet.locator('.data-query-result input, .data-query-result select').count(), 0, 'the result is read-only');
  assert.equal((await sheet.locator('.data-query-input').textContent()), INPUT, 'the query links its input');

  // 4. A chart over the query.
  const made = await api(p, {
    cmd: 'apply', label: 'graphique', merge: null,
    ops: [{
      op: 'add_element', slot: { owner: { slide: 's-constat' }, param: 'blocks' }, defaults: true,
      element: { id: CHART, kind: 'base/chart', params: { option: {
        data: query.id, xAxis: { type: 'category' }, series: [{ type: 'bar' }],
      } } },
    }],
  });
  assert.ok(!made.error, JSON.stringify(made));

  // 5. The chart selected (the page reloaded to show it), its data from the inspector: its
  // settings, and the query's result and SQL, read-only.
  await p.reload();
  await p.waitForSelector('.stage img');
  const order = (await api(p, { cmd: 'snapshot' })).slides.map((s) => s.id);
  await p.click(`.slide-strip .thumb >> nth=${order.indexOf('s-constat')}`);
  await until(p, async () => (await api(p, { cmd: 'slide', id: 's-constat' })).targets.some((t) => t.id === CHART), 'le graphique est une cible');
  const at = (await api(p, { cmd: 'slide', id: 's-constat' })).targets.findIndex((t) => t.id === CHART);
  await p.locator('.overlay .target').nth(at).click({ position: { x: 4, y: 4 } });
  await p.click('.side-rail .inspector-toggle');
  await p.locator('.inspector').getByRole('button', { name: 'Données', exact: true }).click();
  const pane = p.locator(DATA);
  await until(p, async () => (await pane.locator('.data-query-result tbody tr').count()) === 1, "le panneau du graphique ne montre pas le résultat");
  const result = pane.locator('.data-query-result, .data-query');
  assert.equal(await result.locator('input, select, .data-query-apply').count(), 0, 'the result is read-only');
  assert.equal(await pane.locator('.data-query-sql').inputValue(), SQL);
  assert.equal(await pane.locator('.data-query-sql').getAttribute('readonly'), '');
  assert.equal(await pane.locator('.data-pick').count(), 1, 'the pane keeps the view settings');

  // 6. A series color picked in the pane: a view setting, applied to the chart over the query.
  const palette = (await paneData(p)).palette;
  await pane.locator('.series-chip').first().click();
  await pane.locator('.swatches .swatch').nth(2).click();
  await until(p, async () => (await paneData(p))?.series[0]?.color === palette[1], "la couleur de la série ne s'applique pas");
  const drawn = await hash(p);

  // 7. The input opened from the pane's link, its cell (row 2, b) set above zero: the query's
  // result gains the row, the chart's pane shows it and the chart redraws.
  await pane.locator('.data-query-input').click();
  const cell = sheet.locator('[data-testid="cdg-cell-2-1"]');
  await cell.waitFor({ timeout: 15000 });
  await cell.fill('5');
  await cell.press('Tab');
  await until(p, async () => (await sheetOf(p, query.id))?.total === 2, "le résultat de la requête ne change pas");
  await until(p, async () => (await paneData(p))?.source.length === 3, 'le panneau du graphique ne change pas');
  await until(p, async () => (await hash(p)) !== drawn, 'le graphique sur la requête ne change pas');

  // The pane and the inspector closed, the chart and both data removed, the Data tab folded.
  await p.locator('.data-pane').getByRole('button', { name: 'Fermer les données', exact: true }).click();
  await p.waitForSelector('.data-pane', { state: 'detached' });
  await p.click('.side-rail .inspector-toggle');
  await p.waitForSelector('.inspector', { state: 'detached' });
  const removed = await api(p, { cmd: 'apply', label: 'retrait', merge: null, ops: [{ op: 'remove_element', slide: 's-constat', element: CHART }] });
  assert.ok(!removed.error, JSON.stringify(removed));
  for (const data of [query.id, input.id]) {
    const gone = await api(p, { cmd: 'data_action', action: { action: 'remove', data } });
    assert.ok(!gone.error, JSON.stringify(gone));
  }
  await until(p, async () => !(await dataRows(p)).some((r) => r.id === query.id || r.id === input.id), 'les données restent');
  await p.click('.side-rail .data-toggle');
  await p.waitForSelector(LIST, { state: 'detached' });

  assert.deepEqual(errors, []);
  console.log('derived-data : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
