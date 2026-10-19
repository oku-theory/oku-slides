// Panneau des agents (menu des réglages de la dictée) : l'ouvrir, passer au second faux agent, lire sa
// réponse, revenir au premier ; « Local » sans modèle propose de le télécharger.
// Contre `oku-slides serve` lancé par `just e2e` (deux presets : « Faux » choisi, « Faux bis »).
// Choisir un agent change l'agent de tout le serveur : la spec revient à « Faux » pour les suivantes.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./lib/http.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

const prompt = (p, text, tab = 't-1') => post(p, '/api/agent', { cmd: 'prompt', tab, text, ui: {} });
const chip = (p, name) => p.locator('.agent-picker .chip', { hasText: new RegExp(`^\\s*${name}\\s*$`) });
/** Une pastille est choisie quand le ChoiceChips la marque (aria-pressed/checked/selected ou classe). */
const isChosen = (c) =>
  c.evaluate((el) => {
    const on = (v) => v === 'true';
    return on(el.getAttribute('aria-pressed')) || on(el.getAttribute('aria-checked')) || on(el.getAttribute('aria-selected')) || /\b(selected|active|on)\b/.test(el.className);
  });
/**
 * Attend que la pastille `name` soit choisie : le menu rouvert montre d'abord la dernière surface
 * reçue, puis celle que l'hôte renvoie à jour.
 */
const untilChosen = async (p, name) => {
  for (let i = 0; i < 50; i++) {
    if (await isChosen(chip(p, name))) return;
    await p.waitForTimeout(100);
  }
  assert.fail(`« ${name} » devrait être choisi`);
};
/** Clique la pastille de l'agent `name` et attend que l'hôte ait accepté le choix (réponse à `select_agent`). */
const choose = async (p, name) => {
  const accepted = p.waitForResponse((r) => r.url().endsWith('/api/agent') && (r.request().postData() ?? '').includes('"select_agent"'));
  await chip(p, name).click();
  const response = await accepted;
  assert.ok(response.ok(), `choix de « ${name} » refusé : ${response.status()}`);
};
/** Le bouton des réglages de la barre de dictée. */
const gear = (p) => p.locator('.device.dictation button[aria-label="Réglages"]');
/** Ouvre le menu des réglages ; sa section des agents arrive de l'hôte. */
const openSettings = async (p) => {
  await gear(p).click();
  await p.waitForSelector('.dictation-settings .agent-picker');
};
/** Ferme le menu des réglages (même bouton). */
const closeSettings = async (p) => {
  await gear(p).click();
  await p.waitForSelector('.dictation-settings', { state: 'detached' });
};

(async () => {
  const b = await chromium.launch({ channel: process.env.E2E_CHANNEL });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(URL);
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  await p.waitForSelector('.stage img');

  // 1. La section des agents montre les deux agents ; « Faux » est choisi.
  await openSettings(p);
  await chip(p, 'Faux').waitFor();
  await chip(p, 'Faux bis').waitFor();
  assert.ok(await isChosen(chip(p, 'Faux')), '« Faux » devrait être choisi');
  assert.ok(!(await isChosen(chip(p, 'Faux bis'))), '« Faux bis » ne devrait pas être choisi');

  // 2. Choisir « Faux bis » laisse le menu ouvert ; son agent répond au prochain tour.
  await choose(p, 'Faux bis');
  assert.equal(await p.locator('.dictation-settings').count(), 1, 'le menu des réglages devrait rester ouvert');
  await closeSettings(p);
  await prompt(p, 'bonjour');
  await p.locator('.agent-turn', { hasText: "Réponse de l'agent bis." }).waitFor();

  // 3. Le menu rouvert retient « Faux bis » ; on revient à « Faux » (agent par défaut des specs suivantes).
  await openSettings(p);
  await untilChosen(p, 'Faux bis');
  await choose(p, 'Faux');
  await closeSettings(p);
  await openSettings(p);
  await untilChosen(p, 'Faux');

  // 4. « Local » sans modèle : sur Apple silicon, le panneau propose de télécharger le modèle (jamais
  // cliqué : 2,5 Go) ; ailleurs, l'agent local ne tourne pas et rien n'est proposé. Retour à « Faux ».
  await choose(p, 'Local');
  await untilChosen(p, 'Local');
  const download = p.locator('.agent-picker button', { hasText: 'Télécharger le modèle' });
  if (process.platform === 'darwin' && process.arch === 'arm64') {
    await download.waitFor();
  } else {
    assert.equal(await download.count(), 0, 'aucun téléchargement hors Apple silicon');
  }
  await choose(p, 'Faux');
  await untilChosen(p, 'Faux');

  assert.deepEqual(errors, []);
  await b.close();
  console.log('agent-picker : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
