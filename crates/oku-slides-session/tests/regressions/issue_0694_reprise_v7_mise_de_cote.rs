//! t-694 : un `deck.loro` de reprise écrit avant `meta.format` (replica v7,
//! éléments en listes) se lisait sans élément, puis la prochaine écriture
//! effaçait ses slots. Il est désormais refusé, et le dossier mis de côté.

use speculoos::prelude::*;

use crate::common::{data_dir, session_in, set_theme};

/// Le snapshot `bytes` sans `meta.format`, comme l'écrivait le format v7.
fn without_format(bytes: &[u8]) -> Vec<u8> {
    let doc = loro::LoroDoc::new();
    doc.import(bytes).unwrap_or_else(|e| panic!("snapshot illisible : {e}"));
    doc.get_map("meta").delete("format").unwrap_or_else(|e| panic!("meta.format : {e}"));
    doc.commit();
    doc.export(loro::ExportMode::Snapshot).unwrap_or_else(|e| panic!("export : {e}"))
}

#[test]
fn resume_of_a_v7_replica_moves_the_folder_aside() {
    let data = data_dir();
    let resume_dir = data.join("resume");

    // Une session écrit une reprise, avec une édition.
    let mut a = session_in(&data);
    assert_that!(a.resume(resume_dir.clone())).is_true();
    a.handle(set_theme("pro")).unwrap_or_else(|e| panic!("édition impossible : {}", e.one_line()));
    drop(a);

    // Son replica est réécrit comme en v7.
    let loro_path = resume_dir.join("deck/deck.loro");
    let forged = without_format(&std::fs::read(&loro_path).unwrap_or_else(|e| panic!("deck.loro absent : {e}")));
    std::fs::write(&loro_path, &forged).unwrap_or_else(|e| panic!("écriture impossible : {e}"));

    let mut b = session_in(&data);
    assert_that!(b.resume(resume_dir.clone())).is_false();

    let moved: Vec<_> = std::fs::read_dir(&*data)
        .unwrap_or_else(|e| panic!("lecture du dossier de données : {e}"))
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("resume.failed-")))
        .collect();
    assert_that!(moved.len()).is_equal_to(1);
    let kept = std::fs::read(moved[0].join("deck/deck.loro")).unwrap_or_else(|e| panic!("deck.loro perdu : {e}"));
    assert_that!(kept).is_equal_to(forged);
    // Rien n'est écrit à la place.
    assert_that!(resume_dir.exists()).is_false();
}
