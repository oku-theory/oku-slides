// Aperçu d'un widget : centré quand le code est masqué, son ombre tient dans le panneau qui défile.
// Deux formes : l'image seule (aperçu sans champs) et la scène de l'aperçu (champs éditables),
// cadrée comme la scène le fait (au plus grand qui tient dans sa place).
// Sans serveur : la feuille de styles réelle sur un sous-onglet de widget reconstitué.
// Lancer : bun e2e/110-widget-shadow.cjs.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { APP_SHEETS, sheets } = require('./lib/css.cjs');

const css = sheets(...APP_SHEETS);
const svg = encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="300" height="80"><rect width="300" height="80" fill="white"/></svg>');
const img = `<img src="data:image/svg+xml,${svg}">`;
const page = (preview) => `<!doctype html><style>${css}</style>
<div style="height:600px;width:800px;display:flex;flex-direction:column">
  <section class="widget-tab" data-code="folded">
    <section class="widget-tab-preview">${preview}</section>
    <div class="widget-tab-side"><ul class="widget-tab-diagnostics"></ul></div>
    <div class="widget-tab-code" inert></div>
  </section>
</div>`;
const scene = `<div class="stage-wrap stage-preview" dir="ltr"><div class="stage-frame"><div class="stage">${img}</div></div></div>`;
const CASES = [
  { name: 'image', html: page(img), sheet: '.widget-tab-preview > img' },
  { name: 'scène', html: page(scene), sheet: '.stage-preview .stage' },
];

/** Comme la scène : la plus grande taille 300 × 80 qui tient dans sa place. */
const fitScene = () => {
  const wrap = document.querySelector('.stage-preview');
  if (!wrap) return;
  const s = Math.min(wrap.clientWidth / 300, wrap.clientHeight / 80);
  for (const el of [wrap.querySelector('.stage-frame'), wrap.querySelector('.stage')]) {
    el.style.width = `${Math.floor(300 * s)}px`;
    el.style.height = `${Math.floor(80 * s)}px`;
  }
};

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1000, height: 800 } });
  for (const c of CASES) {
    await p.setContent(c.html);
    await p.waitForFunction(() => document.querySelector('img').complete);
    await p.evaluate(fitScene);
    const r = await p.evaluate((sel) => {
      const img = document.querySelector(sel);
      const side = document.querySelector('.widget-tab-preview');
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
    }, c.sheet);
    const { ext, i, box, side } = r;
    const n = c.name;
    // Code masqué : l'aperçu est au centre de sa rangée.
    const dx = (i.left + i.right) / 2 - (side.left + side.right) / 2;
    const dy = (i.top + i.bottom) / 2 - (side.top + side.bottom) / 2;
    assert.ok(Math.abs(dx) <= 1, `${n} : aperçu décentré horizontalement de ${dx}px`);
    assert.ok(Math.abs(dy) <= 1, `${n} : aperçu décentré verticalement de ${dy}px`);
    // La scène prend la place qu'on lui laisse : jamais écrasée à rien.
    assert.ok(i.right - i.left >= 280, `${n} : aperçu trop étroit (${i.right - i.left}px)`);
    assert.ok(ext.bottom > 0, `${n} : aperçu sans ombre : ${JSON.stringify(ext)}`);
    assert.ok(i.top - ext.top >= box.top - 0.5, `${n} : ombre rognée en haut (${i.top - ext.top} < ${box.top})`);
    assert.ok(i.bottom + ext.bottom <= box.bottom + 0.5, `${n} : ombre rognée en bas (${i.bottom + ext.bottom} > ${box.bottom})`);
    assert.ok(i.left - ext.left >= box.left - 0.5, `${n} : ombre rognée à gauche (${i.left - ext.left} < ${box.left})`);
    assert.ok(i.right + ext.right <= box.right + 0.5, `${n} : ombre rognée à droite (${i.right + ext.right} > ${box.right})`);
  }
  await b.close();
  console.log('widget-shadow ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
