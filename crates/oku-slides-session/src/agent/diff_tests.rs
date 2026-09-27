use oku_slides_a2ui::{DiffKind, DiffSegment};
use speculoos::prelude::*;

use super::diff_words;

fn seg(kind: DiffKind, text: &str) -> DiffSegment {
    DiffSegment { kind, text: text.into() }
}

#[test]
fn diff_words_replaced_word_gives_del_then_ins() {
    assert_that!(diff_words("Nos objectifs", "Nos paris")).is_equal_to(vec![
        seg(DiffKind::Keep, "Nos "),
        seg(DiffKind::Del, "objectifs"),
        seg(DiffKind::Ins, "paris"),
    ]);
}

#[test]
fn diff_words_identical_text_is_one_keep() {
    assert_that!(diff_words("Même texte", "Même texte")).is_equal_to(vec![seg(DiffKind::Keep, "Même texte")]);
}

#[test]
fn diff_words_merges_neighbouring_segments_of_same_kind() {
    // La tokenisation mot à mot de `similar` traite ici l'espace du milieu comme
    // un segment `Keep` séparé (contrairement au préfixe commun du test
    // précédent, fusionné avec le mot qui le précède) : deux paires Del/Ins de
    // part et d'autre, jamais deux segments voisins de même nature.
    let segments = diff_words("a b", "x y");
    assert_that!(segments.windows(2).all(|w| w[0].kind != w[1].kind)).is_true();
    assert_that!(segments.iter().filter(|s| s.kind == DiffKind::Del).count()).is_equal_to(2);
    assert_that!(segments.iter().filter(|s| s.kind == DiffKind::Ins).count()).is_equal_to(2);
}
