// Éditeur de texte : markdown mis en forme pendant la saisie, contre `oku-slides serve fixtures/roadmap.json`.
// Lancer : just e2e (serveur neuf, port 4872 ; ce script passe en premier). À la main : bun e2e/editor.cjs dans ui/,
// E2E_URL = adresse d'un serveur neuf (le script modifie la slide 4).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const slide = (p) => p.evaluate(() =>
  fetch('/api', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ tab: 't-1', req: { cmd: 'slide', id: 's-constat' } }) })
    .then((r) => r.json())
    .then((s) => s.slide));
const points = async (p) => (await slide(p)).params.blocks.find((e) => e.id === 'b-points').params.body;
const undoLabel = (p) => p.evaluate(() => document.querySelector('[aria-label^="Annuler"]')?.getAttribute('aria-label'));
const text = (p) => p.locator('.editor .cm-content').innerText();
const image = (p) => p.locator('.stage img').getAttribute('src');
/** Attend que `read()` vérifie `ok` (5 s au plus). */
const until = async (read, ok, what) => {
  for (let i = 0; i < 50; i++) {
    const v = await read();
    if (ok(v)) return v;
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`délai dépassé : ${what}`);
};
const open = async (p, label) => {
  await p.dblclick(`.field[aria-label="${label}"] >> nth=0`);
  await p.waitForSelector('.editor .cm-editor.cm-focused');
};
const closed = (p) => until(() => p.locator('.editor').count(), (n) => n === 0, 'éditeur fermé');
/** Valide par `act`, puis attend que la slide soit redessinée : l'interface a le nouveau texte. */
const commitWith = async (p, act) => {
  const before = await image(p);
  await act();
  await closed(p);
  await until(() => image(p), (src) => src !== before, 'slide redessinée');
};
/** Valide le titre par `act` : le titre devient `title`, l'éditeur passe au champ suivant ; Échap le ferme. */
const nextWith = async (p, act, title) => {
  await act();
  await until(async () => (await slide(p)).params.title, (t) => t === title, `titre « ${title} »`);
  await p.waitForSelector('.editor .cm-editor.cm-focused');
  assert.notEqual(await text(p), title); // un autre champ
  await p.keyboard.press('Escape');
  await closed(p);
};
/** Collage synthétique : `plain` en text/plain, `html` en text/html. */
const paste = (p, plain, html) => p.evaluate(([plain, html]) => {
  const dt = new DataTransfer();
  if (html) dt.setData('text/html', html);
  dt.setData('text/plain', plain);
  document.querySelector('.editor .cm-content')
    .dispatchEvent(new ClipboardEvent('paste', { clipboardData: dt, bubbles: true }));
}, [plain, html]);
const BASE = 'Dites **montrer** et ==ici=={2}';
const RUNS = [{ t: 'Dites ' }, { t: 'montrer', bold: true }, { t: ' et ' }, { t: 'ici', highlight: 2 }];
(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  await p.waitForSelector('.stage img');
  // Slide 4 : le bloc de puces.
  await p.click('.thumb >> nth=3');
  await p.waitForSelector('.field[aria-label="Corps"]');

  // 0. Sélectionner ne déplace pas la slide (la carte des paramètres change dessous) :
  // sinon le 2e clic d'un double-clic tombe à côté du champ.
  // Les flèches slide précédente / suivante restent à mi-hauteur de la slide.
  const stageBox = () => p.locator('.stage').boundingBox();
  const card = () => p.evaluate(() => [...document.querySelectorAll('.host-settings')].map((e) => e.innerText).join('\n'));
  const navOffset = () => p.evaluate(() => {
    const mid = (r) => (r.top + r.bottom) / 2;
    return mid(document.querySelector('.slide-nav.next').getBoundingClientRect())
      - mid(document.querySelector('.stage').getBoundingClientRect());
  });
  // La carte montre d'abord les paramètres de la slide (chargés après elle), puis ceux de l'élément
  // sélectionné, d'une autre hauteur : la mise en page a changé dessous.
  await until(card, (t) => t.includes('Paramètres de Contenu'), 'paramètres de la slide');
  const still = await stageBox();
  assert.ok(still, 'pas de slide');
  await p.click('.field[aria-label="Corps"] >> nth=0');
  await until(card, (t) => t.includes('Paramètres de Texte') && !t.includes('Paramètres de Contenu'), 'paramètres de l\'élément');
  assert.deepEqual(await stageBox(), still, 'la slide a bougé à la sélection');
  assert.ok(Math.abs(await navOffset()) <= 1, 'flèches hors de la mi-hauteur de la slide');

  // 1. Géométrie héritée, markdown mis en forme pendant la saisie, première validation.
  await open(p, 'Corps');
  const fit = await p.evaluate(() => {
    const host = getComputedStyle(document.querySelector('.editor'));
    const content = getComputedStyle(document.querySelector('.editor .cm-content'));
    const line = getComputedStyle(document.querySelector('.editor .cm-line'));
    return {
      font: host.fontFamily === line.fontFamily,
      lead: host.lineHeight === line.lineHeight,
      pad: line.paddingLeft,
      caret: host.caretColor === content.caretColor,
    };
  });
  assert.deepEqual(fit, { font: true, lead: true, pad: '0px', caret: true });
  await p.keyboard.press('ControlOrMeta+a');
  await p.keyboard.type(BASE);
  assert.equal(await p.locator('.editor .cm-md-bold').innerText(), 'montrer');
  const pill = p.locator('.editor .cm-md-highlight');
  assert.equal(await pill.innerText(), 'ici');
  // {2} : deuxième couleur de la palette papier, #c4622d.
  assert.equal(await pill.evaluate((el) => getComputedStyle(el).backgroundColor), 'rgb(196, 98, 45)');
  assert.equal(await p.locator('.editor .cm-md-mark').count(), 4);
  await commitWith(p, () => p.keyboard.press('ControlOrMeta+Enter'));
  assert.deepEqual(await points(p), [{ runs: RUNS }]);
  const undo1 = await undoLabel(p);
  assert.equal(undo1, 'Annuler : Modifier la slide');

  // 2. Rouvrir redonne le même markdown ; Échap ferme sans rien changer.
  await open(p, 'Corps');
  assert.equal(await text(p), BASE);
  await p.keyboard.type('x');
  await p.keyboard.press('Escape');
  await closed(p);
  await p.waitForTimeout(700); // une validation fautive aurait le temps d'arriver
  assert.deepEqual(await points(p), [{ runs: RUNS }]);

  // 3. Un clic dans la marge de l'éditeur ne le ferme pas.
  await open(p, 'Corps');
  const bb = await p.locator('.editor').boundingBox();
  // Au milieu du bord bas : les coins arrondis ne sont pas à l'éditeur.
  const [x, y] = [bb.x + bb.width / 2, bb.y + bb.height - 2];
  // Le point visé est bien hors du texte (sinon le test ne prouverait rien).
  assert.equal(await p.evaluate(([x, y]) => !document.elementFromPoint(x, y)?.closest('.cm-content'), [x, y]), true);
  await p.mouse.click(x, y);
  await p.waitForTimeout(300);
  assert.equal(await p.evaluate(() => document.activeElement?.classList.contains('cm-content')), true);
  assert.equal(await p.locator('.editor').count(), 1);

  // 4. ⌘B est un pas d'annulation à part, même suivi d'une frappe.
  await p.keyboard.press('ControlOrMeta+End');
  await p.keyboard.type(' fin');
  for (let i = 0; i < 3; i++) await p.keyboard.press('Shift+ArrowLeft');
  await p.keyboard.press('ControlOrMeta+b');
  assert.equal(await text(p), `${BASE} **fin**`);
  await p.keyboard.press('ControlOrMeta+End');
  await p.keyboard.type('X');
  await p.keyboard.press('ControlOrMeta+z');
  assert.equal(await text(p), `${BASE} **fin**`);
  await p.keyboard.press('ControlOrMeta+z');
  assert.equal(await text(p), `${BASE} fin`);
  await p.keyboard.press('ControlOrMeta+End');
  for (let i = 0; i < 4; i++) await p.keyboard.press('Backspace');
  assert.equal(await text(p), BASE);

  // 5. Collage : le texte brut seulement, mis en forme comme s'il était tapé.
  await paste(p, ' **collé**', '<i>html</i>');
  assert.equal(await text(p), `${BASE} **collé**`);
  assert.equal(await p.locator('.editor .cm-md-bold').count(), 2);
  await p.keyboard.press('ControlOrMeta+z');
  assert.equal(await text(p), BASE);

  // 6. Entrée dans un bloc : un simple retour à la ligne, même entre crochets.
  await p.keyboard.type('[]');
  await p.keyboard.press('ArrowLeft');
  await p.keyboard.press('Enter');
  assert.equal(await text(p), `${BASE}[\n]`);
  await p.keyboard.press('Backspace');
  await p.keyboard.press('Backspace');
  await p.keyboard.press('Delete');
  assert.equal(await text(p), BASE);
  await p.keyboard.press('Enter');
  await p.keyboard.type('- deux');
  // Les ⌘Z de l'éditeur n'ont pas touché le deck.
  assert.equal(await undoLabel(p), undo1);
  assert.deepEqual(await points(p), [{ runs: RUNS }]);

  // 7. La perte du focus valide, avec le texte affiché ; rouvrir le redonne.
  await commitWith(p, () => p.locator('.editor .cm-content').evaluate((el) => el.blur()));
  assert.deepEqual(await points(p), [{ runs: RUNS }, { runs: [{ t: 'deux' }], list: 'bullet' }]);
  await open(p, 'Corps');
  assert.equal(await text(p), `${BASE}\n- deux`);
  await p.keyboard.press('Escape');
  await closed(p);

  // 8. Champ : texte brut sur une ligne ; collage multiligne aplati ; Maj+Entrée et Entrée
  //    valident et passent au champ suivant (l'éditeur reste ouvert, Échap le ferme).
  const title = (await slide(p)).params.title;
  await open(p, 'Titre');
  await p.keyboard.press('ControlOrMeta+a');
  await paste(p, 'a\nb');
  assert.equal(await text(p), 'a b');
  await p.keyboard.press('ControlOrMeta+z');
  assert.equal(await text(p), title);
  await p.keyboard.press('ControlOrMeta+a');
  await p.keyboard.type('Constat');
  await nextWith(p, () => p.keyboard.press('Shift+Enter'), 'Constat');
  await open(p, 'Titre');
  await p.keyboard.type(' **2**');
  assert.equal(await p.locator('.editor .cm-md-bold').count(), 0);
  await nextWith(p, () => p.keyboard.press('Enter'), 'Constat **2**');

  // L'hôte garde la slide courante (t-87) : rendre la slide 1 pour que les scripts suivants
  // (toolbar.cjs, panel.cjs), qui partent de s-title, ne la trouvent pas sur s-constat.
  await p.click('.thumb >> nth=0');
  await until(
    () => p.evaluate(() =>
      fetch('/api', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ tab: 't-1', req: { cmd: 'snapshot' } }) })
        .then((r) => r.json())
        .then((s) => s.ui.slide)),
    (id) => id === 's-title',
    'retour à la slide 1',
  );

  assert.deepEqual(errors, []);
  await b.close();
})().catch((e) => { console.error(e); process.exit(1); });
