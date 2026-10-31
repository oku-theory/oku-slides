// Réglages d'essai d'un widget au dock : changer un paramètre change le contenu de l'aperçu. Avant : l'étiquette
// d'un tableau du deck ouvre son pack (Base) dans un onglet de pack, sur ce widget. Then "Ouvrir" on Base's row of
// "Packs du deck" opens Base in a pack tab, and again only brings that tab back; in it a new widget's trial settings
// change its preview, F2 renames an item, its trash button deletes one through the confirmation (Enter does
// nothing). Après : fermer l'onglet de pack par sa croix propose de mettre le deck à jour ; accepté, `t-1` épingle
// la version que l'onglet a écrite, marquée adoptée dans la bibliothèque.
// Writes: Base (the `carte` widget, a session version); `t-1` takes the offered version, then one undo restores
// its older pin. Leaves: no pack tab, an empty dock; `t-1` pins an older Essentiel and an older Base (badge lit).
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { closePackTabs, closePacksPanel, packTabShown } = require('./lib/atelier.cjs');
const { post } = require('./lib/http.cjs');
const { answerQuestion } = require('./lib/question.cjs');
const { until, untilValue, settled } = require('./lib/until.cjs');
const { PREVIEW, newPackItem, pileThumb, selectPackItem, widgetPreviewSvg } = require('./lib/widget.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const api = (p, tab, req) => post(p, '/api', { tab, req });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
/** The library's current version of `t-1`'s Base, and the version `t-1` pins. */
const base = async (p) => {
  const pin = (await api(p, 't-1', { cmd: 'snapshot' })).widgets.base;
  const current = (await api(p, 't-1', { cmd: 'packs', kind: 'widgets' })).find((pack) => pack.id === pin.id);
  return { pinned: pin.version, current: current?.version };
};
/** The active tab of the first window. */
const active = async (p) => (await view(p)).windows[0].active;
/**
 * Whether version `version` (`sha256:<hex>`) of pack `id` carries the library's `adopted` mark: the file
 * `<id>/versions/<hex>/adopted`, looked for under the server's data dir (`just e2e` exports it).
 */
const adopted = (id, version) => {
  const data = process.env.OKU_SLIDES_PATHS__DATA;
  assert.ok(data, 'OKU_SLIDES_PATHS__DATA manquant : lancer par just e2e');
  const want = path.join(id, 'versions', version.replace(/^sha256:/, ''), 'adopted');
  return fs.readdirSync(data, { recursive: true }).some((f) => String(f).endsWith(want));
};

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const dialogs = [];
  p.on('pageerror', (e) => errors.push(e.message));
  // No browser dialog; the only question is the in-page update offer naming `t-1`'s deck and Base.
  p.on('dialog', (d) => {
    errors.push(`dialogue inattendu : ${d.message()}`);
    void d.dismiss();
  });
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');

  // 0. A table of the deck: its tag opens Base in a pack tab right after `t-1`, on the `table` sub-tab.
  const added = await api(p, 't-1', {
    cmd: 'apply', label: 'tableau', merge: null,
    ops: [{ op: 'add_element', slot: { owner: { slide: 's-constat' }, param: 'blocks' }, element: { id: 'e2e-tag-open', kind: 'base/table', params: { data: { dataset: { source: [['A'], ['x']] } } } }, defaults: true }],
  });
  assert.ok(!added.error, JSON.stringify(added).slice(0, 300));
  // Serve pushes nothing: the page rereads the deck.
  await p.reload();
  await p.waitForSelector('.stage img');
  await p.click(`.thumb >> nth=${added.slides.findIndex((s) => s.id === 's-constat')}`);
  await p.click('.target[aria-label="Tableau"]');
  await p.locator('.target.selected .tag-open').click();
  const baseId = added.widgets.base.id;
  const opened = await untilValue(p, () => view(p), (v) => v.windows[0].active !== 't-1', "l'étiquette n'ouvre pas d'onglet de pack");
  const tabs = opened.windows[0].tabs;
  const at = tabs.findIndex((t) => t.id === opened.windows[0].active);
  assert.equal(tabs[at - 1]?.id, 't-1', "l'onglet de pack ne s'ouvre pas juste après t-1");
  assert.deepEqual(tabs[at].document, { kind: 'pack', pack: baseId }, "l'onglet ouvert n'édite pas Base");
  await untilValue(p, async () => (await api(p, tabs[at].id, { cmd: 'snapshot' })).ui.widget, (w) => w === 'table', "le sous-onglet table n'est pas ouvert");
  await closePackTabs(p);
  await api(p, 't-1', { cmd: 'undo' });
  const constat = await api(p, 't-1', { cmd: 'slide', id: 's-constat' });
  assert.ok(!JSON.stringify(constat).includes('e2e-tag-open'), "l'annulation ne retire pas le tableau");

  // 1. "Ouvrir" on Base's row of "Packs du deck" (from the toolbar): Base in a pack tab, right after `t-1`.
  const tabCount = async () => (await view(p)).windows[0].tabs.length;
  const count = await tabCount();
  await p.getByRole('button', { name: /^Packs du deck/ }).click();
  const openBase = p.locator('.agent-dock [data-surface-id="host-packs"] [data-component-id="widgets/base/open"]').getByRole('button', { name: 'Ouvrir', exact: true });
  await openBase.click();
  const packTab = await untilValue(p, () => active(p), (id) => id !== 't-1', "« Ouvrir » n'ouvre pas d'onglet de pack");
  const doc = (await view(p)).windows[0].tabs.find((t) => t.id === packTab).document;
  assert.deepEqual(doc, { kind: 'pack', pack: baseId }, "« Ouvrir » n'ouvre pas Base");
  assert.equal(await tabCount(), count + 1);
  // Back on `t-1` (its dock keeps the panel), "Ouvrir" again brings the same tab back: no new tab.
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await openBase.click();
  await untilValue(p, () => active(p), (id) => id === packTab, "« Ouvrir » une seconde fois ne revient pas à l'onglet de Base");
  assert.equal(await tabCount(), count + 1, '« Ouvrir » une seconde fois ouvre un autre onglet');
  await p.reload();
  await packTabShown(p);

  // 2. A new widget in Base: its trial settings change its preview.
  await newPackItem(p, 'carte');
  await p.waitForSelector(PREVIEW);
  // Les réglages d'essai sont chargés (leur surface est là), puis l'aperçu se stabilise (rendu initial,
  // puis valeurs d'essai) avant la mesure : relevés à 400 ms, plus que l'écart entre ces deux rendus.
  await p.waitForSelector('[data-surface-id="host-widget"] input');
  const before = await settled(p, () => widgetPreviewSvg(p), "l'aperçu ne se stabilise pas", 10000, 400);
  // Sous l'aperçu du widget, pas au dock de l'agent.
  assert.equal(await p.locator('.widget-tab .widget-tab-side [data-surface-id="host-widget"]').count(), 1);
  assert.equal(await p.locator('.notes [data-surface-id="host-widget"]').count(), 0);
  const field = p.locator('[data-surface-id="host-widget"] input').first();
  await field.fill('Météo du jour');
  await field.press('Enter');
  await until(p, async () => {
    const now = await widgetPreviewSvg(p);
    return now !== null && now !== before;
  }, "l'aperçu ne change pas après le réglage");
  assert.equal(await p.locator('[data-surface-id="host-widget"] input').first().inputValue(), 'Météo du jour');

  // 3. F2 on a thumbnail renames its item: `brouillon` becomes `essai`.
  await newPackItem(p, 'brouillon');
  await pileThumb(p, 'brouillon').focus();
  await p.keyboard.press('F2');
  const rename = p.getByRole('textbox', { name: /^Nom du widget/ });
  assert.equal(await rename.inputValue(), 'brouillon');
  await rename.fill('essai');
  await rename.press('Enter');
  await pileThumb(p, 'essai').waitFor();
  await pileThumb(p, 'brouillon').waitFor({ state: 'detached' });

  // 4. Its trash button asks in the dialog: Enter does nothing, the button (armed after 500 ms) deletes it.
  await pileThumb(p, 'essai').locator('xpath=..').locator('.thumb-remove').click();
  const confirm = p.getByRole('alertdialog');
  assert.match(await confirm.locator('.confirm-message').innerText(), /^Supprimer le widget « \u2068?essai\u2069? » \?$/);
  await p.keyboard.press('Enter');
  await p.waitForSelector('.confirm-dialog[data-armed]');
  await p.keyboard.press('Enter');
  // négatif : une suppression fautive par Entrée aurait le temps d'arriver.
  await p.waitForTimeout(300);
  assert.equal(await confirm.count(), 1, 'Entrée ferme la confirmation');
  assert.equal(await pileThumb(p, 'essai').count(), 1, 'Entrée supprime le widget');
  await confirm.getByRole('button', { name: 'Supprimer', exact: true }).click();
  await confirm.waitFor({ state: 'detached' });
  await pileThumb(p, 'essai').waitFor({ state: 'detached' });
  await selectPackItem(p, 'carte');

  // 5. The writes made a session version of Base, now the library's current one: `t-1` pins an older one.
  // The × offers it, accepted: `t-1` pins the version the tab wrote, and the library marks it adopted.
  const older = await untilValue(p, () => base(p), (e) => !!e.current && e.current !== e.pinned, "t-1 épingle déjà la Base courante");
  const written = await settled(p, async () => (await api(p, packTab, { cmd: 'snapshot' })).editing?.session_version ?? null, "la version de l'onglet ne se stabilise pas", 15000, 1000);
  assert.ok(written, "l'onglet de Base n'a pas écrit de version");
  await p.locator('.tab.active .tab-close').click();
  const offer = await answerQuestion(p, /^Mettre à jour « ⁨(.+)⁩ » avec cette version du pack « ⁨Base⁩ » \?$/, 'Mettre à jour');
  dialogs.push(`offer ${offer[1]}`);
  await untilValue(p, () => view(p), (v) => !v.windows[0].tabs.some((t) => t.id === packTab) && v.windows[0].active === 't-1', "l'onglet de pack ne se ferme pas");
  const title = (await view(p)).windows[0].tabs.find((t) => t.id === 't-1').title;
  await untilValue(p, () => base(p), (e) => e.pinned === e.current && e.pinned !== older.pinned, "t-1 n'épingle pas la version offerte");
  assert.equal((await base(p)).pinned, written, "t-1 n'épingle pas la version écrite par l'onglet");
  assert.ok(adopted(baseId, written), `la version ${written} de Base n'est pas marquée adoptée`);
  assert.deepEqual(dialogs, [`offer ${title}`]);
  // One undo gives `t-1` its older pin back.
  await api(p, 't-1', { cmd: 'undo' });
  assert.equal((await base(p)).pinned, older.pinned, "l'annulation ne rend pas la Base d'avant");
  // The packs panel opened in step 1 stays in `t-1`'s dock: later specs expect an empty one.
  await p.reload();
  await closePacksPanel(p);
  assert.deepEqual(errors, []);
  console.log('réglages du widget ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
