// A chart's and a table's data in the data pane (surface `host-data`), opened from the inspector:
// the selected chart shows its grid there; a cell, its type, then "Show as table" and a table option
// change the deck. Then the side panel's Data tab (surface `deck-data`): the chart's data is in use
// and refuses removal inline; a data added, renamed, opened in the pane (`data-sheet`), its column
// retyped, then removed. Then one data shown by a chart and a table: a cell set in the table's pane
// shows in the chart's grid, the tab counts two views and its refusal names both, and a pasted copy
// of the chart brings `<name>_2`. The panes close and the elements are removed at the end: the
// suite keeps the original layout and slide. Against `oku-slides serve fixtures/roadmap.json`. Run: just e2e
// (before 160-widgets.cjs: the tab is still in deck mode).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { clipboard } = require('./lib/clipboard.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const DATA = '.data-pane [data-surface-id="host-data"]';
const LIST = '.side-body [data-surface-id="deck-data"]';
const SHEET = '.data-pane [data-surface-id="data-sheet"]';
const ID = 'e2e-chart';
const CHART = 'e2e-shared-chart';
const TABLE = 'e2e-shared-table';

const api = (p, req) => post(p, '/api', { tab: 't-1', req });
const view = (p) => api(p, { cmd: 'slide', id: 's-constat' });
/** The element `id` of s-constat, as the deck holds it. */
const elementOf = async (p, id) => (await view(p)).slide.params.blocks.find((e) => e.id === id);
const element = (p) => elementOf(p, ID);
/** The source the data pane of element `id` shows: the deck data its view names. */
const sourceOf = async (p, id) => (await components(p, { cmd: 'data_panel', slide: 's-constat', element: id }))
  .find((c) => c.component === 'ChartData' || c.component === 'TableData')?.source;
const paneSource = (p) => sourceOf(p, ID);
/** Selects element `id` of the shown slide on the stage. */
const select = async (p, id) => {
  await until(p, async () => (await view(p)).targets.some((t) => t.id === id), `${id} est une cible`);
  const at = (await view(p)).targets.findIndex((t) => t.id === id);
  await p.locator('.overlay .target').nth(at).click({ position: { x: 4, y: 4 } });
};
/** Element `id` selected, its data in the pane through the inspector. */
const openPane = async (p, id) => {
  await select(p, id);
  await p.click('.side-rail .inspector-toggle');
  await p.locator('.inspector').getByRole('button', { name: 'Données', exact: true }).click();
  await p.locator(`${DATA} [data-testid="cdg-cell-1-1"]`).waitFor({ timeout: 15000 });
};
/** The pane and the inspector closed. */
const closePane = async (p) => {
  await p.locator('.data-pane').getByRole('button', { name: 'Fermer les données', exact: true }).click();
  await p.waitForSelector('.data-pane', { state: 'detached' });
  await p.click('.side-rail .inspector-toggle');
  await p.waitForSelector('.inspector', { state: 'detached' });
};
/** The digits of a number as the grid writes it (grouping spaces apart). */
const digits = (shown) => shown.replace(/\D/g, '');

