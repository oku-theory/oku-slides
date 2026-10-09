// Panneau des paramètres de la cible (colonne de droite, `host-target`) : l'élément sélectionné
// (la frise de s-constat) montre « Paramètres de … » et son tableau ; saisie, ajout et annulation
// passent par ce panneau ; sans sélection, la carte est celle du widget de la slide (s-ch1).
// roadmap.json n'a pas de graphique : le cas « graphique sélectionné, pas de carte » n'est pas couvert ici
// (il l'est par les tests de session et de rendu).
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e
// (avant widgets.cjs : l'onglet est encore en mode deck).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');
const { until } = require('./until.cjs');
const fs = require('node:fs');
const path = require('node:path');
const FRISE = fs.readFileSync(path.join(__dirname, '../../fixtures/widgets/frise-test.typ'), 'utf8');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const MOD = process.platform === 'darwin' ? 'Meta' : 'Control';
const PANEL = '[data-surface-id="host-target"]';

const view = (p) => post(p, '/api', { tab: 't-1', req: { cmd: 'slide', id: 's-constat' } });
const items = async (p) => (await view(p)).items.filter((i) => i.owner === 'f');
const ids = async (p) => (await items(p)).map((i) => i.item);
const waitLen = (p, n, why) => until(p, async () => (await ids(p)).length === n, why);
const stageReady = (p, n) =>
  p.waitForFunction((n) => {
    const img = document.querySelector('.stage img');
    return img?.alt === `Slide ${n}` && img.complete && img.naturalWidth > 0;
  }, n);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 2600, height: 1500 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  const set = await post(p, '/api', { tab: 't-1', req: { cmd: 'set_widget', name: 'frise', source: FRISE } });
  assert.ok(!set?.error, JSON.stringify(set));
  // Dans la suite, lists.cjs a déjà posé la frise `f` sur s-constat : on la réutilise telle quelle.
  if (!(await view(p)).targets?.some((t) => t.id === 'f')) {
    const made = await post(p, '/api', {
      tab: 't-1',
      req: {
        cmd: 'apply',
        label: 'frise',
        merge: null,
        ops: [{ op: 'add_element', slot: { owner: { slide: 's-constat' }, param: 'blocks' }, element: { id: 'f', kind: 'frise' }, defaults: true }],
      },
    });
    assert.ok(!made.error, JSON.stringify(made));
  }
  await p.reload();
  await p.waitForSelector('.stage img');
  const original = await ids(p);
  assert.ok(original.length >= 4, `la frise a ses étapes : ${original}`);

  const order = (await post(p, '/api', { tab: 't-1', req: { cmd: 'snapshot' } })).slides.map((s) => s.id);
  const n = order.indexOf('s-constat') + 1;
  await p.click(`.thumb >> nth=${n - 1}`);
  await stageReady(p, n);
  const at = (await view(p)).targets.findIndex((t) => t.id === 'f');
  assert.ok(at >= 0, 'la frise est une cible de la scène');
  const target = p.locator('.overlay .target').nth(at);
  const selectFrise = async () => {
    await target.click({ position: { x: 4, y: 4 } });
    await p.waitForSelector('.overlay .target.selected');
  };

  // 1. Sélection : carte « Paramètres de … » et une ligne par étape.
  await selectFrise();
  const panel = p.locator(PANEL);
  await panel.waitFor({ timeout: 15000 });
  // Le libellé exact du widget visé : la carte de la slide (« Paramètres de Chapitre »…) peut
  // s'afficher un instant avant celle de la frise sélectionnée.
  await until(p, async () => (await panel.innerText().catch(() => '')).includes('Paramètres de Frise chronologique'), 'titre « Paramètres de Frise chronologique »');
  const rows = panel.locator('.lt tbody tr');
  await until(p, async () => (await rows.count()) === original.length, 'une ligne par étape');

  // 2. Date de la première ligne : l'item du deck porte la nouvelle date.
  const first = original[0];
  const firstDate = panel.locator(`tr[data-row="${first}"] td[data-col="date"] input`);
  await firstDate.waitFor({ timeout: 15000 });
  await firstDate.fill('2031');
  await selectFrise(); // le focus quitte le tableau : saisie validée
  const textOf = async (id) => (await view(p)).fields.find((f) => f.field === `etapes/${id}/date`)?.text;
  await until(p, async () => (await textOf(first)) === '2031', 'nouvelle date relue dans le deck');

  // 3. Ajout d'une ligne : ids + 1 ; clic sur la scène puis Cmd/Ctrl+Z : exactement les ids d'origine
  //    (l'annulation rend aussi la date d'origine, c'est un pas de plus).
  const before = await ids(p);
  await panel.locator('[data-list-add]').click();
  await waitLen(p, before.length + 1, 'une étape de plus après ajout');
  await selectFrise();
  await p.keyboard.press(`${MOD}+z`);
  await waitLen(p, before.length, 'ajout annulé');
  assert.deepEqual(await ids(p), before, "ids d'origine après annulation");
  assert.deepEqual(await ids(p), original, "mêmes ids qu'au départ");

  // 4. Fond de la slide s-ch1 : la carte du widget de base de la slide, sous ses noms traduits
  //    (« Chapitre », champ « Titre »), et le titre du deck.
  const ch = order.indexOf('s-ch1') + 1;
  await p.click(`.thumb >> nth=${ch - 1}`);
  await stageReady(p, ch);
  await until(p, async () => (await p.locator('.overlay .target.selected').count()) === 0, 'plus de sélection');
  await panel.waitFor({ timeout: 15000 });
  await until(p, async () => (await panel.innerText().catch(() => '')).includes('Paramètres de Chapitre'), `carte du widget de la slide : ${await panel.innerText().catch(() => '')}`);
  const title = panel.getByRole('textbox', { name: 'Titre', exact: true });
  await until(p, async () => (await title.inputValue({ timeout: 1000 }).catch(() => null)) === 'Contexte', 'champ titre = titre du deck');

  assert.deepEqual(errors, []);
  console.log('target-panel : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
