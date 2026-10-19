// L'atelier ne s'ouvre plus par le menu : l'agent l'ouvre (outil `edit_templates`, t-988).
// Le faux agent joue le tour `ouvre l'atelier` de fixtures/agent/e2e.json.
const assert = require('node:assert/strict');
const { post, activeTab } = require('./http.cjs');

/** Demande l'atelier à l'agent de `tab` (défaut : l'onglet actif), puis attend le mode modèles. */
async function openAtelier(p, tab) {
  const target = tab ?? (await activeTab(p));
  const reply = await post(p, '/api/agent', { cmd: 'prompt', tab: target, text: "ouvre l'atelier", ui: {} });
  assert.equal(reply.error, undefined, `consigne « ouvre l'atelier » refusée : ${reply.error}`);
  await p.getByRole('button', { name: 'Terminer' }).waitFor();
}

module.exports = { openAtelier };
