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
fn widgets_prompt_documents_lists_and_their_ops_in_every_locale() {
    for locale in crate::available() {
        let text = Localizer::new(Some(locale)).tr("agent-instructions-widgets");
        for part in ["p.list", "list-item", "edit_list", "set_param", "\"defaults\": true"] {
            assert!(text.contains(part), "{locale} : « {part} » absent");
        }
    }
}

#[test]
fn widgets_prompt_import_line_lists_list_item_in_every_locale() {
    for locale in crate::available() {
        let text = Localizer::new(Some(locale)).tr("agent-instructions-widgets");
        let at = text.find("fields-stack").unwrap_or_else(|| panic!("{locale} : ligne d'import absente"));
        let line: String = text[at..].chars().take(80).collect();
        assert!(line.contains("list-item"), "{locale} : list-item absent de l'import : {line}");
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

/// Le résumé d'un tour suit les règles de pluriel de chaque langue, pas seulement celles du français.
#[test]
fn agent_turn_summary_plurals_follow_each_language() {
    for (locale, id, arg, n, expected) in [
        ("en", "agent-turn-tools", "tools", 1, "1 tool"),
        ("en", "agent-turn-tools", "tools", 2, "2 tools"),
        ("en", "agent-turn-notes", "notes", 0, "0 notes"),
        ("ru", "agent-turn-tools", "tools", 1, "1 инструмент"),
        ("ru", "agent-turn-tools", "tools", 3, "3 инструмента"),
        ("ru", "agent-turn-tools", "tools", 5, "5 инструментов"),
        ("ru", "agent-turn-notes", "notes", 21, "21 заметка"),
        ("pl", "agent-turn-tools", "tools", 2, "2 narzędzia"),
        ("pl", "agent-turn-tools", "tools", 5, "5 narzędzi"),
        ("pl", "agent-turn-notes", "notes", 22, "22 notatki"),
        ("ar", "agent-turn-tools", "tools", 2, "أداتان"),
        ("ar", "agent-turn-notes", "notes", 1, "ملاحظة واحدة"),
        ("ar", "agent-turn-tools", "tools", 0, "لا أدوات"),
        ("ar", "agent-turn-tools", "tools", 11, "11 أداة"),
        ("th", "agent-turn-tools", "tools", 3, "เครื่องมือ 3 รายการ"),
        ("th", "agent-turn-notes", "notes", 2, "บันทึก 2 รายการ"),
    ] {
        let text = Localizer::new(Some(locale)).tr_with(id, &[(arg, n.into())]);
        asserting(&format!("{locale} : {id} ({n})")).that(&text).is_equal_to(expected.to_owned());
    }
}

/// En japonais, chinois et coréen, le compteur se colle au nombre (« 3個 », « 3秒 », « 3개 ») :
/// aucun blanc, quel qu'il soit, entre le chiffre et ce qui le suit — dans le résumé d'un tour
/// comme dans les messages de la ligne de commande, pour un compteur reçu en paramètre comme pour
/// un nombre écrit dans le message (« 1枚あたり »). Les autres paramètres reçoivent un texte
/// sans chiffre : une unité latine (« 12 KB ») n'est pas un compteur et garde son espace.
#[test]
fn counters_stick_to_the_number_in_cjk() {
    let messages: [(&str, &[&str], &[&str]); 9] = [
        ("agent-turn-tools", &["tools"], &[]),
        ("agent-turn-notes", &["notes"], &[]),
        ("agent-turn-secs", &["secs"], &[]),
        ("cli-deck-valid", &["slides"], &[]),
        ("cli-rendered", &["pages"], &["out", "compile", "svg"]),
        ("cli-saved", &["pages"], &["path", "pdf", "sources"]),
        ("cli-opened", &["slides", "assets", "fonts"], &["path"]),
        ("cli-bench-first", &["pages"], &["time"]),
        ("cli-bench-svg", &[], &["time", "each"]),
    ];
    for locale in ["ja", "ko", "zh-Hans", "zh-Hant"] {
        for (id, counters, others) in messages {
            let args: Vec<(&str, FluentValue<'_>)> = counters
                .iter()
                .map(|arg| (*arg, 3.into()))
                .chain(others.iter().map(|arg| (*arg, "x".into())))
                .collect();
            let text = Localizer::new(Some(locale)).tr_with(id, &args);
            let spaced = text.chars().zip(text.chars().skip(1)).any(|(a, b)| a.is_ascii_digit() && b.is_whitespace());
            asserting(&format!("{locale} : {id} = « {text} »")).that(&spaced).is_false();
        }
    }
}
