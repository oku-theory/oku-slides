//! Sonde t-809 : comment Loro 1.16.2 traite une chaîne profonde, et pourquoi
//! l'ouverture rejoue les opérations au lieu de lire l'état reçu.
//!
//! Mesures du 2026-10-11 (release, pile de 256 Kio, 100 000 niveaux) :
//! un snapshot importé récurse au premier accès, dans `is_node_deleted`, à
//! l'export, au `fork` et au drop ; un document **rejoué** (updates importées
//! d'un document dont l'état n'est jamais lu) ne récurse ni au premier accès,
//! ni en remontant `parent()`, ni au drop — seul l'export d'un arbre profond
//! déborde ; une version passée profonde relue par `updates_till` passe ;
//! 20 000 cartes imbriquées rejouées passent l'export, le `fork` et le drop ;
//! un snapshot *shallow* exporte des updates tronquées (l'état d'avant la
//! coupe est perdu au rejeu). Coût du rejeu : 100 000
//! ops 60 ms (import direct 4 ms), 1 M ops 0,8 s (5 ms).
//!
//! Les tests sans `#[ignore]` vérifient ces faits sur une pile réduite ; les
//! chaînes sont construites sur une grande pile (leur export récurse).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

use std::process::Command;

use loro::{Container, ExportMode, LoroDoc, LoroMap, LoroTree, TreeParentId, ValueOrContainer};
use speculoos::prelude::*;

const N: usize = 100_000;

fn on_stack<T: Send + 'static>(size: usize, f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(size)
        .spawn(f)
        .unwrap_or_else(|e| panic!("fil : {e}"))
        .join()
        .unwrap_or_else(|_| panic!("panique ou débordement sur une pile de {size} o"))
}

fn deck_tree(doc: &LoroDoc) -> LoroTree {
    match doc.get_map("deck").get("slides") {
        Some(ValueOrContainer::Container(Container::Tree(t))) => t,
        other => panic!("arbre attendu : {other:?}"),
    }
}

/// Snapshot d'une chaîne de `n` niveaux sans méta ; tête supprimée si `delete_head`.
fn chain(n: usize, delete_head: bool) -> Vec<u8> {
    on_stack(1 << 30, move || {
        let doc = LoroDoc::new();
        let tree = doc.get_map("deck").insert_container("slides", LoroTree::new()).unwrap_or_else(|e| panic!("{e}"));
        let mut parent = TreeParentId::Root;
        let mut head = None;
        for _ in 0..n {
            let node = tree.create(parent).unwrap_or_else(|e| panic!("{e}"));
            head.get_or_insert(node);
            parent = TreeParentId::Node(node);
        }
        if let (true, Some(h)) = (delete_head, head) {
            tree.delete(h).unwrap_or_else(|e| panic!("{e}"));
        }
        doc.commit();
        let bytes = doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
        std::mem::forget(doc);
        bytes
    })
}

/// Le document rejoué des opérations de `bytes` ; l'état reçu n'est jamais lu.
fn replayed(bytes: &[u8]) -> LoroDoc {
    let received = LoroDoc::new();
    received.import(bytes).unwrap_or_else(|e| panic!("{e}"));
    let updates = received.export(ExportMode::all_updates()).unwrap_or_else(|e| panic!("{e}"));
    drop(received);
    let doc = LoroDoc::new();
    doc.import(&updates).unwrap_or_else(|e| panic!("{e}"));
    doc
}

/// Profondeur d'une chaîne : remontée depuis le nœud créé en dernier (linéaire).
fn depth(tree: &LoroTree) -> usize {
    let Some(mut at) = tree.nodes().into_iter().max_by_key(|n| n.counter) else { return 0 };
    let mut d = 1;
    while let Some(TreeParentId::Node(p)) = tree.parent(at) {
        at = p;
        d += 1;
    }
    d
}

#[test]
fn replayed_deep_chain_is_measured_and_dropped_on_a_small_stack() {
    for delete_head in [false, true] {
        let bytes = chain(N, delete_head);
        let d = on_stack(256 * 1024, move || {
            let doc = replayed(&bytes);
            let d = depth(&deck_tree(&doc));
            drop(doc);
            d
        });
        assert_that!(d).named(&format!("tête supprimée : {delete_head}")).is_equal_to(N);
    }
}

