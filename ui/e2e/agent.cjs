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
const textOf = async (p) => (await api(p, { cmd: 'slide', id: 's-libre' })).fields.find((f) => f.owner === 'e-texte').text;

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
  // Sans `report`, le tour se clôt par un reçu de repli titré par le texte de l'agent.
  const rewritten = p.locator('.agent-turn', { hasText: 'rends ce texte plus clair' }).locator('.a2ui-receipt .a2ui-title');
  await rewritten.waitFor();
  assert.equal(await rewritten.innerText(), 'Texte reformulé.');
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
  // Fil vide : rien de visible, seules ses régions annoncées restent montées (t-451).
  assert.equal(await p.locator('.agent-turn').count(), 0);
  assert.equal(await p.locator('.agent-thread.vacant').count(), 1);

  // Son tour (propre à cette session, voir e2e.json → sessions.2) applique un vrai
  // changement sur son propre deck vierge (slide « s-1 ») : le vérifier confirme que
  // l'onglet 2 a bien joué son tour, pas seulement reçu un morceau de texte.
  await prompt(p, 'bonjour', tab2);
  const greeted = p.locator('.agent-turn', { hasText: 'bonjour' }).locator('.a2ui-receipt .a2ui-title');
  await greeted.waitFor();
  assert.equal(await greeted.innerText(), 'Titre changé.');
  const snap2 = await api(p, { cmd: 'snapshot' }, tab2);
  assert.equal(snap2.slides.find((s) => s.id === 's-1').title, 'Second onglet, bonjour');
  // Dans un tour, l'étiquette d'annulation est celle du tour (le prompt), pas celle
  // passée à `apply` : voir `ApplyArgs::label`.
  assert.equal(snap2.undo_label, 'Agent : bonjour');

  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.agent-sheet .chips');

  // 5. Un tour qui se clôt par `report` : le fil se replie en un résumé et un reçu.
  // (Le fil de t-1 garde ses tours précédents : un seul de plus.)
  // La question du troisième tour attend encore : on y répond, il se clôt, puis le tour suivant part.
  await p.locator('.agent-sheet .chips .chip').first().click();
  await p.waitForSelector('.agent-sheet', { state: 'detached' });
  await p.locator('.agent-turn .a2ui-receipt').first().waitFor();
  const before = await p.locator('.agent-turn').count();
  await prompt(p, 'change le fond');
  const turn = p.locator('.agent-turn', { hasText: 'change le fond' });
  await turn.locator('.a2ui-prompt').waitFor();
  await turn.locator('.a2ui-receipt .a2ui-title').waitFor();
  assert.equal(await turn.locator('.a2ui-receipt .a2ui-title').innerText(), 'Fond bleu nuit');
  // Le reçu désigne sa slide ; l'interface en montre la miniature depuis son cache de SVG (t-450).
  await turn.locator('.a2ui-receipt-thumb').waitFor();
  assert.match(await turn.locator('.a2ui-receipt-thumb').getAttribute('src'), /^blob:/);
  await turn.locator('.a2ui-timeline-summary').waitFor({ state: 'visible' });
  assert.equal(await p.locator('.agent-turn').count(), before + 1);

  // 6. Une question sans ancre : elle vit dans le dock ; répondre la fige dans le tour.
  await prompt(p, 'quelle couleur ?');
  await p.waitForSelector('.agent-dock .chips .chip');
  assert.equal(await p.locator('.agent-dock .chips .chip').count(), 2);
  await p.locator('.agent-dock .chips .chip').nth(1).click();
  await p.waitForSelector('.agent-dock.vacant');
  await p.locator('.agent-turn', { hasText: '✓' }).filter({ hasText: 'Sable' }).first().waitFor();

  // 7. Deux questions parallèles au dock (t-451) : toutes deux visibles ; « 2 » répond à la
  // dernière (au plus près du micro), un clic à l'autre.
  await prompt(p, 'titre et police ?');
  await p.waitForFunction(() => document.querySelectorAll('.agent-dock .agent-dock-item.ask').length === 2);
  // Les deux appels partent ensemble : l'ordre d'arrivée n'est pas fixé, on lit les libellés.
  const asks = p.locator('.agent-dock .agent-dock-item.ask');
  const label = async (ask, i) => (await asks.nth(ask).locator('.chip').nth(i).innerText()).replace(/^\d\s*/, '');
  const [first, last] = [await label(0, 0), await label(1, 1)];
  await p.keyboard.press('2');
  await p.locator('.agent-turn', { hasText: '✓' }).filter({ hasText: last }).first().waitFor();
  assert.equal(await asks.count(), 1);
  await asks.first().locator('.chip').first().click();
  await p.waitForSelector('.agent-dock.vacant');
  await p.locator('.agent-turn', { hasText: '✓' }).filter({ hasText: first }).first().waitFor();

  assert.deepEqual(errors, []);
  await b.close();
  console.log('agent : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
