// Packs de widgets de la bibliothèque créés et supprimés par l'API HTTP d'un onglet (250-packs).
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');

const api = (p, tab, req) => post(p, '/api', { tab, req });

/**
 * Copies the built-in pack `base` under the name `name` into the library, in a pack tab opened from
 * `tab` then closed; the deck of `tab` does not pin the copy. Returns the new pack's id and
 * namespace, read in the widget pack list.
 */
async function createLibraryWidgetPack(p, tab, name) {
  const base = (await api(p, tab, { cmd: 'packs', kind: 'widgets' })).find((pack) => pack.namespace === 'base');
  assert.ok(base, 'la bibliothèque n\'a pas le pack de widgets base');
  const view = await post(p, '/api/workspace', { cmd: 'open_pack', tab, pack: base.id, widget: null });
  const packTab = view.windows?.flatMap((w) => w.tabs).find((t) => t.document.kind === 'pack' && t.document.pack === base.id)?.id;
  try {
    assert.ok(packTab, `open_pack refusé : ${JSON.stringify(view).slice(0, 300)}`);
    const editing = await api(p, packTab, { cmd: 'snapshot' });
    assert.equal(editing.editing?.kind, 'widgets', `l'onglet de pack n'édite pas base : ${JSON.stringify(editing).slice(0, 300)}`);
    // The pack's name is its deck's title; the copy derives its namespace from it.
    const ops = [{ op: 'set_deck_param', key: 'title', value: name }];
    const renamed = await api(p, packTab, { cmd: 'apply', label: 'Nom', ops, merge: null });
    assert.equal(renamed.error, undefined, `renommage refusé : ${renamed.error}`);
    const saved = await api(p, packTab, { cmd: 'save_pack', as_new: true });
    assert.equal(saved.error, undefined, `save_pack refusé : ${saved.error}`);
  } finally {
    if (packTab !== undefined) await post(p, '/api/workspace', { cmd: 'close', tab: packTab, discard: true });
  }
  const made = (await api(p, tab, { cmd: 'packs', kind: 'widgets' })).find((pack) => pack.name === name);
  assert.ok(made, `le pack « ${name} » manque à la bibliothèque`);
  // Saved in the pack tab, the copy is not pinned by the deck: the pack is still to add.
  assert.equal((await api(p, tab, { cmd: 'snapshot' })).widgets[made.namespace], undefined, 'le deck épingle la copie');
  return { id: made.id, namespace: made.namespace };
}

/** Supprime le pack `id` de la bibliothèque, toutes versions comprises. */
async function deleteLibraryPack(p, tab, id) {
  const reply = await api(p, tab, { cmd: 'delete_pack', pack: id });
  assert.ok(Array.isArray(reply), `delete_pack refusé : ${JSON.stringify(reply).slice(0, 300)}`);
}

module.exports = { createLibraryWidgetPack, deleteLibraryPack };
