// Column divider grips: a small pill visible at rest says an area resizes.
// No server: the real style sheet on rebuilt dividers. Lancer : bun e2e/130-handles.cjs.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { APP_SHEETS, sheets } = require('./lib/css.cjs');

const css = sheets(...APP_SHEETS);
const html = `<!doctype html><style>${css}</style>
<div style="display:grid;grid-template-columns:200px 8px 200px;height:300px">
  <div></div><div class="col-divider" data-side="pile" style="grid-row:1;grid-column:2"></div><div></div>
</div>`;

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 800, height: 900 } });
  await p.setContent(html);
  const grips = await p.evaluate(() =>
    [...document.querySelectorAll('.col-divider')].map((el) => {
      const g = getComputedStyle(el, '::after');
      return { el: `${el.className} ${el.dataset.side ?? ''}`, w: parseFloat(g.width), h: parseFloat(g.height), opacity: parseFloat(g.opacity), content: g.content };
    }),
  );
  await b.close();
  for (const g of grips) {
    assert.notEqual(g.content, 'none', `${g.el} : pas de poignée`);
    assert.ok(g.opacity > 0 && g.opacity < 0.5, `${g.el} : poignée invisible ou trop appuyée au repos (${g.opacity})`);
    // Une pastille : longue dans le sens du séparateur (vertical), fine en travers.
    assert.ok(g.h >= 20 && g.h <= 48 && g.w >= 3 && g.w <= 5, `${g.el} : pastille ${g.w}×${g.h}`);
  }
  console.log('handles ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
