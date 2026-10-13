//! t-809 : une chaîne forgée de milliers de niveaux dans l'arbre du deck
//! faisait déborder la pile dans Loro et avorter le processus. L'état d'un
//! snapshot reçu n'est plus lu : ses opérations sont rejouées, puis bornées.

use loro::{Container, ExportMode, LoroDoc, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};
use oku_slides_core::{Deck, DeckReplica, ReplicaErrorKind};
use speculoos::prelude::*;

use crate::common;

/// Profondeur maximale de l'arbre du deck (`replica::entry::MAX_TREE_DEPTH`).
const MAX_TREE_DEPTH: usize = 1024;
const SMALL: usize = 256 * 1024;
const APP_THREAD: usize = 2 << 20;
const BIG: usize = 1 << 30;

fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique ou débordement sur une pile de {size} o"))
}

fn slide_node(tree: &LoroTree, id: &str) -> TreeID {
    tree.roots()
        .into_iter()
        .find(|n| {
            tree.get_meta(*n)
                .is_ok_and(|m| matches!(m.get("id"), Some(ValueOrContainer::Value(LoroValue::String(s))) if *s == id))
        })
        .unwrap_or_else(|| panic!("slide {id} absente"))
}

/// Le snapshot de `roadmap` après `edit` sur son document brut ; construit sur une grande pile.
fn forged(edit: impl FnOnce(&LoroDoc, &LoroTree) + Send + 'static, shallow: bool) -> Vec<u8> {
    forged_as(edit, move |doc| {
        let heads = doc.oplog_frontiers();
        doc.export(if shallow { ExportMode::shallow_snapshot(&heads) } else { ExportMode::Snapshot })
    })
}

/// Comme [`forged`], exporté par `export` (snapshot, blob d'updates…).
fn forged_as(
    edit: impl FnOnce(&LoroDoc, &LoroTree) + Send + 'static,
    export: impl FnOnce(&LoroDoc) -> Result<Vec<u8>, loro::LoroEncodeError> + Send + 'static,
) -> Vec<u8> {
    on_stack(BIG, move || {
        let deck = Deck::from_json(include_str!("../../../../fixtures/roadmap.json")).unwrap_or_else(|e| panic!("{e}"));
        let r = DeckReplica::new(&deck, common::shared()).unwrap_or_else(|e| panic!("{e}"));
        let doc = LoroDoc::new();
        doc.import(&r.snapshot().unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
        let Some(ValueOrContainer::Container(Container::Tree(tree))) = doc.get_map("deck").get("slides") else {
            panic!("arbre du deck attendu")
        };
        edit(&doc, &tree);
        doc.commit();
        let bytes = export(&doc).unwrap_or_else(|e| panic!("{e}"));
        std::mem::forget(doc);
        bytes
    })
}

/// Une chaîne de `n` nœuds sans méta sous `parent` ; rend sa tête.
fn chain(tree: &LoroTree, parent: TreeParentId, n: usize) -> Option<TreeID> {
    let (mut at, mut head) = (parent, None);
    for _ in 0..n {
        let node = tree.create(at).unwrap_or_else(|e| panic!("{e}"));
        head.get_or_insert(node);
        at = TreeParentId::Node(node);
    }
    head
}

/// Ouvre `bytes` sur une pile de `stack` octets ; en cas de succès, enregistre puis libère sur cette pile.
fn open_on(stack: usize, bytes: Vec<u8>) -> Result<(), ReplicaErrorKind> {
    on_stack(stack, move || {
        let (r, _) = DeckReplica::from_snapshot(&bytes, 1 << 26, common::shared()).map_err(|e| e.kind())?;
        r.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
        drop(r);
        Ok(())
    })
}

#[test]
fn from_snapshot_deep_live_chain_is_refused_on_a_small_stack() {
    let bytes = forged(
        |_, tree| {
            chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), 20_000);
        },
        false,
    );
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_deep_chain_sent_as_an_updates_blob_is_refused_on_a_small_stack() {
    let bytes = forged_as(
        |_, tree| {
            chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), 20_000);
        },
        |doc| doc.export(ExportMode::all_updates()),
    );
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_deep_chain_under_a_deleted_head_is_refused_on_a_small_stack() {
    let bytes = forged(
        |_, tree| {
            if let Some(head) = chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), 20_000) {
                tree.delete(head).unwrap_or_else(|e| panic!("{e}"));
            }
        },
        false,
    );
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_with_a_deep_tree_outside_the_deck_is_refused_on_a_small_stack() {
    let bytes = forged(
        |doc, _| {
            chain(&doc.get_tree("autre"), TreeParentId::Root, 20_000);
        },
        false,
    );
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_shallow_is_refused() {
    let bytes = forged(|_, _| {}, true);
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_chain_at_the_bound_opens_saves_and_drops_on_an_app_thread() {
    // La slide est à la profondeur 1 : MAX - 1 niveaux finissent à la borne.
    let bytes = forged(
        |_, tree| {
            chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), MAX_TREE_DEPTH - 1);
        },
        false,
    );
    assert_that!(open_on(APP_THREAD, bytes)).is_ok();
}

#[test]
fn from_snapshot_one_level_past_the_bound_is_refused() {
    let bytes = forged(
        |_, tree| {
            chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), MAX_TREE_DEPTH);
        },
        false,
    );
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

#[test]
fn from_snapshot_without_deck_slides_and_a_deep_foreign_tree_is_refused() {
    let bytes = forged(
        |doc, _| {
            doc.get_map("deck").delete("slides").unwrap_or_else(|e| panic!("{e}"));
            chain(&doc.get_tree("autre"), TreeParentId::Root, 20_000);
        },
        false,
    );
    assert_that!(open_on(SMALL, bytes)).is_equal_to(Err(ReplicaErrorKind::Corrupted));
}

/// Témoin : le même chemin de forge, sans rien ajouter, s'ouvre sur une petite pile.
#[test]
fn from_snapshot_of_an_untouched_forge_opens_on_a_small_stack() {
    assert_that!(open_on(SMALL, forged(|_, _| {}, false))).is_ok();
}

/// À la borne, fusion, restauration et historique tiennent sur un fil de l'app (2 Mio).
#[test]
fn replica_at_the_bound_merges_restores_and_reads_history_on_an_app_thread() {
    let bytes = forged(
        |_, tree| {
            chain(tree, TreeParentId::Node(slide_node(tree, "s-constat")), MAX_TREE_DEPTH - 1);
        },
        false,
    );
    on_stack(APP_THREAD, move || {
        let open = || DeckReplica::from_snapshot(&bytes, 1 << 26, common::shared()).unwrap_or_else(|e| panic!("{e}")).0;
        let (mut mine, mut theirs) = (open(), open());
        let mut renamed = theirs.deck().0;
        renamed.slides[0].name = Some("renommée".to_owned());
        theirs.apply(&renamed, "Renommer", None).unwrap_or_else(|e| panic!("{e}"));
        mine.merge(&theirs).unwrap_or_else(|e| panic!("merge : {e}"));
        let history = mine.history();
        let first = history.deck_at(0).unwrap_or_else(|e| panic!("version : {e}")).0;
        mine.restore(&first, "Restaurer").unwrap_or_else(|e| panic!("restore : {e}"));
        mine.snapshot().unwrap_or_else(|e| panic!("snapshot : {e}"));
        drop((mine, theirs, history));
    });
}
