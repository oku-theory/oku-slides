use speculoos::prelude::*;

use super::{INSTRUCTIONS, WIDGETS};

/// Le paragraphe de la règle des langues, seul : le reste des consignes cite aussi les outils.
fn languages() -> &'static str {
    INSTRUCTIONS.find("Languages:").map_or("", |at| &INSTRUCTIONS[at..])
}

#[test]
fn instructions_ask_for_report_not_suggest_next() {
    assert_that!(INSTRUCTIONS).contains("End every turn with report");
    assert_that!(INSTRUCTIONS).contains("Use render only");
    assert_that!(INSTRUCTIONS).does_not_contain("suggest_next");
}

#[test]
fn languages_rule_names_both_languages_of_the_context() {
    // Les clés que `agent_context` écrit (oku-slides-session).
    assert_that!(languages()).contains("<oku-slides-context>");
    assert_that!(languages()).contains("interface language");
    assert_that!(languages()).contains("deck language");
}

/// Les deux phrases de la règle : ce que lit l'utilisateur, puis le contenu du deck.
fn interface_and_deck_sentences() -> (&'static str, &'static str) {
    let rule = languages();
    let at = rule.find("Write deck content").unwrap_or(rule.len());
    let (interface, deck) = rule.split_at(at);
    let end = deck.find(". ").map_or(deck.len(), |i| i + 1);
    (interface, &deck[..end])
}

const USER_FACING: [&str; 6] = [
    "progress notes",
    "label: and item-label: of widget descriptors",
    "report",
    "questions of ask_choice",
    "title and labels of show_params",
    "labels of show_variants",
];

const DECK_CONTENT: [&str; 6] = [
    "set_param values",
    "set_widget_trial values",
    "the rewordings of ask_rewrite",
    "ask_choice options that are candidate deck text",
    "the values a show_params field writes",
    "new slides",
];

#[test]
fn languages_rule_puts_what_the_user_reads_in_the_interface_language_sentence_only() {
    let (interface, deck) = interface_and_deck_sentences();
    assert_that!(interface).contains("in the interface language");
    for part in USER_FACING {
        asserting(part).that(&interface.contains(part)).is_true();
        asserting(part).that(&deck.contains(part)).is_false();
    }
}

#[test]
fn languages_rule_puts_deck_content_in_the_deck_language_sentence_only() {
    let (interface, deck) = interface_and_deck_sentences();
    assert_that!(deck).contains("in the deck language");
    for part in DECK_CONTENT {
        asserting(part).that(&deck.contains(part)).is_true();
        asserting(part).that(&interface.contains(part)).is_false();
    }
}

#[test]
fn languages_rule_never_translates_the_deck_unasked() {
    assert_that!(languages()).contains("Never translate the deck unless the user asks");
}

#[test]
fn languages_rule_prefers_the_declared_deck_language_then_infers_it_then_follows_the_user() {
    let rule = languages();
    let order: Vec<Option<usize>> =
        ["is the declared one", "if unset, infer it from the slides", "if the deck is empty"]
            .map(|p| rule.find(p))
            .into();
    assert_that!(order.iter().all(Option::is_some)).is_true();
    assert_that!(order.windows(2).all(|w| w[0] < w[1])).is_true();
}

#[test]
fn languages_rule_switches_only_when_the_user_writes_in_another_language() {
    let (interface, _) = interface_and_deck_sentences();
    assert_that!(interface).contains("if the user writes to you in another language");
    assert_that!(interface).contains("pasted or dictated text");
}

#[test]
fn languages_rule_asks_when_the_slides_contradict_the_declared_language() {
    let rule = languages();
    let declared = rule.find("is the declared one").unwrap_or(usize::MAX);
    let ask = rule.find("ask with ask_confirm which language to write in").unwrap_or(0);
    assert_that!(ask).is_greater_than(declared);
}

#[test]
fn widgets_mention_slide_category_and_optional_label_and_default() {
    for part in ["category: \"slide\"", "label:", "default:"] {
        asserting(part).that(&WIDGETS.contains(part)).is_true();
    }
}

#[test]
fn widgets_document_lists_and_their_ops() {
    for part in ["p.list", "list-item", "edit_list", "set_param", "\"defaults\": true", "etapes/<step id>/taches"] {
        asserting(part).that(&WIDGETS.contains(part)).is_true();
    }
}

#[test]
fn widgets_import_line_lists_list_item() {
    let at = WIDGETS.find("fields-stack").unwrap_or_else(|| panic!("ligne d'import absente"));
    let line: String = WIDGETS[at..].chars().take(80).collect();
    assert_that!(line).contains("list-item");
}
