use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::fonts::{FontChoice, FontDecision, FontOrigin, resolve};
use crate::source::{Bullet, Color, Fill, Frame, LevelStyle, MasterTextStyles, Placeholder, Size, Styled, TextBox};
use crate::test_support::{
    DEFAULT_TOKENS, app_fonts, fixture, layout, office_theme, para, run_style, source_deck, source_slide,
};

/// Placeholder de rôle `role` dont le premier niveau est en `size` pt.
fn placeholder(id: &str, role: Role, size: f64) -> Placeholder {
    Placeholder {
        id: id.into(),
        role,
        idx: None,
        frame: Frame::new(0.0, 0.0, 100.0, 50.0),
        levels: vec![level(size)],
        text: TextBox::default(),
    }
}

/// Style de premier niveau en `size` pt.
fn level(size: f64) -> LevelStyle {
    let mut run = run_style();
    run.size = size;
    LevelStyle { para: para(Vec::new()).style, run }
}

/// Master `id` du thème `theme` ; ses `txStyles` : titre 40, corps 20, autres 18.
fn master(id: &str, theme: usize, placeholders: Vec<Placeholder>) -> Master {
    let text_styles = MasterTextStyles { title: level(40.0), body: level(20.0), other: level(18.0) };
    Master {
        id: id.into(),
        name: id.into(),
        theme,
        background: Fill::None,
        decor: Vec::new(),
        placeholders,
        text_styles,
    }
}

/// Un deck au thème Office : un master (titre 44, corps 28, pied 12), un layout de titre (sous-titre 24).
fn office_deck() -> SourceDeck {
    let mut deck = source_deck(vec![source_slide(1, Some("l1"), Vec::new())]);
    deck.themes.push(office_theme());
    deck.masters.push(master(
        "m1",
        0,
        vec![
            placeholder("m1/2", Role::Title, 44.0),
            placeholder("m1/3", Role::Body, 28.0),
            placeholder("m1/4", Role::Footer, 12.0),
        ],
    ));
    let mut title = layout("l1", "title");
    title.placeholders.push(placeholder("l1/3", Role::Subtitle, 24.0));
    deck.layouts.push(title);
    deck
}

/// Calibri → Carlito ; Calibri Light introuvable.
fn office_fonts() -> FontPlan {
    let choice = |family: &str, used: Option<&str>, decision| FontChoice {
        family: family.into(),
        used: used.map(str::to_owned),
        decision,
        restricted: false,
        too_large: false,
        fallback: None,
    };
    FontPlan {
        choices: vec![
            choice("Calibri", Some("Carlito"), FontDecision::Substituted { origin: FontOrigin::App }),
            choice("Calibri Light", None, FontDecision::Missing),
        ],
        ..FontPlan::default()
    }
}

fn office_tokens() -> Map<String, Value> {
    let deck = office_deck();
    theme_tokens(&deck, &deck.masters[0], &office_fonts(), &Page::new(deck.size))
}

#[test]
fn theme_master_is_the_master_of_the_first_slide_layout() {
    let mut deck = office_deck();
    deck.masters.insert(0, master("m0", 0, Vec::new()));
    assert_that!(theme_master(&deck).map(|m| m.id.as_str())).is_equal_to(Some("m1"));
}

#[test]
fn theme_master_without_slides_is_the_first_master() {
    let mut deck = office_deck();
    deck.slides.clear();
    deck.masters.insert(0, master("m0", 0, Vec::new()));
    assert_that!(theme_master(&deck).map(|m| m.id.as_str())).is_equal_to(Some("m0"));
}

#[test]
fn theme_tokens_write_the_twelve_slots_and_the_derived_colors() {
    let tokens = office_tokens();
    assert_that!(tokens["color"]).is_equal_to(json!({
        "bg": "#ffffff", "ink": "#000000", "ink-muted": "#44546a", "surface": "#e7e6e6",
        "accent": "#4472c4", "accent-2": "#ed7d31", "accent-3": "#a5a5a5", "accent-4": "#ffc000",
        "accent-5": "#5b9bd5", "accent-6": "#70ad47", "link": "#0563c1", "link-visited": "#954f72",
        "on-accent": "bg", "rule": "#d9d9d9",
    }));
}

#[test]
fn theme_tokens_put_dark_ink_on_a_light_accent() {
    let mut deck = office_deck();
    deck.themes[0].colors.insert("accent1".into(), Rgba::new(0xff, 0xc0, 0x00));
    let tokens = theme_tokens(&deck, &deck.masters[0], &office_fonts(), &Page::new(deck.size));
    assert_that!(tokens["color"]["on-accent"]).is_equal_to(json!("ink"));
}

#[test]
fn theme_tokens_chart_palette_names_the_six_accents() {
    let tokens = office_tokens();
    assert_that!(tokens["chart"]["palette"])
        .is_equal_to(json!(["accent", "accent-2", "accent-3", "accent-4", "accent-5", "accent-6"]));
}

