// The pack editor's layout (t-1392, t-1419): in the Base widget pack, `card` selected, the preview slot set
// to « Moitié » narrows the preview and is kept after a reload; the form sits under the preview, in the
// Settings tab, with « Mise en avant » as a switch and a localized label; the strip shows a named thumbnail
// per widget, the widget alone cropped as its preview, above the preview; the header keeps the brand on one
// line and room for the title.
// Writes: the UI state only (the slot is reset to full at the end). Leaves: no pack tab.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const { closePackTabs, openPackTab, packTabShown } = require('./lib/atelier.cjs');
const { untilValue } = require('./lib/until.cjs');
const { uiSaved } = require('./lib/uiSaved.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = '.widget-tab';
const PREVIEW = `${TAB} .widget-tab-preview img`;

// The rendered slot's width: the preview image's own pixels (the displayed box is fitted to the pane).
const width = (p) => p.locator(PREVIEW).first().evaluate((img) => img.naturalWidth);
const box = (p, sel) => p.locator(sel).first().boundingBox();

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');
  const slot = () => p.getByRole('combobox', { name: 'Case de l’aperçu' });
  try {
    const baseId = (await post(p, '/api', { tab: 't-1', req: { cmd: 'snapshot' } })).widgets.base.id;
    await openPackTab(p, 't-1', baseId, 'card');
    await p.reload();
    await packTabShown(p);
    await p.waitForSelector(PREVIEW);

    assert.equal(await p.locator('.pack-strip .pile.horizontal').count(), 1, "la bande n'est pas horizontale");
    assert.ok(await p.locator('.pack-strip .thumb img').count() >= 1, 'la bande ne montre pas de miniatures');
    assert.equal(await p.locator('.pack-strip .thumb[aria-current="true"] .thumb-name').textContent(), 'card', 'card pas nommé sous sa miniature');
    // The thumbnail is the widget alone, cropped as its full-slot preview: not its whole slide.
    const natural = (sel) => p.locator(sel).first().evaluate((img) => (img.naturalHeight ? img.naturalWidth / img.naturalHeight : 0));
    const thumbRatio = await untilValue(p, () => natural('.pack-strip .thumb[aria-current="true"] img'), (r) => r > 0, 'miniature de card sans image');
    const previewRatio = await untilValue(p, () => natural(PREVIEW), (r) => r > 0, 'aperçu de card sans image');
    assert.ok(Math.abs(thumbRatio - previewRatio) < 0.01, `miniature ${thumbRatio}, aperçu ${previewRatio} : pas le même cadrage`);
    // The settings form arrives after the preview image: wait for it before counting.
    await p.locator(`${TAB} .host-settings`).getByRole('switch', { name: 'Mise en avant' }).waitFor({ timeout: 10000 });
    assert.equal(await p.locator(`${TAB} .host-settings`).getByRole('switch', { name: 'Mise en avant' }).count(), 1, 'pas de bascule « Mise en avant »');
    assert.ok(await p.locator(`${TAB} .host-settings`).getByText('Titre', { exact: true }).count() >= 1, 'libellé « Titre » absent');
    assert.ok(await p.locator(`${TAB} .host-settings`).getByText('Passe la carte sur le fond sombre du thème').count() >= 1, "aide de « Mise en avant » absente");

    // The header at 1600×900 with the agent column: the brand stays on one line, the title keeps room.
    const brandLines = await p.locator('.label.editing-pack .brand').evaluate((el) => {
      const line = parseFloat(getComputedStyle(el).lineHeight);
      return el.getBoundingClientRect().height / line;
    });
    assert.ok(brandLines < 1.5, `la marque passe sur plusieurs lignes (${brandLines} lignes)`);
    const title = await box(p, '.label.editing-pack .doc-name');
    assert.ok(title.width >= 80, `le titre n'a pas la place (${title.width} px)`);

    // The strip above the preview, the form under it.
    const strip = await box(p, '.pack-strip');
    const pv = await box(p, `${TAB} .widget-tab-preview`);
    const side = await box(p, `${TAB} .widget-tab-side`);
    assert.ok(pv.y >= strip.y + strip.height - 1, `l'aperçu n'est pas sous la bande (${pv.y} < ${strip.y + strip.height})`);
    assert.ok(side.y >= pv.y + pv.height - 1, `le formulaire n'est pas sous l'aperçu (${side.y} < ${pv.y + pv.height})`);

    const full = await untilValue(p, () => width(p), (w) => w > 0, 'aperçu sans image');
    const halfSaved = uiSaved(p, (req) => req.preview_span === 'half');
    await slot().selectOption('half');
    await halfSaved;
    await untilValue(p, () => width(p), (w) => w < full * 0.8, "l'aperçu ne rétrécit pas en case ½");

    await p.reload();
    await p.waitForSelector(`${TAB} .widget-tab-preview`);
    assert.equal(await slot().inputValue(), 'half', 'case non gardée');
  } finally {
    // Later specs expect the default slot; wait for the save before closing.
    if (await slot().count() && (await slot().inputValue()) !== 'full') {
      const fullSaved = uiSaved(p, (req) => req.preview_span === 'full');
      await slot().selectOption('full');
      await fullSaved;
    }
    await closePackTabs(p);
  }
  assert.deepEqual(errors, []);
  console.log('éditeur de pack : disposition et case ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
