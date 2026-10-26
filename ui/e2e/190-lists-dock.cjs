// Liste au dock d'un widget du pack : le tableau (ajout, retrait) ; l'aperçu revient à l'identique.
// Le popover d'une cellule riche s'ouvre sous sa cellule, dans le tableau.
// In Essentiel's pack tab. Writes: Essentiel (the `jalons` widget). Leaves: no pack tab.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e (après 160-widgets.cjs).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openEssentiel } = require('./lib/atelier.cjs');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { PREVIEW, widgetPreviewSvg } = require('./lib/widget.cjs');

const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');
  const packTab = await openEssentiel(p);
  await p.getByRole('button', { name: 'Nouveau widget' }).click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('jalons');
  await name.press('Enter');
  await p.waitForSelector(PREVIEW);
  // Le widget d'essai déclare une liste de trois colonnes (dont une riche), trois lignes par défaut.
  // Un élément dans un pack de widgets ; une slide dans un pack de layouts (Essentiel, ouvert ici).
  const kind = (await post(p, '/api', { tab: packTab, req: { cmd: 'snapshot' } })).editing?.kind;
  const slide = kind === 'layouts';
  const items = 'stack(spacing: 6pt, ..args.etapes.map(e => list-item(ctx, "etapes", e, c => field(c, "texte", e.at("texte", default: "")))))';
  const source = [
    '#import "/oku-slides/lib.typ": widget, field, list-item, blank-slide, params as p',
    `#let widget = widget("jalons", label: "Jalons",${slide ? ' category: "slide",' : ''}`,
    '  params: (etapes: p.list(label: "Étapes", item-label: "jalon", min: 1, max: 6,',
    '    item: (date: p.plain(label: "Date"), texte: p.plain(label: "Texte", default: "Nouveau"), note: p.rich(label: "Note")),',
    '    default: ((date: "2019", texte: "A"), (date: "2021", texte: "B"), (date: "2023", texte: "C")))),',
    `  (args, ctx) => ${slide ? `blank-slide(id: ctx.id, ${items})` : items})`,
    '',
  ].join('\n');
  const set = await post(p, '/api', { tab: packTab, req: { cmd: 'set_widget', name: 'jalons', source } });
  assert.ok(!set?.error, JSON.stringify(set));
  await p.reload();
  const table = '[data-surface-id="host-widget"]';
  await p.waitForSelector(`${table} [data-list-add]`, { timeout: 15000 });
  const rows = () => p.locator(`${table} tbody tr`).count();
  assert.equal(await rows(), 3, 'trois lignes par défaut');
  const before = await widgetPreviewSvg(p);
  assert.ok(before, 'aperçu rendu');
  const rowsAre = (n) => p.waitForFunction(([sel, n]) => document.querySelectorAll(`${sel} tbody tr`).length === n, [table, n]);
  const preview = (same) => until(p, async () => {
    const now = await widgetPreviewSvg(p);
    return now !== null && (now === before) === same;
  }, same ? "l'aperçu ne revient pas à l'identique" : "l'aperçu ne change pas", 15000);
  await p.locator(`${table} [data-list-add]`).click();
  await rowsAre(4);
  await preview(false); // la ligne ajoutée change l'aperçu
  await p.locator(`${table} [data-list-remove]`).last().click();
  await rowsAre(3);
  await preview(true); // la retirer le rend à l'identique

  // Popover d'une cellule riche : sous sa cellule, sans déborder du tableau à droite.
  const rich = p.locator(`${table} tbody tr`).first().locator('[data-list-rich]');
  await rich.click();
  const pop = p.locator(`${table} .lt-popover`);
  await pop.waitFor();
  const cellBox = await rich.locator('xpath=ancestor::td[1]').boundingBox();
  const popBox = await pop.boundingBox();
  const ltBox = await p.locator(`${table} .lt`).boundingBox();
  assert.ok(Math.abs(popBox.y - (cellBox.y + cellBox.height)) <= 1, `popover sous sa cellule : ${JSON.stringify({ cellBox, popBox })}`);
  assert.ok(popBox.x >= ltBox.x - 1 && popBox.x + popBox.width <= ltBox.x + ltBox.width + 1, `popover dans le tableau : ${JSON.stringify({ ltBox, popBox })}`);
  await p.locator(`${table} thead th`).first().click(); // un clic ailleurs le ferme
  await pop.waitFor({ state: 'detached' });

  await closePackTabs(p);
  assert.deepEqual(errors, []);
  console.log('lists-dock : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
