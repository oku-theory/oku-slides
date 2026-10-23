// Packs de widgets de la bibliothèque créés et supprimés par l'API HTTP d'un onglet (250-packs).
const assert = require('node:assert/strict');
const { post } = require('./http.cjs');

const api = (p, tab, req) => post(p, '/api', { tab, req });

/**
 * Copie le pack intégré `base` sous le nom `name` dans la bibliothèque, puis rend l'onglet `tab`
 * à son deck, sans la copie épinglée. Rend l'id et l'espace de noms du nouveau pack, lus dans la liste des packs de widgets.
 */
async function createLibraryWidgetPack(p, tab, name) {
  const base = (await api(p, tab, { cmd: 'packs', kind: 'widgets' })).find((pack) => pack.namespace === 'base');
  assert.ok(base, 'la bibliothèque n\'a pas le pack de widgets base');
  const editing = await api(p, tab, { cmd: 'edit_pack', pack: base.id });
  assert.equal(editing.editing?.kind, 'widgets', `edit_pack refusé : ${JSON.stringify(editing).slice(0, 300)}`);
  try {
    // Le nom du pack est le titre du deck du pack ; la copie en dérive son espace de noms.
    const ops = [{ op: 'set_deck_param', key: 'title', value: name }];
    const renamed = await api(p, tab, { cmd: 'apply', label: 'Nom', ops, merge: null });
    assert.equal(renamed.error, undefined, `renommage refusé : ${renamed.error}`);
    const saved = await api(p, tab, { cmd: 'save_pack', as_new: true });
    assert.equal(saved.error, undefined, `save_pack refusé : ${saved.error}`);
  } finally {
    await api(p, tab, { cmd: 'close_pack' });
  }
  const made = (await api(p, tab, { cmd: 'packs', kind: 'widgets' })).find((pack) => pack.name === name);
  assert.ok(made, `le pack « ${name} » manque à la bibliothèque`);
  // La copie enregistrée est épinglée par le deck mis de côté : il la lâche, le pack reste à ajouter.
  if ((await api(p, tab, { cmd: 'snapshot' })).widgets[made.namespace]) {
    const ops = [{ op: 'remove_pack', kind: 'widgets', namespace: made.namespace }];
    const removed = await api(p, tab, { cmd: 'apply', label: 'Retrait', ops, merge: null });
    assert.equal(removed.error, undefined, `retrait de la copie refusé : ${removed.error}`);
  }
  return { id: made.id, namespace: made.namespace };
}

/** Supprime le pack `id` de la bibliothèque, toutes versions comprises. */
async function deleteLibraryPack(p, tab, id) {
  const reply = await api(p, tab, { cmd: 'delete_pack', pack: id });
  assert.ok(Array.isArray(reply), `delete_pack refusé : ${JSON.stringify(reply).slice(0, 300)}`);
}

module.exports = { createLibraryWidgetPack, deleteLibraryPack };
