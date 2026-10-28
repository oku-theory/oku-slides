use std::time::Duration;

use oku_slides_a2ui::{Action, ButtonIcon, ComponentKind, ServerBody};
use oku_slides_core::Namespace;
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};
use speculoos::prelude::*;
use uuid::Uuid;

use super::{Bounds, HOST_THEME_PICKER, THEMES_RENDERED, ThemeCard, picked_theme, swatches, theme_picker};
use crate::library_kind::LibraryKind;
use crate::test_support::{fresh_dir, ns, opened_on, session};
use crate::{PAPIER_NAMESPACE, PRO_NAMESPACE, PRO_THEME, Session};

/// Les composants `ThemeChoice` d'une surface, dans l'ordre.
fn choices(s: &crate::agent::Surface) -> Vec<ComponentKind> {
    components(s).into_iter().filter(|k| matches!(k, ComponentKind::ThemeChoice { .. })).collect()
}

fn components(s: &crate::agent::Surface) -> Vec<ComponentKind> {
    s.messages
        .iter()
        .filter_map(|m| match &m.body {
            ServerBody::UpdateComponents(u) => Some(u.components.clone()),
            _ => None,
        })
        .flatten()
        .map(|c| c.kind)
        .collect()
}

fn card(namespace: &str, name: &str) -> ThemeCard {
    ThemeCard { namespace: ns(namespace), name: name.into(), slide: "s-1".into(), swatches: vec![], svg: None }
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

fn colour(name: &str) -> String {
    crate::theme_schema::default_tokens()["color"][name].as_str().unwrap().to_owned()
}

fn chart0() -> String {
    crate::theme_schema::default_tokens()["chart"]["palette"][0].as_str().unwrap().to_owned()
}

/// Une session ouverte sur la fixture, sur un dossier de données gardé tant qu'elle vit.
fn session_with_library() -> (Session, tempfile::TempDir) {
    let data = fresh_dir("oku-slides-theme-picker-");
    (opened_on(data.path()), data)
}

/// Ajoute à la bibliothèque de `s` un thème `namespace`, aux jetons de Pro recouverts par `tokens`.
fn add_theme(s: &mut Session, namespace: &str, tokens: Value) -> Namespace {
    let pro = crate::theme::Themes::builtins().iter().find(|b| b.manifest.id == PRO_THEME).unwrap();
    let mut all = pro.manifest.tokens.clone();
    let tokens: serde_json::Map<String, Value> = serde_json::from_value(tokens).unwrap();
    oku_slides_core::tokens::merge(&mut all, &tokens);
    crate::test_support::add_user_theme(&s.themes, Uuid::new_v4(), namespace, &[], all, None);
    s.theme_list = s.themes.themes();
    ns(namespace)
}

const WIDE: Bounds = Bounds { budget: Duration::from_secs(600), thumb_bytes: super::THUMB_BYTES };

#[test]
fn swatches_read_four_colours_then_the_first_chart_colour() {
    let tokens = json!({ "color": { "accent": "#c4622d" }, "chart": { "palette": ["#112233"] } });
    assert_that!(swatches(tokens.as_object().unwrap())).is_equal_to(vec![
        colour("bg"),
        colour("surface"),
        colour("ink"),
        "#c4622d".to_owned(),
        "#112233".to_owned(),
    ]);
}

#[test]
fn swatches_resolve_a_colour_that_names_another() {
    let tokens = json!({ "color": { "accent": "ink", "ink": "#101010" } });
    assert_that!(swatches(tokens.as_object().unwrap())[3].as_str()).is_equal_to("#101010");
}

#[test]
fn swatches_skip_an_unknown_colour_name() {
    let tokens = json!({ "color": { "accent": "nowhere" } });
    assert_that!(swatches(tokens.as_object().unwrap())).is_equal_to(vec![
        colour("bg"),
        colour("surface"),
        colour("ink"),
        chart0(),
    ]);
}

#[test]
fn theme_picker_wires_each_choice_to_its_theme_namespace() {
    let s = theme_picker(&[card("papier", "Papier"), card("pro", "Pro")], &ns("pro"), &fr());
    let picked: Vec<Option<Namespace>> = choices(&s)
        .into_iter()
        .map(|k| match k {
            ComponentKind::ThemeChoice { action: Some(a), .. } => {
                let ctx = serde_json::to_value(&a).unwrap()["event"]["context"].clone();
                picked_theme(ctx.as_object().unwrap())
            }
            _ => None,
        })
        .collect();
    assert_that!(picked).is_equal_to(vec![Some(ns("papier")), Some(ns("pro"))]);
}

#[test]
fn theme_picker_marks_only_the_active_card() {
    let s = theme_picker(&[card("papier", "Papier"), card("pro", "Pro")], &ns("pro"), &fr());
    let active: Vec<bool> =
        choices(&s).into_iter().map(|k| matches!(k, ComponentKind::ThemeChoice { active: true, .. })).collect();
    assert_that!(active).is_equal_to(vec![false, true]);
}

#[test]
fn theme_picker_uses_the_host_surface_id() {
    let s = theme_picker(&[card("papier", "Papier")], &ns("papier"), &fr());
    assert_that!(s.id.as_str()).is_equal_to(HOST_THEME_PICKER);
}

#[test]
fn theme_picker_is_not_a_question() {
    let s = theme_picker(&[card("papier", "Papier")], &ns("papier"), &fr());
    assert_that!(s.question).is_false();
}

#[test]
fn theme_picker_has_a_dismiss_button() {
    let s = theme_picker(&[card("papier", "Papier")], &ns("papier"), &fr());
    let dismiss = components(&s).into_iter().any(|k| match k {
        ComponentKind::Button { action, .. } => action == Action::named("dismiss"),
        _ => false,
    });
    assert_that!(dismiss).is_true();
}

#[test]
fn theme_picker_dismiss_button_shows_a_close_icon() {
    let s = theme_picker(&[card("papier", "Papier")], &ns("papier"), &fr());
    let icon = components(&s).into_iter().find_map(|k| match k {
        ComponentKind::Button { action, icon, .. } if action == Action::named("dismiss") => icon,
        _ => None,
    });
    assert_that!(icon).is_equal_to(Some(ButtonIcon::Close));
}

#[test]
fn picked_theme_refuses_a_malformed_namespace() {
    let ctx = json!({ "theme": "Pas un espace" });
    assert_that!(picked_theme(ctx.as_object().unwrap())).is_none();
}

#[test]
fn theme_choices_list_the_library_themes_by_namespace() {
    let (mut s, _data) = session_with_library();
    add_theme(&mut s, "zephyr", json!({}));
    let listed: Vec<Namespace> =
        s.theme_choices_within(None, WIDE).unwrap().unwrap().cards.iter().map(|c| c.namespace.clone()).collect();
    let expected: Vec<Namespace> = s.theme_list.iter().map(|t| t.namespace.clone()).collect();
    assert_that!(listed).is_equal_to(expected);
    assert_that!(s.theme_list.iter().any(|t| t.namespace.as_str() == "zephyr")).is_true();
}

#[test]
fn theme_choices_of_a_new_deck_mark_papier_active() {
    let mut s = session();
    assert_that!(s.theme_choices_within(None, WIDE).unwrap().unwrap().active).is_equal_to(ns(PAPIER_NAMESPACE));
}

#[test]
fn theme_choices_after_set_theme_marks_it_active() {
    let mut s = session();
    s.set_theme(&ns(PRO_NAMESPACE)).unwrap();
    assert_that!(s.theme_choices_within(None, WIDE).unwrap().unwrap().active).is_equal_to(ns(PRO_NAMESPACE));
}

#[test]
fn theme_choices_render_the_requested_slide() {
    let mut s = session();
    let card = s.theme_choices_within(Some("s-libre"), WIDE).unwrap().unwrap().cards.remove(0);
    assert_that!((card.slide.as_str(), card.svg.is_some())).is_equal_to(("s-libre", true));
}

#[test]
fn theme_choices_with_an_unknown_slide_show_the_first() {
    let mut s = session();
    let first = s.deck().slides[0].id.clone();
    let card = s.theme_choices_within(Some("s-nope"), WIDE).unwrap().unwrap().cards.remove(0);
    assert_that!((card.slide, card.svg.is_some())).is_equal_to((first, true));
}

#[test]
fn theme_choices_give_each_theme_its_own_thumbnail() {
    let mut s = session();
    let cards = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards;
    let svg = |n: &str| cards.iter().find(|c| c.namespace.as_str() == n).and_then(|c| c.svg.clone());
    let (papier, pro) = (svg(PAPIER_NAMESPACE), svg(PRO_NAMESPACE));
    assert_that!(papier.is_some() && pro.is_some() && papier != pro).is_true();
}

#[test]
fn theme_choices_render_at_most_themes_rendered_thumbnails() {
    let (mut s, _data) = session_with_library();
    for i in 0..THEMES_RENDERED {
        add_theme(&mut s, &format!("theme-{i:02}"), json!({}));
    }
    let cards = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards;
    assert_that!(cards.iter().filter(|c| c.svg.is_some()).count()).is_equal_to(THEMES_RENDERED);
}

#[test]
fn theme_choices_past_the_budget_still_render_the_active() {
    let (mut s, _data) = session_with_library();
    let last = add_theme(&mut s, "zzz", json!({}));
    s.set_theme(&last).unwrap();
    let none = Bounds { budget: Duration::ZERO, thumb_bytes: super::THUMB_BYTES };
    let rendered: Vec<Namespace> = s
        .theme_choices_within(None, none)
        .unwrap()
        .unwrap()
        .cards
        .into_iter()
        .filter(|c| c.svg.is_some())
        .map(|c| c.namespace)
        .collect();
    assert_that!(rendered).is_equal_to(vec![last]);
}

#[test]
fn theme_choices_drop_an_oversized_thumbnail() {
    let mut s = session();
    let tiny = Bounds { budget: Duration::from_secs(600), thumb_bytes: 1 };
    let cards = s.theme_choices_within(None, tiny).unwrap().unwrap().cards;
    assert_that!(cards.iter().all(|c| c.svg.is_none()) && !cards.is_empty()).is_true();
}

#[test]
fn theme_choices_keep_a_theme_that_fails_to_render_without_thumbnail() {
    let (mut s, _data) = session_with_library();
    let broken = add_theme(&mut s, "casse", json!({ "color": { "bg": 12 } }));
    let cards = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards;
    let card = cards.iter().find(|c| c.namespace == broken).unwrap();
    let others_rendered = cards.iter().filter(|c| c.namespace != broken).all(|c| c.svg.is_some());
    assert_that!((card.svg.is_none(), others_rendered)).is_equal_to((true, true));
}

#[test]
fn theme_choices_on_an_empty_deck_keep_every_card_without_thumbnail() {
    let mut s = session();
    let ops = s.deck().slides.iter().map(|x| oku_slides_core::Op::RemoveSlide { slide: x.id.clone() }).collect();
    s.apply("vider", ops, None).unwrap();
    let cards = s.theme_choices_within(None, WIDE).unwrap().unwrap().cards;
    assert_that!((cards.len(), cards.iter().all(|c| c.svg.is_none()))).is_equal_to((s.theme_list.len(), true));
}

#[test]
fn theme_choices_outside_deck_mode_are_none() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), crate::pack::ESSENTIEL_PACK);
    assert_that!(s.theme_choices(None).unwrap()).is_none();
}

