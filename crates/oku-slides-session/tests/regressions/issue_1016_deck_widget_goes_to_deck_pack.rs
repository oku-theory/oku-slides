//! t-1016 : un widget écrit en mode deck sans pack nommé va au pack propre du deck.
//!
//! Avant t-1016, un deck n'avait qu'un pack : écrire un widget en mode deck
//! faisait une nouvelle version de son pack, le pack intégré compris. Le
//! pack intégré du deck ne change plus : le widget va au pack `deck`, du
//! deck seul, créé à la première écriture.

use oku_slides_core::Namespace;
use oku_slides_session::{ESSENTIEL_PACK, PackLibrary, PackScope, Storage};
use speculoos::prelude::*;

use crate::common::{data_dir, fixture_path, session_in};

/// Le widget d'élément `note`.
const NOTE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("note", params: (body: p.plain()), (args, ctx) => [#args.body])
"#;

#[test]
fn widget_written_in_deck_mode_without_a_pack_goes_to_the_deck_pack() {
    let data = data_dir();
    let mut s = session_in(data.as_ref());
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let essentiel = Namespace::parse("essentiel").unwrap_or_else(|| panic!("espace de noms"));
    let before = s.deck().widgets.get(&essentiel).cloned();

    let change = s.set_widget(None, "note", NOTE.into()).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(change.saved).is_true();

    // Le pack intégré du deck n'a pas changé, ni dans le deck ni dans la bibliothèque.
    assert_that!(s.deck().widgets.get(&essentiel).cloned()).is_equal_to(before);
    let library = PackLibrary::new(
        Some(data.as_ref().join("widgets")),
        None,
        crate::common::no_user_themes(),
        Storage::local().unwrap_or_else(|e| panic!("{e}")),
    );
    assert_that!(library.versions(ESSENTIEL_PACK).len()).is_equal_to(1);
    // Le widget est au pack `deck`, du deck seul.
    let pin = s.deck().widgets.get(&Namespace::deck()).cloned().unwrap_or_else(|| panic!("pack du deck"));
    let files = s.sources_for_tests().packs.get(&pin.version).cloned().unwrap_or_else(|| panic!("version embarquée"));
    let scope = files.scope().unwrap_or_else(|e| panic!("{e}"));
    assert_that!(scope).is_equal_to(PackScope::Deck);
    assert_that!(s.widget_source(None, "note").unwrap_or_else(|e| panic!("{}", e.one_line())))
        .is_equal_to(NOTE.to_owned());
}
