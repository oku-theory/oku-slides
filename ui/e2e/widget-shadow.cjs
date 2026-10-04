// Aperçu d'un widget : centré quand le code est masqué, son ombre tient dans le panneau qui défile.
// Sans serveur : la feuille de styles réelle sur un sous-onglet de widget reconstitué.
// Lancer : bun e2e/widget-shadow.cjs.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');

const css = fs.readFileSync(path.join(__dirname, '..', 'src', 'styles.css'), 'utf8');
const svg = encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80"><rect width="300" height="80" fill="white"/></svg>');
const html = `<!doctype html><style>${css}</style>
<div style="height:600px;width:800px;display:flex;flex-direction:column">
  <section class="widget-tab" data-code="folded">
    <aside class="widget-tab-side"><img src="data:image/svg+xml,${svg}"><ul class="widget-tab-diagnostics"></ul></aside>
    <div class="widget-tab-code" inert></div>
  </section>
</div>`;

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1000, height: 800 } });
  await p.setContent(html);
  await p.waitForFunction(() => document.querySelector('img').complete);
  const r = await p.evaluate(() => {
    const img = document.querySelector('.widget-tab-side img');
    const side = document.querySelector('.widget-tab-side');
    // Étendue de l'ombre de chaque côté : flou + étalement, décalés.
    const ext = { top: 0, right: 0, bottom: 0, left: 0 };
    for (const layer of getComputedStyle(img).boxShadow.split(/,(?![^(]*\))/)) {
      const [x, y, blur = 0, spread = 0] = (layer.match(/-?[\d.]+px/g) ?? []).map(parseFloat);
      const reach = blur + spread;
      ext.top = Math.max(ext.top, reach - y);
      ext.bottom = Math.max(ext.bottom, reach + y);
      ext.left = Math.max(ext.left, reach - x);
      ext.right = Math.max(ext.right, reach + x);
    }
    const i = img.getBoundingClientRect();
    const s = side.getBoundingClientRect();
    // La boîte de remplissage du panneau, à son défilement près.
    const box = {
      top: s.top + side.clientTop - side.scrollTop,
      left: s.left + side.clientLeft,
      right: s.left + side.clientLeft + side.clientWidth,
      bottom: s.top + side.clientTop - side.scrollTop + side.scrollHeight,
    };
    return { ext, i: { top: i.top, left: i.left, right: i.right, bottom: i.bottom }, box, side: { top: s.top, left: s.left, right: s.right, bottom: s.bottom } };
  });
  await b.close();
  const { ext, i, box, side } = r;
  // Code masqué : l'aperçu est au centre de sa rangée.
  const dx = (i.left + i.right) / 2 - (side.left + side.right) / 2;
  const dy = (i.top + i.bottom) / 2 - (side.top + side.bottom) / 2;
  assert.ok(Math.abs(dx) <= 1, `aperçu décentré horizontalement de ${dx}px`);
  assert.ok(Math.abs(dy) <= 1, `aperçu décentré verticalement de ${dy}px`);
  assert.ok(ext.bottom > 0, `aperçu sans ombre : ${JSON.stringify(ext)}`);
  assert.ok(i.top - ext.top >= box.top - 0.5, `ombre rognée en haut (${i.top - ext.top} < ${box.top})`);
  assert.ok(i.bottom + ext.bottom <= box.bottom + 0.5, `ombre rognée en bas (${i.bottom + ext.bottom} > ${box.bottom})`);
  assert.ok(i.left - ext.left >= box.left - 0.5, `ombre rognée à gauche (${i.left - ext.left} < ${box.left})`);
  assert.ok(i.right + ext.right <= box.right + 0.5, `ombre rognée à droite (${i.right + ext.right} > ${box.right})`);
  console.log('widget-shadow ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
