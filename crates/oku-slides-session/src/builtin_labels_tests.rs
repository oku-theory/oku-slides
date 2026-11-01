use oku_slides_core::{WidgetFile, qualify};
use oku_slides_i18n::Localizer;
use oku_slides_render::BuiltinPack;
use rstest::rstest;
use speculoos::prelude::*;

use super::{LOCALIZERS, inline_labels, labelled, typst_string};
use crate::pack::{BASE_PACK, ESSENTIEL_PACK, PackManifest};
use crate::param_fields::{kind_description, kind_name, param_label};
use crate::test_support::{builtin, fresh_dir};
use crate::test_support_packs::pack_tab_of;

/// The shipped source of built-in widget `name` of pack `id`.
fn shipped(id: uuid::Uuid, name: &str) -> String {
    let file = WidgetFile::parse(name).unwrap();
    builtin(id).files.widget(&file).unwrap_or_else(|| panic!("widget intégré {name}")).to_owned()
}

#[test]
fn labelled_card_title_declares_its_label_by_locale() {
    let out = labelled(&shipped(BASE_PACK, "card"), "base-card", &LOCALIZERS).unwrap();
    assert_that!(out).contains("title: p.plain(label: (\"en\": \"Title\", ");
    assert_that!(out).contains("\"fr\": \"Titre\"");
    assert_that!(out).contains("\"zh-Hans\": ");
}

#[test]
fn labelled_card_widget_declares_its_name_and_description() {
    let out = labelled(&shipped(BASE_PACK, "card"), "base-card", &LOCALIZERS).unwrap();
    assert_that!(out).contains("widget(label: (\"en\": \"Card\", ");
    assert_that!(out).contains("\"fr\": \"Carte\"");
    assert_that!(out).contains("description: (\"en\": \"");
}

#[test]
fn labelled_empty_call_takes_only_the_labels() {
    let out = labelled("params: (highlight: p.flag())", "base-card", &LOCALIZERS).unwrap();
    assert_that!(out).starts_with("params: (highlight: p.flag(label: (\"en\": \"Highlight\", ");
    assert_that!(out).contains("), help: (\"en\": \"Puts the card");
    assert_that!(out).ends_with("\")))");
}

#[test]
fn labelled_call_with_arguments_keeps_them_after_the_labels() {
    let out = labelled("(kicker: p.plain(default: \"x\"))", "base-card", &LOCALIZERS).unwrap();
    assert_that!(out).ends_with("\"), default: \"x\"))");
}

#[test]
fn labelled_call_with_a_label_is_untouched() {
    let source = "(title: p.plain(label: \"Mine\"), kicker: p.plain(help: \"Own\"))";
    let out = labelled(source, "base-card", &LOCALIZERS).unwrap();
    assert_that!(out).starts_with("(title: p.plain(label: \"Mine\"), kicker: p.plain(label: (\"en\": \"Kicker\"");
    assert_that!(out.matches("help:").count()).is_equal_to(1);
}

#[test]
fn labelled_unknown_key_adds_nothing() {
    let source = "#let widget = widget(\"quote\", params: (who: p.plain()))";
    let out = labelled(source, "base-quote", &LOCALIZERS).unwrap();
    assert_that!(out.as_str()).is_equal_to(source);
}

#[test]
fn labelled_ignores_calls_in_strings_and_comments() {
    let source = "// title: p.plain()\n/* title: p.plain() */ \"title: p.plain()\"";
    let out = labelled(source, "base-card", &LOCALIZERS).unwrap();
    assert_that!(out.as_str()).is_equal_to(source);
}

#[rstest]
#[case::unbalanced("(title: p.plain(")]
#[case::unterminated_string("(title: p.plain(\"x))")]
#[case::unterminated_comment("/* (title: p.plain())")]
fn labelled_unreadable_source_is_none(#[case] source: &str) {
    assert_that!(labelled(source, "base-card", &LOCALIZERS)).is_none();
}

#[rstest]
#[case::plain("Title", "\"Title\"")]
#[case::quote("say \"hi\"", "\"say \\\"hi\\\"\"")]
#[case::backslash("a\\b", "\"a\\\\b\"")]
#[case::newline("a\nb", "\"a\\nb\"")]
fn typst_string_escapes(#[case] text: &str, #[case] expected: &str) {
    assert_that!(typst_string(text).as_str()).is_equal_to(expected);
}

#[test]
fn inline_labels_keeps_the_common_file() {
    let files = builtin(ESSENTIEL_PACK).files.widgets().clone();
    let out = inline_labels(BuiltinPack::Essentiel, files.clone());
    let common = WidgetFile::parse("_commun").unwrap();
    assert_that!(out.get(&common)).is_equal_to(files.get(&common));
}

/// Saves built-in pack `id` as a copy from a pack tab; returns the session and the copy's manifest.
fn copy_of(data: &std::path::Path, id: uuid::Uuid) -> (crate::Session, PackManifest) {
    let mut s = pack_tab_of(data, id);
    let saved = s.save_pack(true).unwrap();
    let files = s.doc.sources.packs.get(&saved).unwrap_or_else(|| panic!("copie embarquée"));
    let manifest = PackManifest::from_files(files).unwrap();
    (s, manifest)
}

#[test]
fn save_pack_copy_of_base_keeps_the_card_labels_in_french() {
    let data = fresh_dir("oku-slides-builtin-labels-");
    let (s, copy) = copy_of(data.path(), BASE_PACK);
    let kind = qualify(&copy.namespace, "card");
    let schema = s.registry.get(&kind).unwrap_or_else(|| panic!("{kind} sondé"));
    let title = schema.params.iter().find(|p| p.name == "title").unwrap();
    let fr = Localizer::new(Some("fr"));
    assert_that!(param_label(&kind, title, &fr).as_str()).is_equal_to("Titre");
    assert_that!(kind_name(&kind, schema.label.as_ref(), &fr).as_str()).is_equal_to("Carte");
    assert_that!(kind_description(&kind, schema.description.as_ref(), &fr))
        .is_equal_to(Some(fr.tr("session-widget-description-base-card")));
}

#[rstest]
#[case::base(BASE_PACK)]
#[case::essentiel(ESSENTIEL_PACK)]
fn save_pack_copy_of_a_builtin_probes_every_widget_with_a_label(#[case] id: uuid::Uuid) {
    let data = fresh_dir("oku-slides-builtin-labels-");
    let (s, copy) = copy_of(data.path(), id);
    let names: Vec<String> =
        builtin(id).files.widgets().keys().filter(|f| !f.is_common()).map(|f| f.as_str().to_owned()).collect();
    let labelled: Vec<String> = names
        .iter()
        .filter(|n| s.registry.get(&qualify(&copy.namespace, n)).is_some_and(|w| w.label.is_some()))
        .cloned()
        .collect();
    assert_that!(labelled).is_equal_to(names);
}
