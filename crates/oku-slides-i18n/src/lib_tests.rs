use std::collections::BTreeSet;

use speculoos::prelude::*;

use super::*;

fn keys(locale: &str) -> BTreeSet<String> {
    let (_, sources) = LOCALES.iter().find(|(l, _)| *l == locale).unwrap();
    sources
        .iter()
        .flat_map(|s| s.lines())
        .filter_map(|line| line.split_once(" = ").map(|(k, _)| k.trim().to_owned()))
        .filter(|k| !k.starts_with('#') && !k.starts_with(' ') && !k.is_empty())
        .collect()
}

#[test]
fn every_locale_parses_without_error() {
    for (locale, sources) in LOCALES {
        for source in *sources {
            asserting(&format!("{locale} : ressource invalide"))
                .that(&FluentResource::try_new((*source).to_owned()))
                .is_ok();
        }
    }
}

#[test]
fn every_locale_has_exactly_the_reference_keys() {
    let reference = keys(REFERENCE);
    assert_that!(reference.len()).is_greater_than(50);
    for (locale, _) in LOCALES {
        asserting(&format!("{locale} : clés différentes de {REFERENCE}")).that(&keys(locale)).is_equal_to(&reference);
    }
}

#[test]
fn new_region_tag_selects_its_language() {
    assert_that!(Localizer::new(Some("fr-CA")).locale()).is_equal_to("fr");
    assert_that!(Localizer::new(Some("en_GB")).locale()).is_equal_to("en");
}

#[test]
fn tr_with_args_formats_without_isolation_marks() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(fr.tr_with("cli-deck-valid", &[("slides", 8.into())]))
        .is_equal_to("deck valide : 8 slides".to_owned());
    assert_that!(english().tr_with("session-read", &[("path", "/a/b".into())]))
        .is_equal_to("Cannot read /a/b".to_owned());
}

#[test]
fn tr_unknown_id_returns_the_id() {
    assert_that!(Localizer::new(Some("fr")).tr("does-not-exist")).is_equal_to("does-not-exist".to_owned());
}

#[test]
fn ui_messages_give_english_then_the_chosen_language() {
    let en = include_str!("locales/en/ui.ftl");
    let fr = include_str!("locales/fr/ui.ftl");
    assert_that!(Localizer::new(Some("fr")).ui_messages()).is_equal_to(vec![en, fr]);
    assert_that!(english().ui_messages()).is_equal_to(vec![en]);
    assert_that!(Localizer::new(Some("en")).ui_messages()).is_equal_to(vec![en]);
}

#[test]
fn every_locale_ends_with_its_ui_messages() {
    for (locale, sources) in LOCALES {
        let last = sources.last().copied().unwrap_or_default();
        asserting(&format!("{locale} : ui.ftl doit être la dernière ressource"))
            .that(&last.lines().any(|l| l.starts_with("ui-loading = ")))
            .is_true();
    }
}

#[test]
fn candidate_maps_chinese_regions_and_scripts() {
    for (tag, expected) in [
        ("zh-CN", "zh-Hans"),
        ("zh_SG", "zh-Hans"),
        ("zh", "zh-Hans"),
        ("zh-Hans-CN", "zh-Hans"),
        ("zh-TW", "zh-Hant"),
        ("zh-HK", "zh-Hant"),
        ("zh_MO", "zh-Hant"),
        ("zh-Hant", "zh-Hant"),
        ("pt-BR", "pt"),
        ("FR-ca", "fr"),
    ] {
        asserting(tag).that(&candidate(tag)).is_equal_to(expected.to_owned());
    }
}

#[test]
fn twenty_languages_are_shipped_and_selectable() {
    assert_that!(LOCALES.len()).is_equal_to(20);
    for (locale, _) in LOCALES {
        asserting(locale).that(&Localizer::new(Some(locale)).locale()).is_equal_to(*locale);
    }
    assert_that!(Localizer::new(Some("zh-TW")).tr("deck-new-title")).is_not_equal_to(english().tr("deck-new-title"));
}

#[test]
fn every_locale_formats_every_message_without_errors() {
    for (locale, _) in LOCALES {
        let l = Localizer::new(Some(locale));
        for id in keys(REFERENCE) {
            let mut errors = Vec::new();
            if let Some(pattern) = l.bundle.get_message(&id).and_then(|m| m.value()) {
                // Sans paramètres : seuls comptent les erreurs de structure (références, sélecteurs).
                l.bundle.format_pattern(pattern, None, &mut errors);
            }
            let structural: Vec<_> = errors
                .iter()
                .filter(|e| {
                    !matches!(
                        e,
                        fluent_bundle::FluentError::ResolverError(fluent_bundle::resolver::ResolverError::Reference(_))
                    )
                })
                .collect();
            asserting(&format!("{locale} : {id}")).that(&structural.is_empty()).is_true();
        }
    }
}

#[test]
fn every_locale_names_each_chart_kind_differently() {
    for (locale, _) in LOCALES {
        let l = Localizer::new(Some(locale));
        let names: BTreeSet<String> = ["bar", "line", "pie", "scatter", "radar"]
            .into_iter()
            .map(|kind| l.tr_with("ui-chart-kind", &[("kind", kind.into())]))
            .collect();
        asserting(locale).that(&names.len()).is_equal_to(5);
    }
}

#[test]
fn every_locale_names_the_app_oku_slides() {
    for (locale, sources) in LOCALES {
        let text = sources.concat();
        asserting(&format!("{locale} : ui-brand")).that(&text.contains("\nui-brand = Oku Slides\n")).is_true();
        asserting(&format!("{locale} : ancien nom")).that(&text.contains("Écho")).is_false();
    }
}

#[test]
fn available_lists_every_shipped_language_reference_first() {
    let all = available();
    assert_that!(all.first().copied()).is_equal_to(Some(REFERENCE));
    assert_that!(all).has_length(LOCALES.len());
}

#[test]
fn shipped_maps_a_region_tag_and_refuses_the_unknown() {
    assert_that!(shipped("fr-CA")).is_equal_to(Some("fr"));
    assert_that!(shipped("xx")).is_none();
}

#[test]
fn widgets_prompt_mentions_slide_category_and_optional_label_and_default_in_every_locale() {
    for locale in crate::available() {
        let text = Localizer::new(Some(locale)).tr("agent-instructions-widgets");
        for part in ["category: \"slide\"", "label:", "default:"] {
            assert!(text.contains(part), "{locale} : « {part} » absent");
        }
    }
}

#[test]
fn agent_instructions_ask_for_report_not_suggest_next() {
    for lang in crate::available() {
        let text = Localizer::new(Some(lang)).tr("agent-instructions");
        assert!(text.contains("report"), "{lang} : report absent");
        assert!(text.contains("render"), "{lang} : render absent");
        assert!(!text.contains("suggest_next"), "{lang} : suggest_next present");
    }
}
