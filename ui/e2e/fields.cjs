// Alignement : l'éditeur ouvert recouvre exactement le texte rendu (bord gauche,
// ligne de base de chaque ligne à 1 px près à l'échelle 1, mêmes retours à la ligne).
// Lancer : just e2e. À la main : bun e2e/fields.cjs, E2E_URL = adresse d'un serveur neuf.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const TOL = 1;

/**
 * Lignes du texte rendu dans une boîte (px page), lues dans le SVG de la page.
 * Typst pose chaque glyphe par un `<use x=…>` dans un groupe dont l'origine est
 * sur la ligne de base : le point (x, 0) du groupe est l'origine du glyphe (plume),
 * comme le bord d'un rectangle de texte du DOM. Bord gauche = plume du premier
 * glyphe de la ligne ; ligne de base = y de ce point.
 */
const renderedLines = (p, box) => p.evaluate(async ({ box }) => {
  const img = document.querySelector('.stage img');
  const at = img.getBoundingClientRect();
  const host = document.createElement('div');
  host.style.cssText = `position:fixed;left:${at.left}px;top:${at.top}px;width:${at.width}px;height:${at.height}px;opacity:0;pointer-events:none`;
  host.innerHTML = await fetch(img.src).then((r) => r.text());
  const svg = host.querySelector('svg');
  svg.setAttribute('width', at.width);
  svg.setAttribute('height', at.height);
  document.body.append(host);
  // Glyphes du champ : dans sa largeur, centre vertical dans sa hauteur (la boîte de
  // Typst va de la hauteur de capitale à la ligne de base : accents, ascendantes et
  // jambages en débordent).
  const pens = [];
  for (const g of svg.querySelectorAll('use')) {
    const r = g.getBoundingClientRect();
    const c = (r.top + r.bottom) / 2;
    if (r.width === 0 || r.left < box.left - 2 || r.right > box.right + 2 || c < box.top || c > box.bottom) continue;
    const pen = new DOMPoint(Number(g.getAttribute('x') ?? 0), Number(g.getAttribute('y') ?? 0))
      .matrixTransform(g.parentElement.getScreenCTM());
    pens.push({ x: pen.x, y: pen.y });
  }
  host.remove();
  // Une ligne = une ligne de base (à un demi-pixel près).
  const lines = [];
  for (const pen of pens.sort((a, b) => a.y - b.y)) {
    const last = lines[lines.length - 1];
    if (last && Math.abs(pen.y - last.baseline) < 0.5) last.left = Math.min(last.left, pen.x);
    else lines.push({ left: pen.x, baseline: pen.y });
  }
  return lines;
}, { box });

/**
 * Lignes de l'éditeur : un rectangle par ligne visuelle (Range sur chaque
 * `.cm-line`). Pour du texte en ligne, le rectangle est la zone de contenu de
 * la police : ligne de base = haut + ascendante de la police (`fontBoundingBoxAscent`).
 */
