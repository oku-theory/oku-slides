//! Fichiers de widgets hostiles et deck minimal, pour les tests.
//!
//! Autonome (ne dépend que de `oku_slides_core`, `oku_slides_render` et
//! `serde_json`) : les tests d'intégration l'incluent par `#[path]`.

#![allow(dead_code, reason = "partagé : chaque binaire de test n'en utilise qu'une partie")]

/// Boucle au chargement du fichier `boucle` (sonde et `SetWidgets`), hors des gardes de Typst.
///
/// Deux `for` imbriqués sur des tableaux courts : Typst borne les `while`
/// (boucle jugée infinie), pas les `for`, et `range` y reste petit.
pub(crate) const LOOP_ON_IMPORT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let _spin = { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; n }
#let widget = widget("boucle", params: (body: p.plain()), (args, ctx) => [])
"#;

/// Le layout `boucle` (pack de layouts) : boucle au rendu de la slide seulement.
///
/// Un layout et non un élément : seul dans son emplacement, il est rendu sans
/// autre pack (un élément demande un layout qui le porte).
pub(crate) const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", category: "slide", params: (body: p.plain()),
  (args, ctx) => { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; [#n] })
"#;

/// Le fichier `gros` : alloue sans fin au chargement.
pub(crate) const ALLOCATE: &str = r#"#import "/oku-slides/lib.typ": widget
#let _big = range(400000000).map(i => str(i))
#let widget = widget("gros", (args, ctx) => [])
"#;

/// Le fichier `quote`, valide.
pub(crate) const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich()), (args, ctx) => field(ctx, "body", args.body))
"#;

/// Des fichiers de widgets, par nom.
pub(crate) fn files(entries: &[(&str, &str)]) -> std::collections::BTreeMap<oku_slides_core::WidgetFile, String> {
    entries
        .iter()
        .map(|(n, s)| {
            let name = oku_slides_core::WidgetFile::parse(n).unwrap_or_else(|| panic!("nom invalide : {n}"));
            (name, (*s).to_owned())
        })
        .collect()
}

/// Un deck d'une seule diapositive de titre (`essentiel/title`).
pub(crate) fn tiny_deck() -> oku_slides_core::Deck {
    deck_of(serde_json::json!({ "id": "s", "kind": oku_slides_core::kinds::essentiel::TITLE, "params": {} }))
}

/// Un deck d'une seule diapositive `slide`.
pub(crate) fn deck_of(slide: serde_json::Value) -> oku_slides_core::Deck {
    let json =
        serde_json::json!({ "version": oku_slides_core::DECK_VERSION, "params": { "title": "T" }, "slides": [slide] });
    oku_slides_core::Deck::from_json(&json.to_string()).expect("deck minimal")
}

/// Un deck d'une seule diapositive au layout `rendu/boucle` ([`LOOP_ON_RENDER`]), qui boucle au rendu.
pub(crate) fn looping_deck() -> oku_slides_core::Deck {
    deck_of(serde_json::json!({ "id": "s", "kind": "rendu/boucle", "params": { "body": "x" } }))
}

/// Le pack de layouts `rendu`, au seul layout `boucle` qui boucle au rendu.
pub(crate) fn looping_layouts() -> oku_slides_render::PackWidgets {
    let files = files(&[("boucle", LOOP_ON_RENDER)]);
    oku_slides_render::PackWidgets::new("rendu", ns("rendu"), oku_slides_core::PackKind::Layouts, files).expect("pack")
}

/// L'espace de noms `name` ; panique s'il est invalide (test).
pub(crate) fn ns(name: &str) -> oku_slides_core::Namespace {
    oku_slides_core::Namespace::parse(name).unwrap_or_else(|| panic!("espace de noms invalide : {name}"))
}
