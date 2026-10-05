//! Pack de widgets des tests de rendu : la frise de test (`fixtures/widgets/frise-test.typ`)
//! et le rendu d'un deck d'une slide avec un pack.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Deck, WidgetFile};
use oku_slides_render::{Compiled, Fonts, Layout, PackWidgets, Renderer, load_registry};
use serde_json::json;

/// Source du widget de pack `frise` : liste `etapes` (date, texte), quatre étapes par défaut.
pub const FRISE_TEST: &str = include_str!("../../../../fixtures/widgets/frise-test.typ");

/// Un pack `id` aux fichiers `files` (nom, source).
pub fn pack(id: &str, files: &[(&str, &str)]) -> PackWidgets {
    let files = files
        .iter()
        .map(|(n, s)| (WidgetFile::parse(n).unwrap_or_else(|| panic!("nom de widget invalide : {n}")), (*s).to_owned()))
        .collect();
    PackWidgets::new(id, files).unwrap_or_else(|e| panic!("{e}"))
}

/// Le pack de test qui porte la frise.
pub fn frise_pack() -> PackWidgets {
    pack("frise-test", &[("frise", FRISE_TEST)])
}

/// Rend un deck d'une slide `content` dont les blocs sont `blocks`, avec les widgets de `pack`.
pub fn compile_with_pack(pack: &PackWidgets, blocks: serde_json::Value) -> Compiled {
    let registry = Arc::new(load_registry(&Fonts::default(), Some(pack)).unwrap_or_else(|e| panic!("{e}")));
    let mut r = Renderer::new(Fonts::default()).unwrap_or_else(|e| panic!("{e}"));
    r.set_widgets(Some(pack), registry).unwrap_or_else(|e| panic!("{e}"));
    let deck = json!({
        "version": 3, "params": { "title": "T" },
        "slides": [{ "id": "s", "kind": "content", "params": { "blocks": blocks } }]
    });
    let deck = Deck::from_json(&deck.to_string()).unwrap_or_else(|e| panic!("deck invalide : {e}"));
    r.render(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"))
}

/// Comme [`compile_with_pack`], pour la seule mise en page.
pub fn render_with_pack(pack: &PackWidgets, blocks: serde_json::Value) -> Layout {
    compile_with_pack(pack, blocks).layout().clone()
}
