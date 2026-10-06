//! Le japonais, le chinois et le coréen sortaient en carrés vides (.notdef) :
//! aucune police livrée ne couvrait ces alphabets (t-176). Typst ne signale
//! pas un glyphe manquant ; on lit donc les polices embarquées dans le PDF.
//! Les idéogrammes communs se dessinent différemment selon la région : la
//! police retenue suit la langue du deck.

use std::collections::BTreeMap;

use oku_slides_core::Deck;
use oku_slides_render::{Fonts, Renderer};
use serde_json::json;

use crate::common::{theme_name, theme_ref, theme_tokens};
use speculoos::prelude::*;

/// Les polices livrées avec l'app, comme les charge `oku-slides --fonts`.
fn app_fonts() -> Fonts {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts");
    let files = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{dir} illisible : {e}"));
    Fonts::new(files.filter_map(|f| {
        let path = f.ok()?.path();
        matches!(path.extension()?.to_str()?, "ttf" | "otf" | "ttc").then(|| std::fs::read(path).ok())?
    }))
}

/// PDF d'une slide de titre et de texte en `theme`, dans la langue `lang`.
fn pdf(theme: Option<&str>, lang: &str, text: &str) -> Vec<u8> {
    let deck: Deck = serde_json::from_value(json!({
        "theme": theme.map(theme_ref), "params": { "lang": lang },
        "slides": [{ "id": "s", "kind": "content", "params": { "title": text, "blocks": [
            { "id": "b", "kind": "text", "params": { "body": [{ "runs": [{ "t": text }] }] } }
        ] } }]
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    let mut r = Renderer::new(app_fonts()).unwrap_or_else(|e| panic!("{e}"));
    r.set_theme(theme_tokens(theme_name(deck.theme.as_ref()))).unwrap_or_else(|e| panic!("{e}"));
    r.set_deck(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    r.compile().and_then(|d| d.pdf()).unwrap_or_else(|e| panic!("{e}"))
}

/// Noms des polices embarquées (`/BaseFont/ABCDEF+Nom`), sans le préfixe de sous-ensemble.
fn embedded_fonts(pdf: &[u8]) -> Vec<String> {
    let s = String::from_utf8_lossy(pdf);
    let mut names: Vec<String> = s
        .split("/BaseFont/")
        .skip(1)
        .filter_map(|rest| {
            let name = rest.split(['/', ' ', '>', '\n']).next()?;
            Some(name.split_once('+').map_or(name, |(_, n)| n).trim_end_matches("-Identity-H").to_owned())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn cjk_text_in_every_theme_is_drawn_with_cjk_fonts() {
    for theme in [Some("papier"), Some("pro"), None] {
        let fonts = embedded_fonts(&pdf(theme, "ja", "日本語 한국어 中文"));
        asserting(&format!("{theme:?} : {fonts:?}")).that(&fonts.iter().any(|f| f.starts_with("NotoSansJP"))).is_true();
        asserting(&format!("{theme:?} : {fonts:?}")).that(&fonts.iter().any(|f| f.starts_with("NotoSansKR"))).is_true();
    }
}

#[test]
fn han_characters_follow_the_deck_language_region() {
    for (lang, region) in [("ja", "JP"), ("zh", "SC"), ("zh-TW", "TC"), ("zh-Hant", "TC"), ("ko", "KR"), ("fr", "SC")] {
        let fonts = embedded_fonts(&pdf(Some("papier"), lang, "中文 直"));
        let cjk: Vec<_> = fonts.iter().filter(|f| f.starts_with("NotoSans") && !f.contains("Arabic")).collect();
        asserting(&format!("{lang} : {fonts:?}"))
            .that(&cjk.iter().all(|f| f.starts_with(&format!("NotoSans{region}"))))
            .is_true();
        asserting(&format!("{lang} : {fonts:?}")).that(&cjk.is_empty()).is_false();
    }
}

#[test]
fn cjk_pdf_embeds_only_the_used_glyphs() {
    let size = pdf(Some("papier"), "ja", "日本語 한국어 中文").len();
    // Une police CJK complète pèse plusieurs Mo : seul le sous-ensemble utilisé est embarqué.
    assert_that!(size).is_less_than(300_000);
}

/// t-177 : le hindi, le bengali et le thaï sortaient aussi en carrés vides.
#[test]
fn devanagari_bengali_and_thai_in_every_theme_are_drawn_with_their_fonts() {
    for theme in [Some("papier"), Some("pro"), None] {
        let fonts = embedded_fonts(&pdf(theme, "hi", "हिन्दी বাংলা ภาษาไทย"));
        for family in ["NotoSansDevanagari", "NotoSansBengali", "NotoSansThai"] {
            asserting(&format!("{theme:?}, {family} : {fonts:?}"))
                .that(&fonts.iter().any(|f| f.starts_with(family)))
                .is_true();
        }
    }
}

/// PDF d'une slide dont le titre et le texte diffèrent.
fn pdf_title_body(theme: Option<&str>, lang: &str, title: &str, body: &str) -> Vec<u8> {
    let deck: Deck = serde_json::from_value(json!({
        "theme": theme.map(theme_ref), "params": { "lang": lang },
        "slides": [{ "id": "s", "kind": "content", "params": { "title": title, "blocks": [
            { "id": "b", "kind": "text", "params": { "body": [{ "runs": [{ "t": body }] }] } }
        ] } }]
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    let mut r = Renderer::new(app_fonts()).unwrap_or_else(|e| panic!("{e}"));
    r.set_theme(theme_tokens(theme_name(deck.theme.as_ref()))).unwrap_or_else(|e| panic!("{e}"));
    r.set_deck(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    r.compile().and_then(|d| d.pdf()).unwrap_or_else(|e| panic!("{e}"))
}

/// t-178 : les titres serif du thème papier (Newsreader) prennent le serif
/// CJK de leur région ; le texte courant reste en sans.
#[test]
fn papier_cjk_titles_use_serif_and_body_stays_sans() {
    for (lang, region) in [("ja", "JP"), ("zh", "SC"), ("zh-TW", "TC"), ("ko", "KR")] {
        let fonts = embedded_fonts(&pdf_title_body(Some("papier"), lang, "題名 제목", "本文 본문"));
        asserting(&format!("{lang} : {fonts:?}"))
            .that(&fonts.iter().any(|f| f.starts_with(&format!("NotoSerif{region}"))))
            .is_true();
        asserting(&format!("{lang} : {fonts:?}"))
            .that(&fonts.iter().any(|f| f.starts_with(&format!("NotoSans{region}"))))
            .is_true();
    }
}

/// Un thème aux titres sans (pro) ne prend jamais le serif CJK.
#[test]
fn pro_cjk_titles_stay_sans() {
    let fonts = embedded_fonts(&pdf_title_body(Some("pro"), "ja", "題名", "本文"));
    asserting(&format!("{fonts:?}")).that(&fonts.iter().any(|f| f.starts_with("NotoSerif"))).is_false();
}

/// Une graphie inconnue dans `script-style` (faute de frappe d'un thème)
/// fait échouer le rendu, jamais un repli silencieux sur le sans.
#[test]
fn unknown_script_style_fails_to_compile() {
    let deck: Deck = serde_json::from_value(json!({
        "theme": theme_ref("papier"), "params": { "lang": "ja" },
        "tokens": { "script-style": { "display": "serf" } },
        "slides": [{ "id": "s", "kind": "content", "params": { "title": "題名", "blocks": [] } }]
    }))
    .unwrap_or_else(|e| panic!("{e}"));
    let mut r = Renderer::new(app_fonts()).unwrap_or_else(|e| panic!("{e}"));
    r.set_theme(theme_tokens(theme_name(deck.theme.as_ref()))).unwrap_or_else(|e| panic!("{e}"));
    r.set_deck(&deck, &BTreeMap::new()).unwrap_or_else(|e| panic!("{e}"));
    let err = r.compile().err().map(|e| e.to_string()).unwrap_or_default();
    assert_that!(err).contains("script-style");
}