#[test]
fn theme_tokens_follow_the_latin_font_with_the_retained_script_fonts() {
    let mut deck = office_deck();
    let fonts = &mut deck.themes[0].fonts;
    (fonts.minor_ea, fonts.minor_cs) = (Some("Caladea".into()), Some("Introuvable".into()));
    let plan = resolve(&[], &["Calibri", "Caladea", "Introuvable"], &app_fonts());
    let tokens = theme_tokens(&deck, &deck.masters[0], &plan, &Page::new(deck.size));
    assert_that!(tokens["font"]["body"]).is_equal_to(json!(["Carlito", "Caladea"]));
}

#[test]
fn theme_tokens_keep_the_default_font_of_a_missing_family() {
    let tokens = office_tokens();
    assert_that!(tokens["font"]).is_equal_to(json!({ "body": ["Carlito"] }));
}

#[test]
fn theme_tokens_give_bold_runs_the_bold_face_of_the_source() {
    assert_that!(office_tokens()["weight"]).is_equal_to(json!({ "body": 400, "strong": 700 }));
}

#[test]
fn theme_tokens_scale_the_master_text_sizes_to_the_page() {
    let tokens = office_tokens();
    assert_that!(tokens["size"]).is_equal_to(json!({ "title": 88.0, "subtitle": 48.0, "body": 56.0, "small": 24.0 }));
}

#[rstest]
#[case(Size { w: 720.0, h: 540.0 }, 88.0)]
#[case(Size { w: 960.0, h: 540.0 }, 88.0)]
#[case(Size { w: 1920.0, h: 1080.0 }, 44.0)]
fn theme_tokens_size_follows_the_page_scale(#[case] size: Size, #[case] title: f64) {
    let mut deck = office_deck();
    deck.size = size;
    let tokens = theme_tokens(&deck, &deck.masters[0], &office_fonts(), &Page::new(deck.size));
    assert_that!(tokens["size"]["title"]).is_equal_to(json!(title));
}

#[test]
fn theme_tokens_clamp_a_huge_title_to_the_schema_bound() {
    let mut deck = office_deck();
    deck.masters[0].placeholders[0] = placeholder("m1/2", Role::Title, 250.0);
    let tokens = theme_tokens(&deck, &deck.masters[0], &office_fonts(), &Page::new(deck.size));
    assert_that!(tokens["size"]["title"]).is_equal_to(json!(400.0));
}

#[test]
fn theme_tokens_without_placeholders_take_the_master_text_styles() {
    let mut deck = office_deck();
    deck.layouts.clear();
    deck.masters[0].placeholders.clear();
    let tokens = theme_tokens(&deck, &deck.masters[0], &office_fonts(), &Page::new(deck.size));
    assert_that!(tokens["size"]).is_equal_to(json!({ "title": 80.0, "body": 40.0 }));
}

#[test]
fn theme_tokens_without_theme_keep_the_default_colors_and_fonts() {
    let mut deck = office_deck();
    deck.themes.clear();
    let tokens = theme_tokens(&deck, &deck.masters[0], &office_fonts(), &Page::new(deck.size));
    let groups: Vec<&str> = tokens.keys().map(String::as_str).collect();
    assert_that!(groups).is_equal_to(vec!["color", "size", "weight", "page"]);
    assert_that!(tokens["color"]).is_equal_to(json!({}));
}

#[test]
fn theme_tokens_without_theme_take_the_fallback_font_for_both_roles() {
    let mut deck = office_deck();
    deck.themes.clear();
    let fonts = FontPlan { fallback: Some("Liberation Sans".into()), ..office_fonts() };
    let tokens = theme_tokens(&deck, &deck.masters[0], &fonts, &Page::new(deck.size));
    assert_that!(tokens["font"]).is_equal_to(json!({ "display": ["Liberation Sans"], "body": ["Liberation Sans"] }));
}

#[test]
fn masterless_tokens_take_the_fallback_font() {
    let fonts = FontPlan { fallback: Some("Inter".into()), ..FontPlan::default() };
    let tokens = masterless_tokens(&fonts);
    assert_that!(tokens["font"]).is_equal_to(json!({ "display": ["Inter"], "body": ["Inter"] }));
    assert_that!(masterless_tokens(&FontPlan::default()).contains_key("font")).is_false();
}