const editorLines = (p) => p.evaluate(() => {
  const content = document.querySelector('.editor .cm-content');
  const cs = getComputedStyle(content);
  const ctx = document.createElement('canvas').getContext('2d');
  ctx.font = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
  const ascent = ctx.measureText('Hx').fontBoundingBoxAscent;
  const out = [];
  for (const line of content.querySelectorAll('.cm-line')) {
    const range = document.createRange();
    range.selectNodeContents(line);
    for (const r of range.getClientRects()) {
      if (r.width > 0) out.push({ left: r.left, baseline: r.top + ascent });
    }
  }
  return out.sort((a, b) => a.baseline - b.baseline);
});

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 2600, height: 1500 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  // Les scripts précédents laissent parfois un autre onglet actif : celui du fichier repasse devant.
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');
  // Les paramètres sous la slide prennent de la hauteur : la scène serait limitée par la
  // hauteur, jamais à l'échelle 1. Masqués le temps du test (choix gardé dans l'espace de
  // travail : réaffichés à la fin pour les scripts suivants).
  const params = (name) => p.getByRole('button', { name, exact: true });
  await params('Masquer les paramètres').click();
  await params('Afficher les paramètres').waitFor();
  // Échelle 1 : la slide (1920×1080 pt) affichée à 1 px par pt. La scène suit la largeur
  // de la fenêtre moins les colonnes voisines : on corrige la fenêtre de l'écart mesuré.
  const stageWidth = () => p.evaluate(() => document.querySelector('.stage').getBoundingClientRect().width);
  await p.setViewportSize({ width: 2600 + 1920 - (await stageWidth()), height: 1500 });
  await p
    .waitForFunction(() => Math.abs(document.querySelector('.stage').getBoundingClientRect().width - 1920) < 0.5)
    .catch(async () => {
      throw new Error(`échelle 1 non atteinte : scène de ${await stageWidth()} px de large au lieu de 1920`);
    });
  await p.evaluate(() => document.fonts.ready);
  const order = (await post(p, '/api', { tab: 't-1', req: { cmd: 'snapshot' } })).slides.map((s) => s.id);
  const show = async (slide) => {
    const n = order.indexOf(slide) + 1;
    if (process.env.E2E_DEBUG) console.log("slide", slide, n);
    await p.click(`.thumb >> nth=${n - 1}`);
    // La scène montre la page n (texte de remplacement « Slide n »), image chargée.
    await p.waitForFunction((n) => {
      const img = document.querySelector('.stage img');
      return img?.alt === `Slide ${n}` && img.complete && img.naturalWidth > 0;
    }, n);
    // Les champs de la scène sont ceux de la vue (même ordre) : on attend qu'ils y soient tous.
    const view = await post(p, '/api', { tab: 't-1', req: { cmd: 'slide', id: slide } });
    await p.waitForFunction((n) => document.querySelectorAll('.fields .field').length === n, view.fields.length);
    return view.fields;
  };
  const rows = [];
  const skipped = [];
  const failures = [];
  /** Compare les lignes du champ `i` de la slide courante, rendues puis éditées. */
  const check = async (slide, f, i) => {
    const name = `${slide}/${f.owner}.${f.field}`;
    if (f.empty || f.text === '') return skipped.push(`${name} : source vide${f.empty ? ' (placeholder)' : ' (texte par défaut du rendu)'}`);
    if (f.text.includes('{{')) return skipped.push(`${name} : variable (la source n'a pas la largeur du rendu)`);
    const field = p.locator('.fields .field').nth(i);
    const box = await field.boundingBox();
    const scale = await p.evaluate(() => document.querySelector('.stage').getBoundingClientRect().width / 1920);
    assert.ok(Math.abs(scale - 1) < 0.02, `échelle ${scale} : agrandir la fenêtre`);
    const rendered = await renderedLines(p, { left: box.x, top: box.y, right: box.x + box.width, bottom: box.y + box.height });
    await field.dblclick();
    await p.waitForSelector('.editor .cm-editor.cm-focused');
    const marks = await p.locator('.editor .cm-md-mark').count();
    const edited = marks ? [] : await editorLines(p);
    await p.keyboard.press('Escape');
    await p.waitForSelector('.editor', { state: 'detached' });
    if (marks) return skipped.push(`${name} : marques markdown affichées dans l'éditeur (${marks})`);
    if (process.env.E2E_DEBUG) console.log(name, { rendered, edited });
    const fail = (why) => failures.push(`${name} ${why}`);
    if (rendered.length === 0) fail(': aucun glyphe rendu dans le champ');
    if (edited.length !== rendered.length) fail(`: ${edited.length} lignes éditées, ${rendered.length} rendues`);
    edited.forEach((l, k) => {
      const r = rendered[k];
      if (!r) return;
      rows.push({ champ: name, ligne: k + 1, gauche: +(l.left - r.left).toFixed(2), base: +(l.baseline - r.baseline).toFixed(2) });
      if (Math.abs(l.left - r.left) > TOL) fail(`ligne ${k + 1} : bord gauche ${l.left} ≠ ${r.left}`);
      if (Math.abs(l.baseline - r.baseline) > TOL) fail(`ligne ${k + 1} : ligne de base ${l.baseline} ≠ ${r.baseline}`);
    });
  };
  // Tous les champs de toutes les slides du fichier.
  for (const slide of order) {
    const fields = await show(slide);
    for (let i = 0; i < fields.length; i++) await check(slide, fields[i], i);
  }
  // Plusieurs lignes, saisies dans l'éditeur (modifie le deck : serveur neuf) : le texte
  // libre de s-libre, long (retour à la ligne automatique) et en deux paragraphes ; le
  // titre de s-libre, assez long pour tenir sur deux lignes (interligne serré du titre).
  /** Réécrit le champ `field` de `owner` sur s-libre, puis compare ses lignes (au moins `min`). */
  const rewrite = async (owner, field, text, min) => {
    const fields = await show('s-libre');
    const at = fields.findIndex((f) => f.owner === owner && f.field === field);
    const src = await p.locator('.stage img').getAttribute('src');
    await p.locator('.fields .field').nth(at).dblclick();
    await p.waitForSelector('.editor .cm-editor.cm-focused');
    await p.keyboard.press('ControlOrMeta+a');
    const [first, ...rest] = text.split('\n');
    await p.keyboard.type(first);
    for (const line of rest) {
      await p.keyboard.press('Enter');
      await p.keyboard.type(line);
    }
    await p.keyboard.press('ControlOrMeta+Enter');
    await p.waitForSelector('.editor', { state: 'detached' });
    await p.waitForFunction((src) => {
      const img = document.querySelector('.stage img');
      return !!img && img.getAttribute('src') !== src && img.complete && img.naturalWidth > 0;
    }, src);
    const after = await show('s-libre');
    assert.equal(after[at].text, text);
    const before = rows.length;
    await check('s-libre', after[at], at);
    const n = new Set(rows.slice(before).map((r) => r.ligne)).size;
    assert.ok(n >= min, `s-libre/${owner}.${field} : ${n} lignes comparées, ${min} attendues au moins`);
  };
  await rewrite('e-texte', 'body', 'Un bloc posé à 40 pt du coin, assez long pour revenir à la ligne dans son cadre\nDeuxième paragraphe', 3);
  await rewrite('s-libre', 'title', 'Placement libre, avec un titre bien trop long pour une seule ligne de la slide, qui revient donc', 2);

  console.table(rows);
  for (const s of skipped) console.log(`ignoré : ${s}`);
  assert.deepEqual(failures, []);
  assert.deepEqual(errors, []);
  await params('Afficher les paramètres').click();
  await params('Masquer les paramètres').waitFor();
  await b.close();
  console.log('fields : ok');
})().catch((e) => { console.error(e); process.exit(1); });
