// Liste au dock d'un widget du pack : le tableau (ajout, retrait) ; l'aperçu revient à l'identique.
// Le popover d'une cellule riche s'ouvre sous sa cellule, dans le tableau.
// Puis un ajout par le « + » de l'aperçu : un item de plus aux valeurs d'exemple, au tableau comme sur l'aperçu.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e (après widgets.cjs).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');

const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const post = (p, endpoint, req) =>
  p.evaluate(
    ([e, r]) => fetch(e, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(r) }).then((x) => x.json()),
    [endpoint, req],
  );

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  // Le test précédent peut avoir laissé l'onglet en mode modèles : on y entre s'il le faut.
  await p.waitForSelector('.stage img, .widget-pane', { state: 'attached' });
  const add = p.getByRole('button', { name: 'Nouveau widget' });
  if (!(await add.isVisible())) {
    await p.getByRole('button', { name: 'Menu' }).click();
    await p.getByRole('menuitem', { name: 'Éditer les widgets' }).click();
  }
  await add.click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('jalons');
  await name.press('Enter');
  const IMG = '.widget-tab:not([hidden]) .widget-tab-preview img';
  await p.waitForSelector(IMG);
  const svgOf = () => p.evaluate(async (sel) => {
    const img = document.querySelector(sel);
    return img ? (await fetch(img.src)).text() : null;
  }, IMG);
  // Le widget d'essai déclare une liste de trois colonnes (dont une riche), trois lignes par défaut.
  const source = [
    '#import "/oku-slides/lib.typ": widget, field, list-item, params as p',
    '#let widget = widget("jalons", label: "Jalons",',
    '  params: (etapes: p.list(label: "Étapes", item-label: "jalon", min: 1, max: 6,',
    '    item: (date: p.plain(label: "Date"), texte: p.plain(label: "Texte", default: "Nouveau"), note: p.rich(label: "Note")),',
    '    default: ((date: "2019", texte: "A"), (date: "2021", texte: "B"), (date: "2023", texte: "C")))),',
    '  (args, ctx) => stack(spacing: 6pt, ..args.etapes.map(e => list-item(ctx, "etapes", e,',
    '    c => field(c, "texte", e.at("texte", default: "")))))) ',
    '',
  ].join('\n');
  const set = await post(p, '/api', { tab: 't-1', req: { cmd: 'set_widget', name: 'jalons', source } });
  assert.ok(!set?.error, JSON.stringify(set));
  await p.reload();
  const table = '[data-surface-id="host-widget"]';
  await p.waitForSelector(`${table} [data-list-add]`, { timeout: 15000 });
  const rows = () => p.locator(`${table} tbody tr`).count();
  assert.equal(await rows(), 3, 'trois lignes par défaut');
  const before = await svgOf();
  assert.ok(before, 'aperçu rendu');
  const rowsAre = (n) => p.waitForFunction(([sel, n]) => document.querySelectorAll(`${sel} tbody tr`).length === n, [table, n]);
  const preview = (same) => p.waitForFunction(async ([sel, was, same]) => {
    const img = document.querySelector(sel);
    return !!img && ((await (await fetch(img.src)).text()) === was) === same;
  }, [IMG, before, same], { timeout: 15000 });
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

  // Les poignées de liste sur l'aperçu : le « + » après le dernier item en ajoute un aux valeurs d'exemple.
  const STAGE = '.widget-tab:not([hidden]) .stage-preview';
  const handles = (n) => p.waitForFunction(([sel, n]) => document.querySelectorAll(`${sel} .item-handles`).length === n, [STAGE, n], { timeout: 15000 });
  await handles(3);
  await p.hover(`${STAGE} .stage`);
  await p.locator(`${STAGE} [data-item-insert]`).last().click();
  await rowsAre(4);
  await handles(4);

  assert.deepEqual(errors, []);
  console.log('lists-dock : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