/// Chaque jeton produit est une feuille de `default-tokens.json`, ou une couleur libre `[a-z][a-z0-9-]*`.
fn assert_known_paths(tokens: &Map<String, Value>) {
    let defaults: Value = serde_json::from_str(DEFAULT_TOKENS).unwrap();
    let free_color = |name: &str| {
        name.starts_with(|c: char| c.is_ascii_lowercase())
            && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    };
    for (group, values) in tokens {
        for (name, value) in values.as_object().unwrap() {
            let known = defaults[group].get(name).is_some() || (group == "color" && free_color(name));
            assert_that!(known).named(&format!("{group}.{name}")).is_true();
            if group == "component" {
                for key in value.as_object().unwrap().keys() {
                    assert_that!(defaults[group][name].get(key)).named(&format!("{group}.{name}.{key}")).is_some();
                }
            }
            if group == "color" {
                let v = value.as_str().unwrap();
                let named = tokens["color"].get(v).is_some();
                assert_that!(oku_slides_core::is_hex_color(v) || named).named(v).is_true();
            }
        }
    }
}

#[test]
fn theme_tokens_only_write_known_tokens() {
    assert_known_paths(&office_tokens());
}

/// Les jetons du thème généré pour la fixture `name`, avec les polices de l'app.
fn fixture_tokens(name: &str) -> Map<String, Value> {
    let deck = fixture(name);
    let master = theme_master(&deck).unwrap();
    let theme = &deck.themes[master.theme];
    let fonts = resolve(&deck.fonts, &[&theme.fonts.major, &theme.fonts.minor], &app_fonts());
    let tokens = theme_tokens(&deck, master, &fonts, &Page::new(deck.size));
    assert_known_paths(&tokens);
    tokens
}

#[rstest]
#[case("alto-wealth.pptx", "alto_theme")]
#[case("kitchen", "kitchen_theme")]
fn theme_tokens_of_a_fixture(#[case] fixture: &str, #[case] snapshot: &str) {
    insta::assert_json_snapshot!(snapshot, fixture_tokens(fixture));
}

#[test]
fn theme_manifest_is_a_first_version_of_the_tokens() {
    let tokens = office_tokens();
    let id = Uuid::from_u128(7);
    let m = theme_manifest(tokens.clone(), id, "Alto Wealth", "2026-10-11T08:00:00Z");
    let files = m.to_files().unwrap();
    assert_that!(ThemeManifest::from_files(&files).unwrap()).is_equal_to(&m);
    assert_that!((m.id, m.name.as_str(), m.parent.is_none(), &m.tokens)).is_equal_to((
        id,
        "Alto Wealth",
        true,
        &tokens,
    ));
}

#[test]
fn theme_colors_of_the_theme_master_agree_with_its_tokens() {
    let deck = office_deck();
    let colors = theme_colors(&deck, &deck.masters[0]);
    let tokens = office_tokens();
    for (_, token) in SLOT_TOKENS {
        assert_that!(colors.get(token).map(|c| json!(c.to_hex()))).is_equal_to(tokens["color"].get(token).cloned());
    }
}

/// Le deck Office dont le corps du master a la puce `bullet` au premier niveau.
fn bulleted(bullet: Bullet) -> Map<String, Value> {
    let mut deck = office_deck();
    deck.masters[0].placeholders[1].levels[0].para.bullet = bullet;
    theme_tokens(&deck, &deck.masters[0], &office_fonts(), &Page::new(deck.size))
}

fn bullet_char(char: &str, color: Option<Color>) -> Bullet {
    Bullet::Char { char: char.into(), color, size_pct: 1.0, font: None }
}

#[test]
fn theme_tokens_take_the_list_marker_of_the_master_body() {
    let tokens = bulleted(bullet_char("•", None));
    // Une puce sans couleur propre prend celle du texte.
    assert_that!(tokens["component"]).is_equal_to(json!({ "list": { "marker": "•", "marker-color": null } }));
    assert_known_paths(&tokens);
}

#[test]
fn theme_tokens_give_the_marker_its_own_color() {
    let accent = Styled::scheme(office_theme().colors["accent2"], "accent2");
    let tokens = bulleted(bullet_char("▪", Some(accent)));
    assert_that!(tokens["component"]["list"]).is_equal_to(json!({ "marker": "▪", "marker-color": "accent-2" }));
    let odd = bulleted(bullet_char("▪", Some(Styled::explicit(Rgba::new(1, 2, 3)))));
    assert_that!(odd["component"]["list"]["marker-color"]).is_equal_to(json!("#010203"));
}

#[rstest]
#[case::none(Bullet::None)]
#[case::numbered(Bullet::AutoNum { scheme: "arabicPeriod".into(), start: 1 })]
fn theme_tokens_without_a_bullet_character_keep_the_default_list(#[case] bullet: Bullet) {
    assert_that!(bulleted(bullet).get("component")).is_none();
}

#[test]
fn theme_tokens_without_body_placeholder_take_the_master_body_style() {
    let mut deck = office_deck();
    deck.masters[0].placeholders.clear();
    deck.masters[0].text_styles.body.para.bullet = bullet_char("–", None);
    let tokens = theme_tokens(&deck, &deck.masters[0], &office_fonts(), &Page::new(deck.size));
    assert_that!(tokens["component"]["list"]["marker"]).is_equal_to(json!("–"));
}
