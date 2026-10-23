// Données d'un graphique et d'un tableau au dock de l'agent (surface `host-data`) : le graphique
// sélectionné y montre sa grille ; une cellule, son type, puis « Afficher en tableau » et une
// option du tableau changent le deck. L'élément est retiré à la fin : la suite garde la slide d'origine.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e
// (avant 160-widgets.cjs : l'onglet est encore en mode deck).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const DATA = '.agent-dock [data-surface-id="host-data"]';
const ID = 'e2e-chart';

const api = (p, req) => post(p, '/api', { tab: 't-1', req });
const view = (p) => api(p, { cmd: 'slide', id: 's-constat' });
/** L'élément `e2e-chart` de s-constat, tel que le deck le garde. */
const element = async (p) => (await view(p)).slide.params.blocks.find((e) => e.id === ID);

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

  // 1. Sélection du graphique : ses données au dock, rien sous la slide.
  const order = (await api(p, { cmd: 'snapshot' })).slides.map((s) => s.id);
  await p.click(`.slide-strip .thumb >> nth=${order.indexOf('s-constat')}`);
  await until(p, async () => (await view(p)).targets.some((t) => t.id === ID), 'le graphique est une cible');
  const at = (await view(p)).targets.findIndex((t) => t.id === ID);
  await p.locator('.overlay .target').nth(at).click({ position: { x: 4, y: 4 } });
  const chart = p.locator(`${DATA} .chart-panel`);
  await chart.waitFor({ timeout: 15000 });
  assert.equal(await p.locator('.sheet-area .chart-panel').count(), 0, 'pas de grille sous la slide');

  // 2. Une cellule : Jan / Ventes passe à 9 (validée en quittant le champ).
  const cell = chart.locator('input.cdg-number').first();
  await cell.fill('9');
  await cell.press('Tab');
  await until(p, async () => (await element(p))?.params.option.dataset.source[1][1] === 9, 'la cellule ne passe pas à 9');

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

  // L'élément retiré : la slide d'origine pour la suite.
  const removed = await api(p, { cmd: 'apply', label: 'retrait', merge: null, ops: [{ op: 'remove_element', slide: 's-constat', element: ID }] });
  assert.ok(!removed.error, JSON.stringify(removed));
  await p.waitForSelector(DATA, { state: 'detached' });

  assert.deepEqual(errors, []);
  console.log('data-panel : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
