// Panneau des agents : ouvrir la fiche, passer au second faux agent, lire sa réponse, revenir au premier.
// Contre `oku-slides serve` lancé par `just e2e` (deux presets : « Faux » choisi, « Faux bis »).
// Dernier de la liste : choisir un agent change l'agent de tout le serveur.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';

const prompt = (p, text, tab = 't-1') => post(p, '/api/agent', { cmd: 'prompt', tab, text, ui: {} });
const chip = (p, name) => p.locator('.agent-picker .chip', { hasText: new RegExp(`^\\s*${name}\\s*$`) });
/** Une pastille est choisie quand le ChoiceChips la marque (aria-pressed/checked/selected ou classe). */
const isChosen = (c) =>
  c.evaluate((el) => {
    const on = (v) => v === 'true';
    return on(el.getAttribute('aria-pressed')) || on(el.getAttribute('aria-checked')) || on(el.getAttribute('aria-selected')) || /\b(selected|active|on)\b/.test(el.className);
  });
const openSheet = async (p) => {
  await p.locator('button.agent-picker-toggle').click();
  await p.waitForSelector('.agent-picker');
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

  // 1. La fiche montre les deux agents ; « Faux » est choisi.
  await openSheet(p);
  await chip(p, 'Faux').waitFor();
  await chip(p, 'Faux bis').waitFor();
  assert.ok(await isChosen(chip(p, 'Faux')), '« Faux » devrait être choisi');
  assert.ok(!(await isChosen(chip(p, 'Faux bis'))), '« Faux bis » ne devrait pas être choisi');

  // 2. Choisir « Faux bis » ferme la fiche ; son agent répond au prochain tour.
  await chip(p, 'Faux bis').click();
  await p.waitForSelector('.agent-picker', { state: 'detached' });
  await prompt(p, 'bonjour');
  await p.locator('.agent-turn', { hasText: "Réponse de l'agent bis." }).waitFor();

  // 3. La fiche rouverte retient « Faux bis » ; on revient à « Faux » (agent par défaut des specs suivantes).
  await openSheet(p);
  assert.ok(await isChosen(chip(p, 'Faux bis')), '« Faux bis » devrait être choisi');
  await chip(p, 'Faux').click();
  await p.waitForSelector('.agent-picker', { state: 'detached' });
  await openSheet(p);
  assert.ok(await isChosen(chip(p, 'Faux')), '« Faux » devrait être de nouveau choisi');

  assert.deepEqual(errors, []);
  await b.close();
  console.log('agent-picker : ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
