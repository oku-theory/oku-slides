//! Fichiers de widgets hostiles et deck minimal, pour les tests.
//!
//! Autonome (ne dépend que de `oku_slides_core`) : les tests d'intégration
//! l'incluent par `#[path]`.

#![allow(dead_code, reason = "partagé : chaque binaire de test n'en utilise qu'une partie")]

/// Boucle au chargement du fichier `boucle` (sonde et `SetWidgets`), hors des gardes de Typst.
///
/// Deux `for` imbriqués sur des tableaux courts : Typst borne les `while`
/// (boucle jugée infinie), pas les `for`, et `range` y reste petit.
pub(crate) const LOOP_ON_IMPORT: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let _spin = { let n = 0; for _ in range(100000) { for _ in range(100000) { n += 1 } }; n }
#let widget = widget("boucle", params: (body: p.plain()), (args, ctx) => [])
"#;

/// Le fichier `boucle` : boucle au rendu du widget seulement.
pub(crate) const LOOP_ON_RENDER: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("boucle", params: (body: p.plain()),
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

/// Un deck d'une seule diapositive de titre.
pub(crate) fn tiny_deck() -> oku_slides_core::Deck {
    let json = r#"{"version":4,"params":{"title":"T"},"slides":[{"id":"s","kind":"title","params":{}}]}"#;
    oku_slides_core::Deck::from_json(json).expect("deck minimal")
}