#[test]
fn replayed_past_version_of_a_flattened_chain_is_read_on_a_small_stack() {
    // Les octets, et la version juste après la chaîne (avant toute remontée).
    let (bytes, chained) = on_stack(1 << 30, || {
        let doc = LoroDoc::new();
        let tree = doc.get_map("deck").insert_container("slides", LoroTree::new()).unwrap_or_else(|e| panic!("{e}"));
        let mut parent = TreeParentId::Root;
        let mut nodes = Vec::new();
        for _ in 0..N {
            let node = tree.create(parent).unwrap_or_else(|e| panic!("{e}"));
            nodes.push(node);
            parent = TreeParentId::Node(node);
        }
        doc.commit();
        let chained = doc.oplog_frontiers();
        for n in nodes.iter().rev() {
            tree.mov(*n, TreeParentId::Root).unwrap_or_else(|e| panic!("{e}"));
        }
        doc.commit();
        let bytes = doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
        std::mem::forget(doc);
        (bytes, chained)
    });
    let d = on_stack(256 * 1024, move || {
        let doc = replayed(&bytes);
        // Les ID survivent au rejeu : la version se retrouve par sa frontière.
        let vv = doc.frontiers_to_vv(&chained).unwrap_or_else(|| panic!("version inconnue"));
        let version = LoroDoc::new();
        version
            .import(&doc.export(ExportMode::updates_till(&vv)).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
        let d = depth(&deck_tree(&version));
        drop(version);
        drop(doc);
        d
    });
    assert_that!(d).is_equal_to(N);
}

#[test]
fn replayed_nested_maps_export_fork_and_drop_on_a_small_stack() {
    let bytes = on_stack(1 << 30, || {
        let doc = LoroDoc::new();
        let mut m: LoroMap =
            doc.get_map("deck").insert_container("x", LoroMap::new()).unwrap_or_else(|e| panic!("{e}"));
        for _ in 0..20_000 {
            m = m.insert_container("x", LoroMap::new()).unwrap_or_else(|e| panic!("{e}"));
        }
        doc.commit();
        let bytes = doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}"));
        std::mem::forget(doc);
        bytes
    });
    on_stack(256 * 1024, move || {
        let doc = replayed(&bytes);
        drop(doc.export(ExportMode::Snapshot).unwrap_or_else(|e| panic!("{e}")));
        drop(doc.fork());
        drop(doc);
    });
}

#[test]
fn shallow_snapshot_is_detected_and_its_replayed_updates_lose_the_cut_state() {
    let doc = LoroDoc::new();
    let deck = doc.get_map("deck");
    deck.insert("k", 1).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    deck.insert("j", 2).unwrap_or_else(|e| panic!("{e}"));
    doc.commit();
    let bytes = doc.export(ExportMode::shallow_snapshot(&doc.oplog_frontiers())).unwrap_or_else(|e| panic!("{e}"));
    let received = LoroDoc::new();
    received.import(&bytes).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(received.is_shallow()).is_true();
    let updates = received.export(ExportMode::all_updates()).unwrap_or_else(|e| panic!("{e}"));
    let replayed = LoroDoc::new();
    replayed.import(&updates).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(received.get_map("deck").get("k").is_some()).is_true();
    assert_that!(replayed.get_map("deck").get("k").is_none()).is_true();
}

/// Enfant : importe `PROBE_FILE` et y accède sur une pile de 256 Kio (sans rejeu).
#[test]
#[ignore = "lancé par la mesure, dans un processus enfant"]
fn probe_child_reads_a_received_snapshot() {
    let file = std::env::var("PROBE_FILE").unwrap_or_else(|e| panic!("{e}"));
    let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{e}"));
    on_stack(256 * 1024, move || {
        let doc = LoroDoc::new();
        doc.import(&bytes).unwrap_or_else(|e| panic!("{e}"));
        let _ = deck_tree(&doc).nodes();
        std::mem::forget(doc);
    });
}

/// Mesure : lire l'état d'un snapshot reçu déborde (d'où le rejeu).
#[test]
#[ignore = "mesure"]
fn measure_received_snapshot_first_access_overflows() {
    let path = std::env::temp_dir().join(format!("oku-depth-probe-{}.loro", std::process::id()));
    std::fs::write(&path, chain(N, false)).unwrap_or_else(|e| panic!("{e}"));
    let exe = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
    let ok = Command::new(exe)
        .args(["probe_child_reads_a_received_snapshot", "--exact", "--ignored", "--test-threads=1"])
        .env("PROBE_FILE", &path)
        .output()
        .is_ok_and(|o| o.status.success());
    std::fs::remove_file(&path).ok();
    println!("premier accès d'un snapshot reçu (100 000 niveaux, 256 Kio) : {}", if ok { "OK" } else { "déborde" });
}
