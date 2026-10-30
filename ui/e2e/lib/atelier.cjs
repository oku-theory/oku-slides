// The workshop no longer opens from the menu: the agent opens it (tool `open_pack`, t-988), in a
// tab of its own: a pack tab for a library pack, a deck-pack tab for a deck-only one. The fake
// agent plays the `ouvre l'atelier` turn of fixtures/agent/e2e.json.
// A theme opens in its own tab, from the theme row of "Packs du deck" (openThemeTab).
const assert = require('node:assert/strict');
const { post, activeTab } = require('./http.cjs');
const { logOffset, waitForTool } = require('./agentLog.cjs');
const { SLOW, until, untilValue } = require('./until.cjs');

const view = (p) => post(p, '/api/workspace', { cmd: 'view' });

/** Waits for the active tab to be a pack tab (`data-kind="pack"`) showing its pile and "Terminer". */
async function packTabShown(p) {
  await p.locator('.tab.active[data-kind="pack"]').waitFor();
  await p.locator('.pile-column .pack-pile').waitFor();
  await p.getByRole('button', { name: 'Terminer' }).waitFor();
}
const snapshot = (p, tab) => post(p, '/api', { tab, req: { cmd: 'snapshot' } });

/**
 * Asks the agent of `tab` (default: the active tab) for the workshop, then waits for it: a new
 * active tab of kind pack, or deck_pack for a deck-only pack, never `tab` itself. Reloads the page
 * (serve pushes no view) and returns the tab that now edits the pack.
 */
async function openAtelier(p, tab) {
  const LOG = process.env.E2E_AGENT_LOG;
  assert.ok(LOG, 'E2E_AGENT_LOG manquant : lancer par just e2e');
  const before = tab ?? (await activeTab(p));
  const from = logOffset(LOG);
  const reply = await post(p, '/api/agent', { cmd: 'prompt', tab: before, text: "ouvre l'atelier", ui: {} });
  assert.equal(reply.error, undefined, `consigne « ouvre l'atelier » refusée : ${reply.error}`);
  await waitForTool(p, from, 'open_pack');
  let editing = null;
  await until(p, async () => {
    const win = (await view(p)).windows[0];
    const active = win.tabs.find((x) => x.id === win.active);
    if (active && active.id !== before && ['pack', 'deck_pack'].includes(active.document.kind)) editing = active.id;
    return editing !== null;
  }, `l'atelier ne s'ouvre pas depuis ${before}`, SLOW);
  await p.reload();
  await packTabShown(p);
  return editing;
}

/**
 * Closes every pack, deck-pack and theme tab over HTTP (idempotent): `t-1` active first and the
 * page reloaded, so no App of such a tab is mounted; a dirty one is saved first (save_pack or
 * save_theme: an HTTP-dirtied or inactive tab never autosaves), then closed with discard.
 * Reloads the page at the end.
 */
async function closePackTabs(p) {
  await post(p, '/api/workspace', { cmd: 'activate', tab: 't-1' });
  await p.reload();
  const tabs = (await view(p)).windows.flatMap((w) => w.tabs).filter((x) => ['pack', 'deck_pack', 'theme'].includes(x.document.kind));
  for (const { id, document } of tabs) {
    const save = document.kind !== 'theme' ? { cmd: 'save_pack', as_new: false } : { cmd: 'save_theme', as_new: false };
    if ((await snapshot(p, id)).dirty) await post(p, '/api', { tab: id, req: save });
    await untilValue(p, async () => (await snapshot(p, id)).dirty, (dirty) => dirty === false, `l'onglet ${document.kind} ${id} reste modifié`, 10_000);
    const closed = await post(p, '/api/workspace', { cmd: 'close', tab: id, discard: true });
    assert.equal(closed.error, undefined, `fermeture de l'onglet ${document.kind} ${id} refusée : ${closed.error}`);
  }
  await p.reload();
}

/**
 * Opens a theme tab from `tab` and returns it: over HTTP for `theme` (an id), else by the
 * "Ouvrir" button on the theme row of `tab`'s "Packs du deck" (opened from the toolbar). Waits
 * for an active theme tab, then reloads the page (serve pushes no view) and waits for its panel.
 * The packs panel stays in `tab`'s dock: `closePacksPanel` closes it once back there.
 */
async function openThemeTab(p, tab, theme = null) {
  if (theme) {
    const v = await post(p, '/api/workspace', { cmd: 'open_theme', tab, theme });
    assert.equal(v.error, undefined, `open_theme refusé : ${v.error}`);
  } else {
    await post(p, '/api/workspace', { cmd: 'activate', tab });
    await p.reload();
    await p.getByRole('button', { name: /^Packs du deck/ }).click();
    await p.locator('[data-surface-id="host-packs"] [data-component-id="theme-open"]').getByRole('button', { name: 'Ouvrir', exact: true }).click();
  }
  let opened = null;
  await until(p, async () => {
    const win = (await view(p)).windows[0];
    const active = win.tabs.find((x) => x.id === win.active);
    if (active && active.id !== tab && active.document.kind === 'theme') opened = active.id;
    return opened !== null;
  }, `aucun onglet de thème ne s'ouvre depuis ${tab}`, SLOW);
  await p.reload();
  await p.locator('.tab.active[data-kind="theme"]').waitFor();
  await p.locator('.pile-column .pile').waitFor();
  await p.waitForSelector('#theme-panel [data-surface-id="host-theme"]');
  return opened;
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
  await packTabShown(p);
  return tab;
}

/**
 * Closes the "Packs du deck" panel that `openThemeTab` left in the opener's dock, once back on the opener.
 * Later specs expect an empty dock.
 */
async function closePacksPanel(p) {
  const panel = p.locator('.agent-dock [data-surface-id="host-packs"]');
  await panel.waitFor();
  await panel.locator('[data-component-id="close"]').getByRole('button').click();
  await panel.waitFor({ state: 'detached' });
}

module.exports = { packTabShown, openAtelier, closePackTabs, openPackTab, openEssentiel, openThemeTab, closePacksPanel };
