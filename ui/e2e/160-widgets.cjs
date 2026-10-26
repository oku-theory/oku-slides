// Réglages d'essai d'un widget au dock : changer un paramètre change le contenu de l'aperçu. Avant : l'étiquette
// d'un tableau du deck ouvre son pack (Base) dans un onglet de pack, sur le sous-onglet du widget. Après : fermer
// l'onglet de pack par sa croix propose de mettre le deck à jour ; accepté, `t-1` épingle la version offerte.
// Writes: Essentiel (the "Nouveau widget" `set_widget`); `t-1` takes the offered version, then one undo restores
// its older pin. Leaves: no pack tab; `t-1` pins an older Essentiel (its badge stays lit).
// Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openEssentiel } = require('./lib/atelier.cjs');
const { post } = require('./lib/http.cjs');
const { until, untilValue, settled } = require('./lib/until.cjs');
const { PREVIEW, widgetPreviewSvg } = require('./lib/widget.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const api = (p, tab, req) => post(p, '/api', { tab, req });
const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
/** The library's current version of `t-1`'s Essentiel, and the version `t-1` pins. */
const essentiel = async (p) => {
  const pin = (await api(p, 't-1', { cmd: 'snapshot' })).layouts.essentiel;
  const current = (await api(p, 't-1', { cmd: 'packs', kind: 'layouts' })).find((pack) => pack.id === pin.id);
  return { pinned: pin.version, current: current?.version };
};

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const dialogs = [];
  p.on('pageerror', (e) => errors.push(e.message));
  // The only dialog: the update offer naming `t-1`'s deck and Essentiel, accepted.
  p.on('dialog', (d) => {
    const offer = /^Mettre à jour « ⁨(.+)⁩ » avec cette version du pack « ⁨Essentiel⁩ » \?$/.exec(d.message());
    if (offer) {
      dialogs.push(`offer ${offer[1]}`);
      void d.accept();
    } else {
      errors.push(`dialogue inattendu : ${d.message()}`);
      void d.dismiss();
    }
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
  const base = added.widgets.base.id;
  const opened = await untilValue(p, () => view(p), (v) => v.windows[0].active !== 't-1', "l'étiquette n'ouvre pas d'onglet de pack");
  const tabs = opened.windows[0].tabs;
  const at = tabs.findIndex((t) => t.id === opened.windows[0].active);
  assert.equal(tabs[at - 1]?.id, 't-1', "l'onglet de pack ne s'ouvre pas juste après t-1");
  assert.deepEqual(tabs[at].document, { kind: 'pack', pack: base }, "l'onglet ouvert n'édite pas Base");
  await untilValue(p, async () => (await api(p, tabs[at].id, { cmd: 'snapshot' })).ui.widget, (w) => w === 'table', "le sous-onglet table n'est pas ouvert");
  await closePackTabs(p);
  await api(p, 't-1', { cmd: 'undo' });
  const constat = await api(p, 't-1', { cmd: 'slide', id: 's-constat' });
  assert.ok(!JSON.stringify(constat).includes('e2e-tag-open'), "l'annulation ne retire pas le tableau");

  // 1. Essentiel in its pack tab: a new widget, its trial settings change its preview.
  const packTab = await openEssentiel(p);
  assert.equal(await p.locator('.tab.active .tab-badge').innerText(), 'Pack', "l'onglet de pack n'a pas son badge");
  await p.getByRole('button', { name: 'Nouveau widget' }).click();
  const name = p.getByRole('textbox', { name: /Nom du widget/ });
  await name.fill('carte');
  await name.press('Enter');
  await p.waitForSelector(PREVIEW);
  // Les réglages d'essai sont chargés (leur surface est là), puis l'aperçu se stabilise (rendu initial,
  // puis valeurs d'essai) avant la mesure : relevés à 400 ms, plus que l'écart entre ces deux rendus.
  await p.waitForSelector('[data-surface-id="host-widget"] input');
  const before = await settled(p, () => widgetPreviewSvg(p), "l'aperçu ne se stabilise pas", 10000, 400);
  // Sous l'aperçu du widget, pas au dock de l'agent.
  assert.equal(await p.locator('.widget-tab:not([hidden]) .widget-tab-side [data-surface-id="host-widget"]').count(), 1);
  assert.equal(await p.locator('.notes [data-surface-id="host-widget"]').count(), 0);
  const field = p.locator('[data-surface-id="host-widget"] input').first();
  await field.fill('Météo du jour');
  await field.press('Enter');
  await until(p, async () => {
    const now = await widgetPreviewSvg(p);
    return now !== null && now !== before;
  }, "l'aperçu ne change pas après le réglage");
  assert.equal(await p.locator('[data-surface-id="host-widget"] input').first().inputValue(), 'Météo du jour');

  // 2. The widget write made a newer Essentiel: `t-1` pins an older one. The × offers it, accepted.
  const older = await untilValue(p, () => essentiel(p), (e) => !!e.current && e.current !== e.pinned, "t-1 épingle déjà l'Essentiel courant");
  await p.locator('.tab.active .tab-close').click();
  await untilValue(p, () => view(p), (v) => !v.windows[0].tabs.some((t) => t.id === packTab) && v.windows[0].active === 't-1', "l'onglet de pack ne se ferme pas");
  const title = (await view(p)).windows[0].tabs.find((t) => t.id === 't-1').title;
  await untilValue(p, () => essentiel(p), (e) => e.pinned === e.current && e.pinned !== older.pinned, "t-1 n'épingle pas la version offerte");
  assert.deepEqual(dialogs, [`offer ${title}`]);
  // One undo gives `t-1` its older pin back.
  await api(p, 't-1', { cmd: 'undo' });
  assert.equal((await essentiel(p)).pinned, older.pinned, "l'annulation ne rend pas l'Essentiel d'avant");
  assert.deepEqual(errors, []);
  console.log('réglages du widget ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
