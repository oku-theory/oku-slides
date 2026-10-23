//! t-1111 : les exemples des layouts d'Essentiel étaient écrits en français dans son manifeste,
//! et le sélecteur de layouts les montrait tels quels dans toutes les langues.

use oku_slides_session::{Fonts, Platform, Session, Settings};
use rstest::rstest;
use speculoos::prelude::*;

use crate::common::data_dir;

/// Une session neuve sur `data`, dans la langue `locale`.
fn session_speaking(data: &std::path::Path, locale: &str) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some(locale.into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {}", e.one_line()))
}

#[rstest]
#[case("en", "Thank you")]
#[case("fr", "Merci")]
fn layout_picker_end_example_follows_the_ui_language(#[case] locale: &str, #[case] expected: &str) {
    let data = data_dir();
    let mut s = session_speaking(&data, locale);
    let view = serde_json::to_value(s.layouts_view().unwrap_or_else(|e| panic!("layouts : {}", e.one_line())))
        .unwrap_or_else(|e| panic!("vue : {e}"));
    let layouts = view["packs"].as_array().into_iter().flatten().flat_map(|p| p["layouts"].as_array()).flatten();
    let end = layouts.into_iter().find(|l| l["kind"] == "essentiel/end").unwrap_or_else(|| panic!("layout de fin"));
    assert_that!(end["slide"]["params"]["text"].as_str()).is_equal_to(Some(expected));
}
