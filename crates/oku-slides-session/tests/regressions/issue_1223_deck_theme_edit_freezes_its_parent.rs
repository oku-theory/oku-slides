//! t-1223: a deck-mode theme edit built on a theme tab's unfrozen draft.
//!
//! A theme tab saves its draft `v`, then sleeps without its lease (a restart).
//! A deck-mode `edit_theme` writes `w` on top of `v` without freezing it. The
//! resumed tab's next save replaced `v` in place: `w`'s parent left the library.

use oku_slides_core::Op;
use oku_slides_session::{Session, ThemeLibrary, TokenOp};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{add_user_theme, data_dir, session_in};

/// The theme library of the data folder `data`.
fn themes(data: &std::path::Path) -> ThemeLibrary {
    let storage = oku_slides_session::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    ThemeLibrary::new(Some(data.join("themes")), storage)
}

/// Sets the theme tab's `color.accent` to `value`, then saves the draft: the saved version.
fn save_accent(tab: &mut Session, value: &str) -> oku_slides_core::PackVersion {
    let op = Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!(value) };
    tab.apply("accent", vec![op], None).unwrap_or_else(|e| panic!("jeton : {}", e.one_line()));
    let saved = tab.save_theme(false).unwrap_or_else(|e| panic!("enregistrement : {}", e.one_line()));
    saved.unwrap_or_else(|| panic!("brouillon inchangé")).version
}

#[test]
fn issue_1223_tab_save_keeps_the_parent_of_a_deck_theme_edit() {
    let data = data_dir();
    add_user_theme(&data, "maison", &[], json!({"color": {"accent": "#204060"}}));
    let dir = data.join("tab");
    let id = session_in(&data).find_theme_info("maison").unwrap_or_else(|e| panic!("{}", e.one_line())).id;
    let mut tab = session_in(&data);
    tab.open_theme_here(id).unwrap_or_else(|e| panic!("onglet : {}", e.one_line()));
    assert_that!(tab.resume(dir.clone())).is_true();
    let draft = save_accent(&mut tab, "#111111");
    // The app quits: the tab's state is written, its lease drops.
    drop(tab);
    let mut deck = session_in(&data);
    let ops = vec![TokenOp::SetToken { path: vec!["color".into(), "surface".into()], value: json!("#222222") }];
    let edited = deck.edit_theme(id, None, ops).unwrap_or_else(|e| panic!("édition : {}", e.one_line()));
    drop(deck);
    let mut back = session_in(&data);
    assert_that!(back.resume(dir)).is_true();
    let resaved = save_accent(&mut back, "#333333");
    let library = themes(&data);
    let listed: Vec<_> = library.versions(id).into_iter().map(|v| v.version).collect();
    let parent_of = |v: &oku_slides_core::PackVersion| {
        library.versions(id).into_iter().find(|s| s.version == *v).and_then(|s| s.manifest.parent)
    };
    assert_that!(listed).contains(&edited.version);
    assert_that!(parent_of(&edited.version)).is_equal_to(Some(draft.clone()));
    assert_that!(listed).contains(&draft);
    // The tab built on its frozen draft: a sibling of the deck's edit, not a replacement.
    assert_that!(parent_of(&resaved)).is_equal_to(Some(draft));
}