#[test]
fn theme_choices_leave_the_deck_unchanged() {
    let mut s = session();
    let before = (s.deck().clone(), s.snapshot().undo_label);
    s.theme_choices_within(None, WIDE).unwrap();
    assert_that!((s.deck().clone(), s.snapshot().undo_label)).is_equal_to(before);
}

#[test]
fn pick_theme_sets_the_deck_theme() {
    let mut s = session();
    s.pick_theme(&ns(PRO_NAMESPACE)).unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(Some(ns(PRO_NAMESPACE)));
}

#[test]
fn pick_theme_is_one_undo_step() {
    let mut s = session();
    let before = s.deck().theme.clone();
    s.pick_theme(&ns(PRO_NAMESPACE)).unwrap();
    s.undo().unwrap();
    assert_that!(s.deck().theme.clone()).is_equal_to(before);
}

#[test]
fn pick_theme_notes_the_quoted_name_for_the_agent() {
    let mut s = session();
    s.pick_theme(&ns(PRO_NAMESPACE)).unwrap();
    let note = "theme picked: \"Pro\" (pro)";
    assert_that!(s.agent_context(&crate::agent::context::AgentUi::default()).contains(note)).is_true();
}

#[test]
fn pick_theme_of_an_unknown_namespace_is_refused() {
    let mut s = session();
    assert_that!(s.pick_theme(&ns("absent")).map_err(|e| e.kind()))
        .is_err_containing(crate::SessionErrorKind::UnknownTheme);
}

#[test]
fn pick_theme_in_pack_mode_is_refused() {
    let data = crate::test_support::fresh_dir("oku-slides-pack-tab-");
    let mut s = crate::test_support_packs::pack_tab_of(data.path(), crate::pack::ESSENTIEL_PACK);
    assert_that!(s.pick_theme(&ns(PRO_NAMESPACE))).is_err();
}
