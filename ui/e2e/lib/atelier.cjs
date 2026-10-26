// The workshop no longer opens from the menu: the agent opens it (tool `edit_pack`, t-988), in a
// pack tab of its own for a library pack. The fake agent plays the `ouvre l'atelier` turn of fixtures/agent/e2e.json.
const assert = require('node:assert/strict');
const { post, activeTab } = require('./http.cjs');
const { logOffset, mcpResults } = require('./agentLog.cjs');
const { SLOW, until, untilValue } = require('./until.cjs');

const view = (p) => post(p, '/api/workspace', { cmd: 'view' });
const snapshot = (p, tab) => post(p, '/api', { tab, req: { cmd: 'snapshot' } });

/**
 * Asks the agent of `tab` (default: the active tab) for the workshop, then waits for it: a new
 * active pack tab, or for a deck-only pack the same tab in its workshop. Reloads the page (serve
 * pushes no view) and returns the tab that now edits the pack.
 */
async function openAtelier(p, tab) {
  const LOG = process.env.E2E_AGENT_LOG;
  assert.ok(LOG, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const before = tab ?? (await activeTab(p));
  const from = logOffset(LOG);
  const reply = await post(p, '/api/agent', { cmd: 'prompt', tab: before, text: "ouvre l'atelier", ui: {} });
  assert.equal(reply.error, undefined, `consigne « ouvre l'atelier » refusée : ${reply.error}`);
  let editing = null;
  await until(p, async () => {
    if (mcpResults(LOG, from, 'edit_pack').length === 0) return false;
    const win = (await view(p)).windows[0];
    const active = win.tabs.find((x) => x.id === win.active);
    const packTab = active && active.id !== before && active.document.kind === 'pack';
    const workshop = active && active.id === before && active.document.kind === 'deck' && active.editing_pack;
    if (packTab || workshop) editing = active.id;
    return editing !== null;
  }, `l'atelier ne s'ouvre pas depuis ${before}`, SLOW);
  await p.reload();
  await p.getByRole('button', { name: 'Terminer' }).waitFor();
  return editing;
}

/**
 * Closes every pack tab over HTTP (idempotent): `t-1` active first and the page reloaded, so no
 * App of a pack tab is mounted; a dirty one is saved first (an HTTP-dirtied or inactive tab never
 * autosaves), then closed with discard. Reloads the page at the end.
 */
async function closePackTabs(p) {
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  const tabs = (await view(p)).windows.flatMap((w) => w.tabs).filter((x) => x.document.kind === 'pack');
  for (const { id } of tabs) {
    if ((await snapshot(p, id)).dirty) await post(p, '/api', { tab: id, req: { cmd: 'save_pack', as_new: false } });
    await untilValue(p, async () => (await snapshot(p, id)).dirty, (dirty) => dirty === false, `l'onglet de pack ${id} reste modifié`, 10_000);
    const closed = await post(p, '/api/workspace', { cmd: 'close', tab: id, discard: true });
    assert.equal(closed.error, undefined, `fermeture de l'onglet de pack ${id} refusée : ${closed.error}`);
  }
  await p.reload();
}

/** Opens pack `pack` from tab `from` over HTTP, as a widget click does; returns its tab, read in the view. */
async function openPackTab(p, from, pack, widget = null) {
  const v = await post(p, '/api/workspace', { cmd: 'open_pack', tab: from, pack, widget });
  const tab = v.windows?.flatMap((w) => w.tabs).find((t) => t.document.kind === 'pack' && t.document.pack === pack)?.id;
  assert.ok(tab, `open_pack refusé : ${JSON.stringify(v).slice(0, 300)}`);
  return tab;
}

/**
 * Opens the Essentiel `t-1` pins in its pack tab (from 140 on, the agent's default pack is the deck's
 * own one, edited in place), reloads the page and returns the tab.
 */
async function openEssentiel(p) {
  const tab = await openPackTab(p, 't-1', (await snapshot(p, 't-1')).layouts.essentiel.id);
  await p.reload();
  await p.getByRole('button', { name: 'Terminer' }).waitFor();
  return tab;
}

module.exports = { openAtelier, closePackTabs, openPackTab, openEssentiel };
