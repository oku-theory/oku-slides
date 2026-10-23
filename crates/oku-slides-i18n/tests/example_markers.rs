//! Chaque marqueur `ftl:` des manifestes des packs intégrés a sa clé dans chaque langue livrée.

use oku_slides_i18n::{Localizer, available};
use speculoos::prelude::*;

/// Les manifestes livrés des packs intégrés, `base` et `essentiel`.
const MANIFESTS: [&str; 2] = [
    include_str!("../../../typst/oku-slides/packs/base/widgets.json"),
    include_str!("../../../typst/oku-slides/packs/essentiel/layouts.json"),
];

/// Les clés des marqueurs `"ftl:<clé>"` d'un manifeste, dans l'ordre.
fn markers(manifest: &str) -> Vec<&str> {
    manifest.split("\"ftl:").skip(1).filter_map(|rest| rest.split('"').next()).collect()
}

#[test]
fn builtin_manifests_hold_markers() {
    let counts: Vec<usize> = MANIFESTS.iter().map(|m| markers(m).len()).collect();
    assert_that!(counts.iter().all(|n| *n > 0)).is_true();
}

#[test]
fn every_example_marker_has_its_key_in_every_locale() {
    let mut missing = Vec::new();
    for locale in available() {
        let l = Localizer::new(Some(locale));
        for key in MANIFESTS.iter().flat_map(|m| markers(m)) {
            if !key.starts_with("session-example-") || !l.has(key) {
                missing.push(format!("{locale}: {key}"));
            }
        }
    }
    assert_that!(missing).is_empty();
}
