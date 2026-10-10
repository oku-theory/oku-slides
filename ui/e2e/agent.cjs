// Agent (faux agent ACP) : reformulation dans une fiche, variantes dans le tiroir, annulation,
// puis un second onglet qui répond pendant que le premier attend toujours une question.
// Contre `oku-slides serve fixtures/roadmap.json` lancé avec le faux agent comme agent actif
// (`config.toml` du dossier de données isolé ; just e2e s'en charge).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');
const { until } = require('./until.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

const api = (p, req, tab = 't-1') => post(p, '/api', { tab, req });
const prompt = (p, text, tab = 't-1') => post(p, '/api/agent', { cmd: 'prompt', tab, text, ui: {} });
/** Le compteur d'outils du résumé d'un tour (« ▸ 2 outils · 0 s » : le premier nombre, quelle que soit la langue). */
const toolCount = async (turn) => {
  const summary = turn.locator('.a2ui-timeline-summary');
  await summary.waitFor({ state: 'visible' });
  return Number((await summary.innerText()).match(/\d+/)?.[0]);
};
/** Titre et faits (joints par « · ») du reçu d'un tour. */
const receiptOf = async (turn) => {
  const card = turn.locator('.a2ui-receipt');
  await card.locator('.a2ui-title').waitFor();
  return { title: await card.locator('.a2ui-title').innerText(), facts: await card.locator('.a2ui-hint').innerText() };
};
/** Fil, dock vides et région annoncée : aucune place (au plus 1 px, et le retirer ne déplace rien
 * autour : hors du flux, donc hors du `gap` du panneau), mais ni masqués ni retirés de l'arbre
 * d'accessibilité (`checkVisibility` : ni `display: none` ni `visibility: hidden`). */
const assertUnobtrusive = async (p, selector) => {
  const seen = await p.locator(selector).evaluate((el) => {
    const r = el.getBoundingClientRect();
    const shown = el.checkVisibility({ visibilityProperty: true });
    const others = [...el.parentElement.children].filter((x) => x !== el);
    const at = () => others.map((x) => JSON.stringify(x.getBoundingClientRect()));
    const before = at();
    el.style.display = 'none';
    const moved = at().some((b, i) => b !== before[i]);
    el.style.display = '';
    return { w: r.width, h: r.height, shown, moved };
  });
  assert.ok(seen.w <= 1 && seen.h <= 1 && !seen.moved, `${selector} prend de la place : ${JSON.stringify(seen)}`);
  assert.ok(seen.shown, `${selector} est retiré de l'arbre d'accessibilité`);
};
/** Pastilles posées dans la page (feuilles de style de l'app) et mesurées : un libellé sur
 * plusieurs lignes part du début de la ligne (à gauche, à droite en RTL), pas du centre ;
 * « ↓ Tour en cours » tient sur une ligne, dans sa pastille. */
const assertPills = async (p) => {
  const seen = await p.evaluate(() => {
    const host = document.createElement('div');
    host.style.cssText = 'position: fixed; inset: 0 auto auto 0; width: 180px; display: flex; flex-direction: column;';
    document.body.append(host);
    const lines = (el) => {
      const range = document.createRange();
      range.selectNodeContents(el);
      return [...range.getClientRects()].map((r) => ({ left: r.left, right: r.right, top: r.top }));
    };
    const aligned = (html, dir) => {
      host.dir = dir;
      host.innerHTML = html;
      const el = host.querySelector('.probe');
      const rects = lines(el);
      const tops = new Set(rects.map((r) => Math.round(r.top)));
      const edges = rects.map((r) => (dir === 'rtl' ? r.right : r.left));
      const widths = new Set(rects.map((r) => Math.round(r.right - r.left)));
      return { lines: tops.size, ragged: widths.size > 1, spread: Math.max(...edges) - Math.min(...edges) };
    };
    const text = 'Remplacer le titre par une phrase nettement plus courte';
    const out = {};
    for (const dir of ['ltr', 'rtl']) {
      out[`chip-${dir}`] = aligned(`<button class="chip probe">${text}</button>`, dir);
      out[`btn-${dir}`] = aligned(`<div class="a2ui-row"><button class="btn probe">${text}</button></div>`, dir);
    }
    host.dir = 'ltr';
    host.innerHTML = '<button class="token agent-jump probe">↓ Tour en cours</button>';
    const jump = host.querySelector('.probe');
    out.jump = { lines: new Set(lines(jump).map((r) => Math.round(r.top))).size, overflow: jump.scrollWidth - jump.clientWidth };
    host.remove();
    return out;
  });
  for (const key of ['chip-ltr', 'chip-rtl', 'btn-ltr', 'btn-rtl']) {
    const s = seen[key];
    assert.ok(s.lines >= 2 && s.ragged, `${key} : libellé d'essai sur une seule ligne ou lignes égales : ${JSON.stringify(s)}`);
    assert.ok(s.spread < 1, `${key} : lignes non alignées au début : ${JSON.stringify(s)}`);
  }
  assert.ok(seen.jump.lines === 1 && seen.jump.overflow <= 0, `« ↓ Tour en cours » déborde ou passe à la ligne : ${JSON.stringify(seen.jump)}`);
};
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
  assert.equal(await toolCount(p.locator('.agent-turn', { hasText: 'rends ce texte plus clair' })), 1);
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
  const variants = p.locator('.agent-turn', { hasText: 'propose deux variantes' });
  assert.deepEqual(await receiptOf(variants), { title: 'Deux variantes proposées', facts: 'Un : texte court · Deux : texte long' });
  assert.equal(await toolCount(variants), 2);

  // 3. Deux annulations : la variante, puis tout le premier tour.
  await p.locator('.stage').click({ position: { x: 5, y: 5 } });
  const undoLabel = async () => (await api(p, { cmd: 'snapshot' })).undo_label;
  await p.keyboard.press('Control+z');
  await until(p, async () => (await undoLabel()) === 'Agent : rends ce texte plus clair', 'première annulation (la variante) pas faite');
  await p.keyboard.press('Control+z');
  await until(p, async () => (await textOf(p)) === original, 'seconde annulation (le premier tour) pas faite');
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
  // Fil vide : seul son en-tête (bouton des agents) reste visible ; aucun tour, la zone des
  // tours ne prend aucune hauteur, le dock vide et la région annoncée restent montés sans place (t-451, t-775).
  assert.equal(await p.locator('.agent-turn').count(), 0);
  assert.equal(await p.locator('.agent-thread.vacant').count(), 0);
  assert.equal(await p.locator('.agent-thread-head .agent-picker-toggle').count(), 1);
  const scrollerHeight = await p.locator('.agent-scroller').evaluate((el) => el.getBoundingClientRect().height);
  assert.ok(scrollerHeight <= 1, `la zone des tours vide prend de la place : ${scrollerHeight}px`);
  for (const selector of ['.agent-dock.vacant', '.agent-announce']) await assertUnobtrusive(p, selector);

  // Son tour (propre à cette session, voir e2e.json → sessions.2) applique un vrai
  // changement sur son propre deck vierge (slide « s-1 ») : le vérifier confirme que
  // l'onglet 2 a bien joué son tour, pas seulement reçu un morceau de texte.
  await prompt(p, 'bonjour', tab2);
  const greeted = p.locator('.agent-turn', { hasText: 'bonjour' });
  assert.deepEqual(await receiptOf(greeted), { title: 'Titre du second onglet', facts: '« Second onglet, bonjour »' });
  assert.equal(await toolCount(greeted), 1);
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
  const anchored = p.locator('.agent-turn', { hasText: 'et si je changeais' });
  assert.deepEqual(await receiptOf(anchored), { title: 'Choix noté', facts: 'la réponse sera suivie' });
  assert.equal(await toolCount(anchored), 1);
  const before = await p.locator('.agent-turn').count();
  await prompt(p, 'change le fond');
  const turn = p.locator('.agent-turn', { hasText: 'change le fond' });
  await turn.locator('.a2ui-prompt').waitFor();
  await turn.locator('.a2ui-receipt .a2ui-title').waitFor();
  assert.equal(await turn.locator('.a2ui-receipt .a2ui-title').innerText(), 'Fond bleu nuit');
  // Le reçu désigne sa slide ; l'interface en montre la miniature depuis son cache de SVG (t-450).
  await turn.locator('.a2ui-receipt-thumb').waitFor();
  assert.match(await turn.locator('.a2ui-receipt-thumb').getAttribute('src'), /^blob:/);
  assert.equal(await toolCount(turn), 2);
  assert.equal(await p.locator('.agent-turn').count(), before + 1);

  // 6. Une question sans ancre : elle vit dans le dock ; répondre la fige dans le tour.
  await prompt(p, 'quelle couleur ?');
  await p.waitForSelector('.agent-dock .chips .chip');
  assert.equal(await p.locator('.agent-dock .chips .chip').count(), 2);
  // En Verre, le dock flotte comme une fiche : marge intérieure, coins arrondis, pas de filet.
  const glass = await p.locator('.agent-dock').evaluate((el) => {
    const c = getComputedStyle(el);
    return { pad: parseFloat(c.paddingTop), radius: parseFloat(c.borderTopLeftRadius), rule: parseFloat(c.borderTopWidth) };
  });
  assert.ok(glass.pad > 0 && glass.radius > 0 && glass.rule === 0, `dock en Verre : ${JSON.stringify(glass)}`);
  await p.locator('.agent-dock .chips .chip').nth(1).click();
  await p.waitForSelector('.agent-dock.vacant');
  await p.locator('.agent-turn', { hasText: '✓' }).filter({ hasText: 'Sable' }).first().waitFor();
  const colour = p.locator('.agent-turn', { hasText: 'quelle couleur ?' });
  assert.deepEqual(await receiptOf(colour), { title: 'Couleur retenue', facts: 'le fond suivra' });
  assert.equal(await toolCount(colour), 1);

  // 7. Deux questions parallèles au dock (t-451) : toutes deux visibles ; « 2 » répond à la
  // dernière (au plus près du micro), un clic à l'autre.
  await prompt(p, 'titre et police ?');
  await p.waitForFunction(() => document.querySelectorAll('.agent-dock .agent-dock-item.ask').length === 2);
  // Les deux appels partent ensemble : l'ordre d'arrivée n'est pas fixé, on lit les libellés.
  const asks = p.locator('.agent-dock .agent-dock-item.ask');
  // La hauteur du dock se mesure au panneau entier (45 % de sa hauteur), pas au fil ni à la fenêtre :
  // panneau rétréci de moitié, plafond du dock rétréci d'autant.
  const dockCap = () =>
    p.locator('.agent-dock').evaluate((el) => {
      const notes = el.closest('.notes');
      const c = getComputedStyle(notes);
      const inner = notes.clientHeight - parseFloat(c.paddingTop) - parseFloat(c.paddingBottom);
      return { cap: parseFloat(getComputedStyle(el).maxHeight), inner, height: el.getBoundingClientRect().height };
    });
  for (const height of [900, 520]) {
    await p.setViewportSize({ width: 1440, height });
    // La mise en page suit le redimensionnement : attendre que le plafond soit recalculé (au
    // plus 5 s ; sinon l'assertion qui suit dit ce qui a été vu).
    await p
      .waitForFunction(() => {
        const el = document.querySelector('.agent-dock');
        const notes = el?.closest('.notes');
        if (!notes) return false;
        const c = getComputedStyle(notes);
        const inner = notes.clientHeight - parseFloat(c.paddingTop) - parseFloat(c.paddingBottom);
        return Math.abs(parseFloat(getComputedStyle(el).maxHeight) - 0.45 * inner) < 1;
      }, null, { timeout: 5000 })
      .catch(() => {});
    const seen = await dockCap();
    assert.ok(Math.abs(seen.cap - 0.45 * seen.inner) < 1, `plafond du dock à ${height} px : ${JSON.stringify(seen)}`);
    assert.ok(seen.height <= seen.cap + 0.5, `dock plus haut que son plafond : ${JSON.stringify(seen)}`);
    // Fenêtre basse : les deux questions dépassent le plafond, le dock s'y arrête (et défile).
    if (height === 520) assert.ok(seen.cap - seen.height < 1, `dock non plafonné à ${height} px : ${JSON.stringify(seen)}`);
  }
  await p.setViewportSize({ width: 1440, height: 900 });
  await assertPills(p);
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