/** The components of a surface the session answers for `req`. */
const components = async (p, req) => ((await api(p, req))?.messages ?? []).flatMap((m) => m.updateComponents?.components ?? []);
/** The deck's data rows, as the Data tab lists them. */
const dataRows = async (p) => (await components(p, { cmd: 'data_list' })).find((c) => c.component === 'DataList')?.rows ?? [];

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  const made = await api(p, {
    cmd: 'apply',
    label: 'graphique',
    merge: null,
    ops: [{
      op: 'add_element', slot: { owner: { slide: 's-constat' }, param: 'blocks' }, defaults: true,
      element: { id: ID, kind: 'base/chart', params: { option: {
        dataset: { source: [['Mois', 'Ventes', 'Objectif'], ['Jan', 3, 4], ['Fév', 5, 6]] },
        xAxis: { type: 'category' }, series: [{ type: 'bar' }, { type: 'bar' }],
      } } },
    }],
  });
  assert.ok(!made.error, JSON.stringify(made));
  await p.reload();
  await p.waitForSelector('.stage img');

  // 1. The chart selected, then the inspector and its Data button: its data in the pane, under the slide.
  const order = (await api(p, { cmd: 'snapshot' })).slides.map((s) => s.id);
  await p.click(`.slide-strip .thumb >> nth=${order.indexOf('s-constat')}`);
  await select(p, ID);
  await p.click('.side-rail .inspector-toggle');
  await p.locator('.inspector').getByRole('button', { name: 'Données', exact: true }).click();
  const chart = p.locator(`${DATA} .chart-panel`);
  await chart.waitFor({ timeout: 15000 });
  assert.equal(await p.locator('.sheet-area > .data-pane .chart-panel').count(), 1, 'the grid is in the data pane');
  assert.equal(await p.locator('.agent-dock [data-surface-id="host-data"]').count(), 0, "nothing at the agent's dock");

  // 2. A cell: Jan / Ventes becomes 9 (committed on leaving the field), in the deck data.
  const cell = chart.locator('input.cdg-number').first();
  await cell.fill('9');
  await cell.press('Tab');
  await until(p, async () => (await paneSource(p))?.[1]?.[1] === 9, 'la cellule ne passe pas à 9');
  assert.equal(typeof (await element(p)).params.option.data, 'string', 'the chart names its deck data');

  // 3. Le type : Lignes.
  await chart.getByRole('button', { name: 'Lignes', exact: true }).click();
  await until(p, async () => (await element(p))?.params.option.series[0].type === 'line', 'le graphique ne passe pas en lignes');
  await until(p, async () => (await chart.getByRole('button', { name: 'Lignes', exact: true }).getAttribute('aria-pressed')) === 'true', 'le type relu ne se montre pas');

  // 4. Afficher en tableau, puis l'option « Lignes zébrées ».
  await chart.locator('.show-as button').click();
  await until(p, async () => (await element(p))?.kind === 'base/table', "le graphique ne devient pas un tableau");
  const table = p.locator(`${DATA} .table-panel`);
  await table.waitFor({ timeout: 15000 });
  await table.getByRole('button', { name: 'Lignes zébrées', exact: true }).click();
  await until(p, async () => (await element(p))?.params.data.striped === true, "l'option zébrée n'est pas posée");

  // The pane and the inspector closed, then the element removed: the suite starts from the original layout and slide.
  await p.locator('.data-pane').getByRole('button', { name: 'Fermer les données', exact: true }).click();
  await p.waitForSelector('.data-pane', { state: 'detached' });
  await p.click('.side-rail .inspector-toggle');
  await p.waitForSelector('.inspector', { state: 'detached' });

  // 5. The Data tab: the chart's data, shown by one view, refuses removal on its row.
  await p.click('.side-rail .data-toggle');
  const used = p.locator(`${LIST} .data-row`).filter({ has: p.locator('.data-row-user') });
  await used.first().waitFor({ timeout: 15000 });
  assert.equal(await used.first().locator('.data-row-views').textContent(), '1 vue');
  await used.first().locator('.data-row-remove').click();
  await used.first().locator('.data-row-error').waitFor({ timeout: 15000 });

  // 6. A data added (unused), renamed, opened in the pane, its column retyped as numbers.
  const before = (await dataRows(p)).map((r) => r.id);
  await p.locator(LIST).getByRole('button', { name: 'Ajouter des données', exact: true }).click();
  let added;
  await until(p, async () => (added = (await dataRows(p)).find((r) => !before.includes(r.id))), "aucune donnée n'est ajoutée");
  assert.deepEqual([added.columns, added.rows, added.users.length], [1, 1, 0]);
  const row = p.locator(`${LIST} .data-row`).filter({ has: p.locator('.data-row-unused') });
  await row.locator('.data-row-rename').fill('e2e-donnees');
  await row.locator('.data-row-rename').press('Enter');
  await until(p, async () => (await dataRows(p)).find((r) => r.id === added.id)?.name === 'e2e-donnees', "le renommage n'est pas posé");
  await p.locator(`${LIST} .data-row-name`, { hasText: 'e2e-donnees' }).click();
  const sheet = p.locator(SHEET);
  await sheet.locator('select.cdg-type').first().waitFor({ timeout: 15000 });
  await sheet.locator('select.cdg-type').first().selectOption('number');
  const types = async () => (await components(p, { cmd: 'data_sheet', data: added.id, offset: 0 })).find((c) => c.component === 'DataSheet')?.types;
  await until(p, async () => (await types())?.[0] === 'number', "le type de colonne n'est pas posé");

  // 7. The pane closed, the added data removed, the Data tab folded.
  await p.locator('.data-pane').getByRole('button', { name: 'Fermer les données', exact: true }).click();
  await p.waitForSelector('.data-pane', { state: 'detached' });
  await p.locator(`${LIST} .data-row`).filter({ has: p.locator('.data-row-unused') }).locator('.data-row-remove').click();
  await until(p, async () => !(await dataRows(p)).some((r) => r.id === added.id), "la donnée n'est pas retirée");
  await p.click('.side-rail .data-toggle');
  await p.waitForSelector(LIST, { state: 'detached' });

  const removed = await api(p, { cmd: 'apply', label: 'retrait', merge: null, ops: [{ op: 'remove_element', slide: 's-constat', element: ID }] });
  assert.ok(!removed.error, JSON.stringify(removed));
  await p.waitForSelector(DATA, { state: 'detached' });

  // 8. One data, two views: a chart brings it, a table is bound to it.
  const shared = await api(p, {
    cmd: 'apply', label: 'graphique partagé', merge: null,
    ops: [{
      op: 'add_element', slot: { owner: { slide: 's-constat' }, param: 'blocks' }, defaults: true,
      element: { id: CHART, kind: 'base/chart', params: { option: {
        dataset: { source: [['a', 'b'], ['x', 1], ['y', 2]] }, xAxis: { type: 'category' }, series: [{ type: 'bar' }],
      } } },
    }],
  });
  assert.ok(!shared.error, JSON.stringify(shared));
  const dataId = (await elementOf(p, CHART)).params.option.data;
  assert.equal(typeof dataId, 'string', 'the chart names its deck data');
  const bound = await api(p, {
    cmd: 'apply', label: 'tableau lié', merge: null,
    ops: [{
      op: 'add_element', slot: { owner: { slide: 's-constat' }, param: 'blocks' }, defaults: true,
      element: { id: TABLE, kind: 'base/table', params: { data: { data: dataId } } },
    }],
  });
  assert.ok(!bound.error, JSON.stringify(bound));
  const name = (await dataRows(p)).find((r) => r.id === dataId).name;
  await p.reload();
  await p.waitForSelector('.stage img');
  await p.click(`.slide-strip .thumb >> nth=${order.indexOf('s-constat')}`);

  // 9. In the table's pane, (row 1, column b) becomes 918273; the chart's grid shows it.
  await openPane(p, TABLE);
  const tableCell = p.locator(`${DATA} [data-testid="cdg-cell-1-1"]`);
  await tableCell.fill('918273');
  await tableCell.press('Tab');
  await until(p, async () => (await sourceOf(p, CHART))?.[1]?.[1] === 918273, "la cellule n'atteint pas le graphique");
  await closePane(p);
  await openPane(p, CHART);
  await p.locator(`${DATA} .chart-panel`).waitFor({ timeout: 15000 });
  await until(p, async () => digits(await p.locator(`${DATA} [data-testid="cdg-cell-1-1"]`).inputValue()) === '918273', 'la grille du graphique ne montre pas 918273');
  await closePane(p);

  // 10. The Data tab: the data, shown by two views, refuses removal and names both.
  await p.click('.side-rail .data-toggle');
  const sharedRow = p.locator(`${LIST} [data-testid="data-row-${name}"]`);
  await sharedRow.waitFor({ timeout: 15000 });
  assert.equal(await sharedRow.locator('[data-testid="data-row-views"]').textContent(), '2 vues');
  await sharedRow.locator('[data-testid="data-row-remove"]').click();
  const refusal = sharedRow.locator('[data-testid="data-row-error"]');
  await refusal.waitFor({ timeout: 15000 });
  const said = (await refusal.textContent()).replace(/[\u2068\u2069]/g, '');
  assert.ok(said.includes(CHART) && said.includes(TABLE), `the refusal names both views: ${said}`);

  // 11. The chart copied and pasted: its copy brings a data of its own, `<name>_2`.
  const blocks = async () => (await view(p)).slide.params.blocks.map((e) => e.id);
  const ids = await blocks();
  await select(p, CHART);
  const text = await clipboard(p, 'copy');
  await clipboard(p, 'paste', text);
  let pasted;
  await until(p, async () => (pasted = (await blocks()).find((id) => !ids.includes(id))), "le graphique n'est pas collé");
  await p.locator(`${LIST} [data-testid="data-row-${name}_2"]`).waitFor({ timeout: 15000 });
  await p.click('.side-rail .data-toggle');
  await p.waitForSelector(LIST, { state: 'detached' });

  const cleared = await api(p, {
    cmd: 'apply', label: 'retrait', merge: null,
    ops: [CHART, TABLE, pasted].map((element) => ({ op: 'remove_element', slide: 's-constat', element })),
  });
  assert.ok(!cleared.error, JSON.stringify(cleared));

  assert.deepEqual(errors, []);
  console.log('data-panel : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
