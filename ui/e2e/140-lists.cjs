// Frise (widget de pack des tests, fixtures/widgets/frise-test.typ) : ajouter une étape par le
// tableau du panneau des paramètres, la saisir, la déplacer au clavier depuis la slide, la supprimer
// par le tableau puis annuler. Aucune poignée d'item sur la scène.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e
// (avant 160-widgets.cjs : l'onglet est encore en mode deck ; la frise reste sur s-constat).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const fs = require('node:fs');
const path = require('node:path');
const FRISE = fs.readFileSync(path.join(__dirname, '../../fixtures/widgets/frise-test.typ'), 'utf8');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const MOD = process.platform === 'darwin' ? 'Meta' : 'Control';

const view = (p) => post(p, '/api', { tab: 't-1', req: { cmd: 'slide', id: 's-constat' } });
const items = async (p) => (await view(p)).items.filter((i) => i.owner === 'f');
const ids = async (p) => (await items(p)).map((i) => i.item);
const waitLen = (p, n, why) => until(p, async () => (await ids(p)).length === n, why);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 2600, height: 1500 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  // Le widget `frise` est écrit dans le pack du deck (set_widget en mode deck), puis
  // l'élément est créé par l'API, avec ses quatre étapes par défaut (d0…d3).
  const set = await post(p, '/api', { tab: 't-1', req: { cmd: 'set_widget', name: 'frise', source: FRISE } });
  assert.ok(!set?.error, JSON.stringify(set));
  const made = await post(p, '/api', {
    tab: 't-1',
    req: {
      cmd: 'apply',
      label: 'frise',
      merge: null,
      ops: [{ op: 'add_element', slot: { owner: { slide: 's-constat' }, param: 'blocks' }, element: { id: 'f', kind: 'deck/frise' }, defaults: true }],
    },
  });
  assert.ok(!made.error, JSON.stringify(made));
  await p.reload();
  await p.waitForSelector('.stage img');
  assert.deepEqual(await ids(p), ['d0', 'd1', 'd2', 'd3'], 'les quatre étapes par défaut sont amorcées');

  // Aller à s-constat, sélectionner la frise.
  const order = (await post(p, '/api', { tab: 't-1', req: { cmd: 'snapshot' } })).slides.map((s) => s.id);
  const n = order.indexOf('s-constat') + 1;
  await p.click(`.thumb >> nth=${n - 1}`);
  await p.waitForFunction((n) => {
    const img = document.querySelector('.stage img');
    return img?.alt === `Slide ${n}` && img.complete && img.naturalWidth > 0;
  }, n);
  const v = await view(p);
  const at = v.targets.findIndex((t) => t.id === 'f');
  assert.ok(at >= 0, 'la frise est une cible de la scène');
  const target = p.locator('.overlay .target').nth(at);
  await target.click({ position: { x: 4, y: 4 } });
  await p.waitForSelector('.overlay .target.selected');
  // Pas de « + » sur la scène : l'ajout passe par le tableau du panneau des paramètres, à droite.
  assert.equal(await p.locator('[data-item-insert]').count(), 0, 'aucun « + » sur la scène');
  const PANEL = '[data-surface-id="host-target"]';
  await p.waitForSelector(`${PANEL} [data-list-add]`, { timeout: 15000 });
  await p.locator(`${PANEL} [data-list-add]`).click();
  await waitLen(p, 5, 'cinq étapes après ajout');
  const added = (await ids(p)).find((i) => !['d0', 'd1', 'd2', 'd3'].includes(i));
  assert.ok(added, 'nouvel id');
  assert.equal((await ids(p))[4], added, 'ajouté en dernier');

  // Saisie dans le tableau : « 2027 » dans la date du nouvel item ; un clic sur la scène, hors du
  // tableau, la valide ; set_field relu dans la vue.
  const date = p.locator(`${PANEL} tr[data-row="${added}"] td[data-col="date"] input`);
  await date.waitFor({ timeout: 15000 });
  await date.fill('2027');
  await target.click({ position: { x: 4, y: 4 } });
  await p.waitForSelector('.overlay .target.selected');
  const textOf = async (id) => (await view(p)).fields.find((f) => f.field === `etapes/${id}/date`)?.text;
  await until(p, async () => (await textOf(added)) === '2027', 'date saisie relue dans la vue');
  assert.equal((await ids(p)).length, 5);

  // Déplacement au clavier depuis un champ édité : Cmd/Ctrl+Alt+← (axe horizontal) d'un rang.
  // Les champs de la scène suivent l'ordre de la vue.
  const fieldAt = (await view(p)).fields.findIndex((f) => f.field === `etapes/${added}/date`);
  assert.ok(fieldAt >= 0);
  await p.locator('.fields .field').nth(fieldAt).dblclick();
  await p.waitForSelector('.editor .cm-editor.cm-focused');
  await p.keyboard.press(`${MOD}+Alt+ArrowLeft`);
  await waitLen(p, 5, 'cinq étapes après déplacement');
  await until(p, async () => (await ids(p))[3] === added, "avancé d'un rang au clavier");
  await p.keyboard.press('Escape');
  await p.waitForSelector('.editor', { state: 'detached' });

  // Aucune poignée d'item (⋮⋮, « × ») sur la scène, la frise sélectionnée.
  await target.click({ position: { x: 4, y: 4 } });
  await p.waitForSelector('.overlay .target.selected');
  assert.equal(await p.locator('.items, [data-item-remove], [data-item-move]').count(), 0, "aucune poignée d'item sur la scène");

  // « × » du tableau sur le nouvel item : quatre items ; Cmd/Ctrl+Z : cinq, à sa place.
  const before = await ids(p);
  await p.locator(`${PANEL} tr[data-row="${added}"] [data-list-remove]`).click();
  await waitLen(p, 4, 'quatre étapes après suppression');
  assert.ok(!(await ids(p)).includes(added));
  await p.locator('body').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press(`${MOD}+z`);
  await waitLen(p, 5, 'cinq étapes après annulation');
  assert.deepEqual(await ids(p), before, "l'annulation rend l'étape à sa place");

  assert.deepEqual(errors, []);
  console.log('lists : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
