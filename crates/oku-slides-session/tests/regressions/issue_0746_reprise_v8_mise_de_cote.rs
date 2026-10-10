//! t-746 : un `deck.loro` de reprise à la forme v8 (slides en liste, arbre
//! `@elements` par slide, `meta.format = 2`) n'a pas la forme du format 3.
//! Il est refusé, et le dossier mis de côté.

use speculoos::prelude::*;

use crate::common::{data_dir, session_in, set_theme};

/// Un replica à la forme v8, comme l'écrivait le format 2.
fn v8_shaped() -> Vec<u8> {
    let doc = loro::LoroDoc::new();
    let ok = |r: Result<(), loro::LoroError>| r.unwrap_or_else(|e| panic!("forge : {e}"));
    ok(doc.get_map("meta").insert("deck_uid", "uid-v8"));
    ok(doc.get_map("meta").insert("format", 2_i64));
    let deck = doc.get_map("deck");
    ok(deck.insert("version", 5_i64));
    let slides =
        deck.insert_container("slides", loro::LoroMovableList::new()).unwrap_or_else(|e| panic!("slides : {e}"));
    let slide = slides.push_container(loro::LoroMap::new()).unwrap_or_else(|e| panic!("slide : {e}"));
    ok(slide.insert("id", "s"));
    ok(slide.insert("kind", "content"));
    let tree = slide.insert_container("@elements", loro::LoroTree::new()).unwrap_or_else(|e| panic!("arbre : {e}"));
    let node = tree.create(loro::TreeParentId::Root).unwrap_or_else(|e| panic!("nœud : {e}"));
    let meta = tree.get_meta(node).unwrap_or_else(|e| panic!("méta : {e}"));
    ok(meta.insert("id", "t"));
    doc.commit();
    doc.export(loro::ExportMode::Snapshot).unwrap_or_else(|e| panic!("export : {e}"))
}

#[test]
fn resume_of_a_v8_replica_moves_the_folder_aside() {
    let data = data_dir();
    let resume_dir = data.join("resume");

    // Une session écrit une reprise, avec une édition.
    let mut a = session_in(&data);
    assert_that!(a.resume(resume_dir.clone())).is_true();
    a.handle(set_theme("pro")).unwrap_or_else(|e| panic!("édition impossible : {}", e.one_line()));
    drop(a);

    // Son replica est réécrit à la forme v8.
    let loro_path = resume_dir.join("deck/deck.loro");
    let forged = v8_shaped();
    std::fs::write(&loro_path, &forged).unwrap_or_else(|e| panic!("écriture impossible : {e}"));

    let mut b = session_in(&data);
    assert_that!(b.resume(resume_dir.clone())).is_false();

    let moved: Vec<_> = std::fs::read_dir(&*data)
        .unwrap_or_else(|e| panic!("lecture du dossier de données : {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("resume.failed-")))
        .collect();
    assert_that!(moved.len()).is_equal_to(1);
    let kept = std::fs::read(moved[0].join("deck/deck.loro")).unwrap_or_else(|e| panic!("deck.loro perdu : {e}"));
    assert_that!(kept).is_equal_to(forged);
    // Rien n'est écrit à la place.
    assert_that!(resume_dir.exists()).is_false();
}
