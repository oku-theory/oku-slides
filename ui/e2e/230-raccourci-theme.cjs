// "Theme" shortcut: the prompt alone opens the host's picker in the side panel's Themes tab, without
// the agent; the rail's Themes tab opens it too;
// hovering a theme shows it on the stage, a click or its digit applies it and closes the picker,
// undo goes back to the previous one. Runs against `oku-slides serve` started by `just e2e` (after 220: log created).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post, activeSnapshot } = require('./lib/http.cjs');
const { until } = require('./lib/until.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;

const prompts = (from) => logSince(LOG, from).filter((m) => m.method === 'session/prompt');
const themeNs = async (p) => (await activeSnapshot(p)).theme ?? null;
const picker = (p) => p.locator('.side-body [data-surface-id="host-theme-picker"]');
const cards = (p) => picker(p).locator('.theme-choice');
/** Les noms des cartes actives ; jamais d'exception, une liste vide s'il n'y en a pas. */
const activeNames = (p) => picker(p).locator('.theme-choice[aria-pressed="true"] bdi').allInnerTexts();
/** Le rang de la première carte non active parmi les quatre premières (touches 1–4). */
const firstInactive = async (p) =>
  (await cards(p).evaluateAll((els) => els.map((e) => e.getAttribute('aria-pressed')))).slice(0, 4).indexOf('false');
/** Écrit une consigne dans le brouillon de l'agent et l'envoie. */
const say = async (p, text) => {
  await p.keyboard.press('ControlOrMeta+k');
  await p.locator('.draft-text').fill(text);
  await p.locator('.draft-send').click();
  await p.waitForSelector('.draft-text', { state: 'detached' });
};
/** Rend le focus au document : les touches vont à l'app, pas au dock ni au brouillon. */
const blur = (p) => p.evaluate(() => (document.activeElement instanceof HTMLElement ? document.activeElement.blur() : undefined));

(async () => {
  assert.ok(LOG, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');
  const from = logOffset(LOG);
  const before = await themeNs(p);

  // 1. « Thème. » typed in the draft opens the picker in the side panel, its tab pressed; one theme is active.
  await say(p, 'Thème.');
  await cards(p).first().waitFor();
  assert.equal(await p.locator('.side-rail .themes-toggle').getAttribute('aria-pressed'), 'true');
  assert.ok((await cards(p).count()) >= 2, 'au moins deux thèmes');
  const startActive = await activeNames(p);
  assert.equal(startActive.length, 1);

  // 2. Survoler un thème non actif montre sa miniature sur la scène ; la quitter la retire.
  const i = await firstInactive(p);
  assert.ok(i >= 0, 'une carte non active parmi les quatre premières');
  const other = cards(p).nth(i);
  const name = await other.locator('bdi').innerText();
  const thumb = await other.locator('img').getAttribute('src');
  await other.hover();
  await p.waitForSelector('.stage .stage-hover-preview');
  assert.equal(await p.locator('.stage .stage-hover-preview').getAttribute('src'), thumb);
  await p.mouse.move(0, 0);
  await p.waitForSelector('.stage .stage-hover-preview', { state: 'detached' });

  // 3. A click applies it and closes the picker (t-1539).
  await other.click();
  await until(p, async () => (await themeNs(p)) !== before, 'le thème du deck doit changer');
  await picker(p).waitFor({ state: 'detached' });
  const picked = await themeNs(p);

  // 4. ⌘Z gives the previous theme back; reopened, the picker shows the starting one active.
  await blur(p);
  await p.keyboard.press('ControlOrMeta+z');
  await until(p, async () => (await themeNs(p)) === before, "l'annulation doit rendre le thème précédent");
  await say(p, 'Thème.');
  await cards(p).first().waitFor();
  await until(p, async () => JSON.stringify(await activeNames(p)) === JSON.stringify(startActive), "l'active doit suivre l'annulation");

  // 5. A non-active card's digit applies it and closes the picker too (keys 1–4).
  await blur(p);
  await p.keyboard.press(`Digit${i + 1}`);
  await until(p, async () => (await themeNs(p)) === picked, 'la touche doit appliquer le thème de sa carte');
  await picker(p).waitFor({ state: 'detached' });
  await blur(p);
  await p.keyboard.press('ControlOrMeta+z');
  await until(p, async () => (await themeNs(p)) === before, "l'annulation doit rendre le thème précédent");

  // 6. Reopened from the rail's Themes tab, the pressed tab closes it without picking.
  await p.click('.side-rail .themes-toggle');
  await cards(p).first().waitFor();
  await p.click('.side-rail .themes-toggle');
  await picker(p).waitFor({ state: 'detached' });
  assert.equal(await themeNs(p), before);

  // 7. Aucun prompt n'est parti vers l'agent : un témoin envoyé ensuite est le seul noté.
  await post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text: 'témoin', ui: {} });
  await until(p, async () => prompts(from).length > 0, 'le témoin doit atteindre le faux agent', 15000);
  const sent = prompts(from);
  assert.equal(sent.length, 1, JSON.stringify(sent));
  assert.ok(JSON.stringify(sent[0]).includes('témoin'));

  assert.deepEqual(errors, []);
  console.log('raccourci-theme : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
