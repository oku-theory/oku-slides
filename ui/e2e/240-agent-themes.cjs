// L'agent ouvre le sélecteur de thème de l'hôte (outil `show_themes`) : il arrive au dock, reste
// ouvert après la fin du tour, un clic applique le thème et le choix est noté pour l'agent.
// Contre `oku-slides serve` lancé par `just e2e` (faux agent « Faux », journal dans E2E_AGENT_LOG).
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { post, activeSnapshot } = require('./lib/http.cjs');
const { until, untilValue } = require('./lib/until.cjs');
const { logOffset, logSince } = require('./lib/agentLog.cjs');
const URL = process.env.E2E_URL ?? 'http://127.0.0.1:4870/';
const LOG = process.env.E2E_AGENT_LOG;

/** Les consignes reçues par le faux agent depuis `from`, en JSON. */
const prompts = (from) => logSince(LOG, from).filter((m) => m.method === 'session/prompt').map((m) => JSON.stringify(m));
/** Les résultats MCP (`mcp/result`) de `tool` notés depuis `from`. */
const results = (tool, from) =>
  logSince(LOG, from).filter((m) => m.method === 'mcp/result' && m.params.tool === tool).map((m) => m.params.out);
const themeNs = async (p) => (await activeSnapshot(p)).theme ?? null;
const picker = (p) => p.locator('.agent-dock [data-surface-id="host-theme-picker"]');
const cards = (p) => picker(p).locator('.theme-choice');
const prompt = (p, text) => post(p, '/api/agent', { cmd: 'prompt', tab: 't-1', text, ui: {} });

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

  // 1. L'agent appelle show_themes : l'outil répond tout de suite, le sélecteur arrive au dock.
  await prompt(p, 'propose les thèmes');
  const outs = await untilValue(p, () => results('show_themes', from), (o) => o.length > 0, 'show_themes doit répondre', 15000);
  const out = JSON.parse(outs[0]);
  assert.ok(!out.isError, `show_themes en erreur : ${outs[0].slice(0, 300)}`);
  assert.deepEqual(JSON.parse(out.content[0].text), { shown: 'host-theme-picker' });
  await cards(p).first().waitFor();
  assert.ok((await cards(p).count()) >= 2, 'au moins deux thèmes');

  // 2. Le tour suivant commence (le précédent est donc clos) : le sélecteur reste ouvert.
  await prompt(p, 'suite des thèmes');
  await until(p, async () => prompts(from).some((m) => m.includes('suite des thèmes')), 'la suite doit atteindre le faux agent', 15000);
  await cards(p).first().waitFor();

  // 3. Un clic sur un thème non actif l'applique.
  const other = picker(p).locator('.theme-choice[aria-pressed="false"]').first();
  const name = await other.locator('bdi').innerText();
  await other.click();
  await until(p, async () => (await themeNs(p)) !== before, 'le thème du deck doit changer');

  // 4. Le choix est noté pour l'agent : la consigne suivante le porte.
  const picked = logOffset(LOG);
  await prompt(p, 'suite des thèmes');
  const sent = await untilValue(p, () => prompts(picked), (m) => m.length > 0, 'la consigne doit atteindre le faux agent', 15000);
  assert.ok(sent[0].includes(`theme picked: \\"${name}\\"`), sent[0].slice(0, 600));

  // 5. Fermer le sélecteur, puis rendre le thème du début.
  await picker(p).getByRole('button', { name: 'Fermer', exact: true }).click();
  await picker(p).waitFor({ state: 'detached' });
  await post(p, '/api', { tab: 't-1', req: { cmd: 'undo' } });
  await until(p, async () => (await themeNs(p)) === before, "l'annulation doit rendre le thème du début");

  assert.deepEqual(errors, []);
  console.log('agent-themes : ok');
  await b.close();
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
