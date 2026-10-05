// Frise (widget de pack des tests, fixtures/widgets/frise-test.typ) : ajouter une étape sur la
// slide, la saisir, la déplacer (clavier et glisser ⋮⋮), la supprimer puis annuler.
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e
// (avant widgets.cjs : l'onglet est encore en mode deck ; la frise reste sur s-constat).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const FRISE = fs.readFileSync(path.join(__dirname, '../../fixtures/widgets/frise-test.typ'), 'utf8');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const MOD = process.platform === 'darwin' ? 'Meta' : 'Control';

const post = (p, endpoint, req) =>
  p.evaluate(
    ([e, r]) => fetch(e, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(r) }).then((x) => x.json()),
    [endpoint, req],
  );
const view = (p) => post(p, '/api', { tab: 't-1', req: { cmd: 'slide', id: 's-constat' } });
const items = async (p) => (await view(p)).items.filter((i) => i.owner === 'f');
const ids = async (p) => (await items(p)).map((i) => i.item);
/** Rejoue `fn` jusqu'à une valeur vraie ; échoue avec `why` après `timeout` ms. */
const until = async (p, fn, why, timeout = 8000) => {
  const t0 = Date.now();
  while (Date.now() - t0 < timeout) {
    if (await fn()) return;
    await p.waitForTimeout(100);
  }
  assert.fail(why);
};
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
      ops: [{ op: 'add_element', slot: { owner: { slide: 's-constat' }, param: 'blocks' }, element: { id: 'f', kind: 'frise' }, defaults: true }],
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
  // Pas d'item sélectionné : les « + » sont ceux de l'élément sélectionné (quatre items → cinq points d'insertion).
  assert.equal(await p.locator('[data-item-insert]').count(), 5, "cinq « + » autour de quatre étapes");

  // « + » après la dernière étape : cinq items, le champ date du nouvel item est en édition.
  await p.locator('[data-item-insert]').last().click();
  await waitLen(p, 5, 'cinq étapes après insertion');
  await p.waitForSelector('.editor .cm-editor.cm-focused');
  const added = (await ids(p)).find((i) => !['d0', 'd1', 'd2', 'd3'].includes(i));
  assert.ok(added, 'nouvel id');
  assert.equal((await ids(p))[4], added, 'ajouté en dernier');

  // Saisie : « 2027 », Cmd/Ctrl+Entrée valide ; set_field relu dans la vue.
  await p.keyboard.type('2027');
  await p.keyboard.press(`${MOD}+Enter`);
  await p.waitForSelector('.editor', { state: 'detached' });
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

  // Glisser ⋮⋮ avec de vrais événements pointeur : du nouvel item vers la première place.
  await target.click({ position: { x: 4, y: 4 } });
  await p.waitForSelector('.overlay .target.selected');
  // La scène a fini de se redessiner (aperçu rechargé) avant de viser la poignée.
  await p.waitForFunction(() => {
    const img = document.querySelector('.stage img');
    return img?.complete && img.naturalWidth > 0;
  });
  // Les poignées sont posées : cinq items, et leurs boîtes ne bougent plus d'une image à la suivante.
  await p.waitForFunction(() => document.querySelectorAll('.items .item-handles').length === 5);
  await p.waitForFunction(() => {
    const now = [...document.querySelectorAll('.items .item-handles')].map((h) => JSON.stringify(h.getBoundingClientRect())).join('|');
    const same = window.__handles === now;
    window.__handles = now;
    return same;
  }, null, { polling: 'raf' });
  const list = await items(p);
  const me = list.find((i) => i.item === added);
  const handles = p.locator('.items .item-handles');
  const firstBox = await handles.nth(list[0].index).boundingBox();
  // Survol de l'item : ses poignées apparaissent.
  const box = await handles.nth(me.index).boundingBox();
  await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  const grip = handles.nth(me.index).locator('[data-item-move]');
  await grip.waitFor({ state: 'visible' });
  const g = await grip.boundingBox();
  const gx = g.x + g.width / 2;
  const gy = g.y + g.height / 2;
  await p.mouse.move(gx, gy);
  const under = await p.evaluate(([x, y]) => document.elementFromPoint(x, y)?.outerHTML.slice(0, 160), [gx, gy]);
  assert.ok(under?.includes('data-item-move'), `la poignée ⋮⋮ est sous le pointeur : ${under}`);
  await p.mouse.down();
  // On passe au-delà du centre de chaque item voisin, vers la gauche, par petits pas.
  const to = firstBox.x + firstBox.width * 0.1;
  const steps = 20;
  for (let i = 1; i <= steps; i++) await p.mouse.move(gx + ((to - gx) * i) / steps, gy);
  await p.waitForSelector('.item-drop', { timeout: 3000 }); // repère de dépôt pendant le glisser
  await p.mouse.up();
  await until(p, async () => (await ids(p))[0] === added, 'glissé en première place');
  assert.equal((await ids(p)).length, 5);

  // « × » sur le nouvel item : quatre items ; Cmd/Ctrl+Z : cinq.
  const mine = p.locator('.items .item-handles').nth(0);
  const mb = await mine.boundingBox();
  await p.mouse.move(mb.x + mb.width / 2, mb.y + mb.height / 2);
  await mine.locator('[data-item-remove]').waitFor({ state: 'visible' });
  await mine.locator('[data-item-remove]').click();
  await waitLen(p, 4, 'quatre étapes après suppression');
  assert.ok(!(await ids(p)).includes(added));
  await p.locator('body').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press(`${MOD}+z`);
  await waitLen(p, 5, 'cinq étapes après annulation');
  assert.equal((await ids(p))[0], added, "l'annulation rend l'étape à sa place");

  assert.deepEqual(errors, []);
  console.log('lists : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
