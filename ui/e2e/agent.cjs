// Agent (faux agent ACP) : reformulation dans une fiche, variantes dans le tiroir, annulation,
// puis un second onglet qui répond pendant que le premier attend toujours une question.
// Contre `oku-slides serve fixtures/roadmap.json` lancé avec OKU_SLIDES_AGENT__COMMAND = faux agent
// (just e2e s'en charge).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

// Le serveur n'accepte que du JSON déclaré (garde anti-CSRF de `oku-slides serve`).
const post = (p, endpoint, req) =>
  p.evaluate(
    ([e, r]) => fetch(e, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(r) }).then((x) => x.json()),
    [endpoint, req],
  );
const api = (p, req, tab = 't-1') => post(p, '/api', { tab, req });
const prompt = (p, text, tab = 't-1') => post(p, '/api/agent', { cmd: 'prompt', tab, text, ui: {} });
const textOf = async (p) => (await api(p, { cmd: 'slide', id: 's-libre' })).targets.find((t) => t.id === 'e-texte').text;

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  await p.goto(URL);
  // tabs.cjs laisse un autre onglet actif : celui du fichier (t-1) repasse devant.
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');
  const original = await textOf(p);

  // 1. Reformulation : la fiche s'ouvre sous le texte, « 2 » choisit la seconde proposition.
  await prompt(p, 'rends ce texte plus clair');
  await p.waitForSelector('.agent-sheet .diff-text');
  assert.equal(await p.locator('.agent-sheet .diff-text').count(), 2);
  await p.keyboard.press('2');
  await p.waitForSelector('.agent-sheet', { state: 'detached' });
  await p.getByText('Texte reformulé.').waitFor();
  assert.equal(await textOf(p), 'Un bloc libre');
  assert.equal((await api(p, { cmd: 'snapshot' })).undo_label, 'Agent : rends ce texte plus clair');

  // 2. Variantes : le tiroir montre deux rendus ; cliquer la seconde l'applique.
  await prompt(p, 'propose deux variantes');
  await p.waitForSelector('.variant-drawer .slide-thumb');
  assert.equal(await p.locator('.variant-drawer .slide-thumb').count(), 2);
  await p.locator('.variant-drawer .slide-thumb').nth(1).click();
  await p.waitForSelector('.variant-drawer', { state: 'detached' });
  assert.equal(await textOf(p), 'Variante deux');
  assert.equal((await api(p, { cmd: 'snapshot' })).undo_label, 'Variante : Deux');

  // 3. Deux annulations : la variante, puis tout le premier tour.
  await p.locator('.stage').click({ position: { x: 5, y: 5 } });
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(500);
  await p.keyboard.press('Control+z');
  await p.waitForTimeout(700);
  assert.equal(await textOf(p), original);

  // 4. Un second onglet : son fil est vide ; il répond pendant que l'onglet 1, à qui
  // on vient de poser une question (troisième tour, laissée sans réponse), attend
  // toujours — la fiche reste là au retour.
  await prompt(p, 'et si je changeais d’avis ?');
  await p.waitForSelector('.agent-sheet .chips');
  assert.equal(await p.locator('.agent-sheet .chips .chip').count(), 2);

  const created = await post(p, '/api/workspace', { cmd: 'new_tab', window: 'w-1' });
  const tab2 = created.windows.find((w) => w.label === 'w-1').active;
  assert.notEqual(tab2, 't-1');
  await p.reload();
  await p.waitForSelector('.stage img');
  assert.equal(await p.locator('.agent-thread').count(), 0);

  // Son tour (propre à cette session, voir e2e.json → sessions.2) applique un vrai
  // changement sur son propre deck vierge (slide « s-1 ») : le vérifier confirme que
  // l'onglet 2 a bien joué son tour, pas seulement reçu un morceau de texte.
  await prompt(p, 'bonjour', tab2);
  await p.getByText('Titre changé.').waitFor();
  const snap2 = await api(p, { cmd: 'snapshot' }, tab2);
  assert.equal(snap2.slides.find((s) => s.id === 's-1').title, 'Second onglet, bonjour');
  // Dans un tour, l'étiquette d'annulation est celle du tour (le prompt), pas celle
  // passée à `apply` : voir `ApplyArgs::label`.
  assert.equal(snap2.undo_label, 'Agent : bonjour');

  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.agent-sheet .chips');

  assert.deepEqual(errors, []);
  await b.close();
  console.log('agent : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
