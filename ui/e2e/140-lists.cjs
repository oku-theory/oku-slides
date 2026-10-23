// Frise (widget de pack des tests, fixtures/widgets/frise-test.typ) : aucune poignée d'item sur la
// scène ; une étape se déplace au clavier depuis la slide, puis l'annulation la remet à sa place.
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
  // Ni « + » ni poignée d'item (⋮⋮, « × ») sur la scène : les listes se changent en parlant à l'agent.
  assert.equal(await p.locator('[data-item-insert]').count(), 0, 'aucun « + » sur la scène');
  assert.equal(await p.locator('.items, [data-item-remove], [data-item-move]').count(), 0, "aucune poignée d'item sur la scène");

  // Déplacement au clavier depuis un champ édité : Cmd/Ctrl+Alt+← (axe horizontal) d'un rang.
  // Les champs de la scène suivent l'ordre de la vue.
  const before = await ids(p);
  const fieldAt = (await view(p)).fields.findIndex((f) => f.field === 'etapes/d3/date');
  assert.ok(fieldAt >= 0);
  await p.locator('.fields .field').nth(fieldAt).dblclick();
  await p.waitForSelector('.editor .cm-editor.cm-focused');
  await p.keyboard.press(`${MOD}+Alt+ArrowLeft`);
  await until(p, async () => (await ids(p))[2] === 'd3', "avancé d'un rang au clavier");
  await p.keyboard.press('Escape');
  await p.waitForSelector('.editor', { state: 'detached' });

  // Cmd/Ctrl+Z : l'étape revient à sa place.
  await p.locator('body').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press(`${MOD}+z`);
  await until(p, async () => JSON.stringify(await ids(p)) === JSON.stringify(before), "l'annulation rend l'étape à sa place");

  assert.deepEqual(errors, []);
  console.log('lists : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
