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
fn no_locale_defines_a_key_twice() {
    for (locale, sources) in LOCALES {
        let mut seen = BTreeSet::new();
        for line in sources.iter().flat_map(|s| s.lines()) {
            let Some((key, _)) = line.split_once(" = ") else { continue };
            if key.starts_with(['#', ' ']) || key.is_empty() {
                continue;
            }
            asserting(&format!("{locale} : {key} défini deux fois")).that(&seen.insert(key.trim())).is_true();
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
fn has_ignores_the_english_fallback() {
    assert_that!(english().has("deck-untitled")).is_true();
    assert_that!(Localizer::new(Some("fr")).has("deck-untitled")).is_true();
    assert_that!(Localizer::new(Some("fr")).has("does-not-exist")).is_false();
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
fn try_tr_unknown_id_is_none_and_known_id_falls_back_to_english() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(fr.try_tr("does-not-exist")).is_none();
    assert_that!(fr.try_tr("session-widget-param-yes")).is_equal_to(Some("Oui".to_owned()));
    assert_that!(english().try_tr("session-widget-param-yes")).is_equal_to(Some("Yes".to_owned()));
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
fn english_names_are_distinct_and_not_empty() {
    let names: BTreeSet<&str> = ENGLISH_NAMES.iter().map(|(_, n)| *n).filter(|n| !n.is_empty()).collect();
    assert_that!(names.len()).is_equal_to(ENGLISH_NAMES.len());
}

#[test]
fn english_name_names_every_shipped_language() {
    for locale in available() {
        asserting(locale).that(&english_name(locale)).is_some();
    }
}

#[test]
fn english_name_names_the_language_in_english() {
    for (locale, expected) in
        [("fr", "French"), ("en", "English"), ("zh-Hant", "Traditional Chinese"), ("zh-Hans", "Simplified Chinese")]
    {
        asserting(locale).that(&english_name(locale)).is_equal_to(Some(expected));
    }
}

#[test]
fn english_name_of_an_unshipped_code_is_none() {
    assert_that!(english_name("xx")).is_none();
    assert_that!(english_name("fr-CA")).is_none();
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

/// Les refus de liste accordent « item » au plafond ou au plancher selon les règles de pluriel
/// de chaque langue (t-537).
#[test]
fn list_refusals_plurals_follow_each_language() {
    for (locale, id, arg, n, expected) in [
        ("fr", "op-list-full", "max", 1, "La liste L est pleine (1 item au plus)"),
        ("fr", "op-list-full", "max", 5, "La liste L est pleine (5 items au plus)"),
        ("fr", "op-list-minimum", "min", 1, "La liste L doit garder au moins 1 item"),
        ("fr", "op-list-minimum", "min", 5, "La liste L doit garder au moins 5 items"),
        ("en", "op-list-full", "max", 1, "The list L is full (1 item at most)"),
        ("en", "op-list-full", "max", 5, "The list L is full (5 items at most)"),
        ("en", "op-list-minimum", "min", 1, "The list L needs at least 1 item"),
        ("en", "op-list-minimum", "min", 5, "The list L needs at least 5 items"),
        ("ru", "op-list-full", "max", 1, "Список L заполнен (не более 1 пункта)"),
        ("ru", "op-list-full", "max", 2, "Список L заполнен (не более 2 пунктов)"),
        ("ru", "op-list-full", "max", 5, "Список L заполнен (не более 5 пунктов)"),
        ("ru", "op-list-minimum", "min", 1, "В списке L должно быть не менее 1 пункта"),
        ("ru", "op-list-minimum", "min", 2, "В списке L должно быть не менее 2 пунктов"),
        ("ru", "op-list-minimum", "min", 5, "В списке L должно быть не менее 5 пунктов"),
        ("pl", "op-list-full", "max", 1, "Lista L jest pełna (najwyżej 1 pozycja)"),
        ("pl", "op-list-full", "max", 2, "Lista L jest pełna (najwyżej 2 pozycje)"),
        ("pl", "op-list-full", "max", 5, "Lista L jest pełna (najwyżej 5 pozycji)"),
        ("pl", "op-list-minimum", "min", 1, "Lista L musi mieć co najmniej 1 pozycję"),
        ("pl", "op-list-minimum", "min", 2, "Lista L musi mieć co najmniej 2 pozycje"),
        ("pl", "op-list-minimum", "min", 5, "Lista L musi mieć co najmniej 5 pozycji"),
        ("ar", "op-list-full", "max", 1, "القائمة L ممتلئة (بند واحد على الأكثر)"),
        ("ar", "op-list-full", "max", 2, "القائمة L ممتلئة (بندان على الأكثر)"),
        ("ar", "op-list-full", "max", 5, "القائمة L ممتلئة (5 بنود على الأكثر)"),
        ("ar", "op-list-minimum", "min", 1, "يجب أن تحتوي القائمة L على بند واحد على الأقل"),
        ("ar", "op-list-minimum", "min", 2, "يجب أن تحتوي القائمة L على بندين على الأقل"),
        ("ar", "op-list-minimum", "min", 5, "يجب أن تحتوي القائمة L على 5 بنود على الأقل"),
    ] {
        let text = Localizer::new(Some(locale)).tr_with(id, &[("subject", "L".into()), (arg, n.into())]);
        asserting(&format!("{locale} : {id} ({n})")).that(&text).is_equal_to(expected.to_owned());
    }
}

/// L'écart de pages du témoin accorde « page » au nombre de pages du rendu de référence
/// selon les règles de pluriel de chaque langue (t-887).
#[test]
fn import_reference_pages_plurals_follow_each_language() {
    for (locale, n, expected) in [
        ("en", 1, "No visual score: the reference render has 1 page for 8 in the deck"),
        ("en", 7, "No visual score: the reference render has 7 pages for 8 in the deck"),
        ("fr", 1, "Pas d'écart visuel : le rendu de référence a 1 page pour 8 dans le deck"),
        ("de", 1, "Keine visuelle Bewertung: das Referenz-Rendering hat 1 Seite für 8 im Deck"),
        ("ru", 1, "Нет визуальной оценки: в эталонном рендеринге 1 страница при 8 в презентации"),
        ("ru", 3, "Нет визуальной оценки: в эталонном рендеринге 3 страницы при 8 в презентации"),
        ("ru", 5, "Нет визуальной оценки: в эталонном рендеринге 5 страниц при 8 в презентации"),
        ("pl", 1, "Brak oceny wizualnej: renderowanie referencyjne ma 1 stronę na 8 w prezentacji"),
        ("pl", 2, "Brak oceny wizualnej: renderowanie referencyjne ma 2 strony na 8 w prezentacji"),
        ("pl", 5, "Brak oceny wizualnej: renderowanie referencyjne ma 5 stron na 8 w prezentacji"),
        ("ar", 1, "لا توجد درجة مرئية: يحتوي العرض المرجعي على صفحة واحدة مقابل 8 في العرض التقديمي"),
        ("ar", 2, "لا توجد درجة مرئية: يحتوي العرض المرجعي على صفحتين مقابل 8 في العرض التقديمي"),
    ] {
        let args = [("cause", "pages".into()), ("reference", n.into()), ("deck", 8.into())];
        let text = Localizer::new(Some(locale)).tr_with("import-reference-unavailable", &args);
        asserting(&format!("{locale} : {n} page(s)")).that(&text).is_equal_to(expected.to_owned());
    }
}

/// Les variables d'un message sur lesquelles il choisit une variante (`{ $n -> … }`).
fn selectors(source: &str) -> BTreeSet<String> {
    variables(source)
        .into_iter()
        .filter(|name| source.split(&format!("${name}")).skip(1).any(|rest| rest.trim_start().starts_with("->")))
        .collect()
}

/// Un blanc, visible ou non, entre un chiffre et ce qui le suit (espace de largeur nulle et
/// gluons compris : `char::is_whitespace` ne voit pas U+200B).
fn spaced_after_digit(text: &str) -> bool {
    let blank = |c: char| c.is_whitespace() || matches!(c, '\u{200B}' | '\u{2060}' | '\u{FEFF}');
    text.chars().zip(text.chars().skip(1)).any(|(a, b)| a.is_ascii_digit() && blank(b))
}

#[test]
fn spaced_after_digit_sees_invisible_blanks() {
    for (text, spaced) in
        [("3個", false), ("3 個", true), ("3\u{3000}個", true), ("3\u{200B}個", true), ("3\u{2060}秒", true)]
    {
        asserting(&format!("« {text} »")).that(&spaced_after_digit(text)).is_equal_to(spaced);
    }
}

/// En japonais, chinois et coréen, le compteur se colle au nombre (« 3個 », « 3秒 », « 3개 ») :
/// aucun blanc, quel qu'il soit (U+200B compris), entre le chiffre et ce qui le suit, dans chaque
/// message qui compte. Un compteur est une variable sur laquelle le message de référence choisit
/// son pluriel (`{ $n -> … }`), relevée dans les sources plutôt que tenue à la main (t-465) ;
/// les autres paramètres reçoivent un texte sans chiffre : une unité latine (« 12 KB ») n'est pas
/// un compteur et garde son espace. Un message sans compteur n'est pas lu : ses nombres écrits
/// (« 1–4 », « F2 », « 148 MB ») ne sont pas des quantités comptées.
#[test]
fn counters_stick_to_the_number_in_cjk() {
    let counters: std::collections::BTreeMap<String, BTreeSet<String>> = keys(REFERENCE)
        .into_iter()
        .map(|id| {
            let counters = selectors(&source_of(REFERENCE, &id));
            (id, counters)
        })
        .collect();
    let counted = counters.values().filter(|c| !c.is_empty()).count();
    asserting("des messages de référence ont un compteur").that(&counted).is_greater_than(5);
    let mut spaced = Vec::new();
    for locale in ["ja", "ko", "zh-Hans", "zh-Hant"] {
        let localizer = Localizer::new(Some(locale));
        for id in keys(locale) {
            let counters = counters.get(&id).cloned().unwrap_or_default();
            if counters.is_empty() {
                continue;
            }
            let args: Vec<(String, FluentValue<'_>)> = variables(&source_of(locale, &id))
                .union(&counters)
                .map(|arg| (arg.clone(), if counters.contains(arg) { 3.into() } else { "x".into() }))
                .collect();
            let args: Vec<(&str, FluentValue<'_>)> = args.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            let text = localizer.tr_with(&id, &args);
            if spaced_after_digit(&text) {
                spaced.push(format!("{locale} : {id} = « {text} »"));
            }
        }
    }
    assert_that!(spaced.join("\n")).is_equal_to(String::new());
}

/// Classificateurs (compteurs) japonais, chinois et coréens : un nombre qui les précède s'y colle.
const CJK_COUNTERS: &[&str] = &[
    "个",
    "個",
    "张",
    "張",
    "枚",
    "件",
    "页",
    "頁",
    "ページ",
    "秒",
    "次",
    "条",
    "條",
    "项",
    "項",
    "つ",
    "本",
    "回",
    "개",
    "장",
    "초",
    "번",
    "쪽",
];

/// Un chiffre suivi d'un blanc (invisible compris) puis d'un classificateur CJK.
fn counter_detached(text: &str) -> bool {
    let blank = |c: char| c.is_whitespace() || matches!(c, '\u{200B}' | '\u{2060}' | '\u{FEFF}');
    text.char_indices().any(|(i, c)| {
        let rest = &text[i + c.len_utf8()..];
        let after = rest.trim_start_matches(blank);
        c.is_ascii_digit() && after.len() < rest.len() && CJK_COUNTERS.iter().any(|k| after.starts_with(k))
    })
}

#[test]
fn counter_detached_sees_a_blank_before_a_classifier_only() {
    for (text, detached) in
        [("第 3 张", true), ("第3张", false), ("3\u{200B}個", true), ("3 を開く", false), ("148 MB", false)]
    {
        asserting(&format!("« {text} »")).that(&counter_detached(text)).is_equal_to(detached);
    }
}

/// Tout argument numérique, compteur de pluriel ou non (« 第 { $index } 张，共 { $count } 张 ») :
/// chaque variable de chaque message reçoit un nombre ; aucun ne se détache du classificateur
/// qui le suit. Un nombre suivi d'autre chose (particule, unité latine) garde son espace.
#[test]
fn numeric_arguments_stick_to_their_cjk_classifier() {
    let mut detached = Vec::new();
    for locale in ["ja", "ko", "zh-Hans", "zh-Hant"] {
        let localizer = Localizer::new(Some(locale));
        for id in keys(locale) {
            let names = variables(&source_of(locale, &id));
            let args: Vec<(&str, FluentValue<'_>)> = names.iter().map(|n| (n.as_str(), 3.into())).collect();
            let text = localizer.tr_with(&id, &args);
            if counter_detached(&text) {
                detached.push(format!("{locale} : {id} = « {text} »"));
            }
        }
    }
    assert_that!(detached.join("\n")).is_equal_to(String::new());
}

/// Clés de la fonctionnalité des listes, relues dans chaque langue (t-535).
const LIST_KEYS: [&str; 21] = [
    "widget-fault-empty-list",
    "widget-fault-column-type",
    "widget-fault-reserved-name",
    "widget-fault-list-bounds",
    "widget-fault-list-default-too-large",
    "op-item-not-found",
    "op-list-full",
    "op-list-minimum",
    "op-list-cell-too-long",
    "op-list-id-too-long",
    "op-list-too-many-items",
    "issue-empty-item-id",
    "issue-duplicate-item-id",
    "issue-list-bounds",
    "issue-unreadable-item",
    "ui-list-item",
    "ui-list-add",
    "ui-list-remove",
    "ui-list-move",
    "ui-list-full",
    "ui-undo-list-move",
];

/// Source brute d'un message, telle qu'écrite dans les fichiers de la langue, lignes de
/// continuation comprises (variantes d'un sélecteur de pluriel, accolade fermante).
fn source_of(locale: &str, id: &str) -> String {
    let (_, sources) = LOCALES.iter().find(|(l, _)| *l == locale).unwrap();
    let prefix = format!("{id} = ");
    let mut lines = sources.iter().flat_map(|s| s.lines());
    let first = lines
        .find_map(|line| line.strip_prefix(&prefix).map(str::to_owned))
        .unwrap_or_else(|| panic!("{locale} : {id} absent"));
    let continuation = lines.take_while(|line| line.starts_with(' ') || line.starts_with('}'));
    std::iter::once(first).chain(continuation.map(str::to_owned)).collect::<Vec<_>>().join("\n")
}

/// Variables Fluent (`$x`) citées par un message.
fn variables(source: &str) -> BTreeSet<String> {
    source
        .split('$')
        .skip(1)
        .map(|rest| rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect())
        .collect()
}

#[test]
fn list_messages_use_the_reference_variables_in_every_locale() {
    for id in LIST_KEYS {
        let reference = variables(&source_of(REFERENCE, id));
        for locale in crate::available() {
            asserting(&format!("{locale} : {id}")).that(&variables(&source_of(locale, id))).is_equal_to(&reference);
        }
    }
}

#[test]
fn resolve_picks_the_requested_shipped_language() {
    assert_that(&resolve(Some("fr-CA"))).is_equal_to("fr");
    assert_that(&resolve(Some("zh_TW"))).is_equal_to("zh-Hant");
}

#[test]
fn resolve_agrees_with_the_localizer_without_building_it() {
    for requested in [None, Some("de"), Some("en"), Some("xx"), Some("")] {
        assert_that(&resolve(requested)).is_equal_to(Localizer::new(requested).locale());
    }
}

#[test]
fn resolve_always_names_a_shipped_language() {
    assert_that(&shipped(resolve(Some("klingon")))).is_some();
}
