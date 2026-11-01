// A pack item's layout (t-1419), in Essentiel's pack tab: the item strip under the header, the preview
// under it, then the Settings and Code tabs; the Code tab opened is kept after a reload; in a narrow window
// nothing overflows.
// Writes: Essentiel (the `cadran` widget). Leaves: no pack tab. Contre `oku-slides serve fixtures/roadmap.json`. Lancer : just e2e.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { closePackTabs, openEssentiel } = require('./lib/atelier.cjs');
const { uiSaved } = require('./lib/uiSaved.cjs');
const { newPackItem } = require('./lib/widget.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TAB = '.widget-tab';

/** Boxes of the strip, the preview, the tab list and the open panel of the selected item. */
const boxes = (p) => p.evaluate((sel) => {
  const tab = document.querySelector(sel);
  const r = (el) => { const b = el.getBoundingClientRect(); return { left: b.left, right: b.right, top: b.top, bottom: b.bottom, width: b.width }; };
  const code = tab.querySelector('.widget-tab-code');
  return {
    strip: r(document.querySelector('.pack-strip')),
    preview: r(tab.querySelector('.widget-tab-preview')),
    tabs: r(tab.querySelector('.widget-tab-tabs')),
    panel: r(code.hasAttribute('inert') ? tab.querySelector('.widget-tab-side') : code),
    overflow: tab.scrollWidth - tab.clientWidth,
  };
}, TAB);

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1600, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await closePackTabs(p);
  await p.waitForSelector('.stage img');
  await openEssentiel(p);
  await newPackItem(p, 'cadran');
  await p.waitForSelector(`${TAB} .widget-tab-preview`);
  const tab = (name) => p.getByRole('tab', { name });

  // Settings tab: strip, preview, tabs, settings, from top to bottom, with room between the groups.
  assert.equal(await tab('Paramètres').getAttribute('aria-selected'), 'true', 'onglet Paramètres pas ouvert par défaut');
  let r = await boxes(p);
  assert.ok(r.preview.top >= r.strip.bottom + 8, `aperçu pas sous la bande : ${JSON.stringify(r)}`);
  assert.ok(r.tabs.top >= r.preview.bottom + 8, `onglets pas sous l'aperçu : ${JSON.stringify(r)}`);
  assert.ok(r.panel.top >= r.tabs.bottom, `réglages pas sous les onglets : ${JSON.stringify(r)}`);

  // Code tab: the code takes the panel, under the tabs; kept after a reload.
  const shown = uiSaved(p, (req) => req.code);
  await tab('Code source').click();
  await p.waitForSelector(`${TAB}[data-code="shown"]`);
  r = await boxes(p);
  assert.ok(r.panel.top >= r.tabs.bottom && r.panel.width > 200, `code pas sous les onglets : ${JSON.stringify(r)}`);
  assert.ok((await shown).ok(), "onglet Code non gardé par l'hôte");
  await p.reload();
  await p.waitForSelector(`${TAB}[data-code="shown"]`);
  assert.equal(await tab('Code source').getAttribute('aria-selected'), 'true', 'onglet Code non repris');

  // Narrow window: nothing overflows the item view.
  await p.setViewportSize({ width: 1100, height: 900 });
  r = await boxes(p);
  assert.ok(r.overflow <= 0, `la vue de l'élément déborde de ${r.overflow} px`);

  // Back to the Settings tab, for the following specs.
  const folded = uiSaved(p, (req) => !req.code);
  await tab('Paramètres').click();
  assert.ok((await folded).ok(), "onglet Paramètres non gardé par l'hôte");

  await closePackTabs(p);
  assert.deepEqual(errors, []);
  console.log('disposition de l\'élément ok');
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
