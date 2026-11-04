use oku_slides_i18n::Localizer;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

#[test]
fn spoken_receipt_without_title_is_silent() {
    assert_that!(SpokenText::receipt(" \n ", &["Un fait".to_owned()], &fr())).is_none();
}

#[test]
fn spoken_text_drops_isolation_marks_and_line_breaks() {
    let said = SpokenText::receipt("Fond\u{2068} bleu\u{2069}\nnuit", &[], &fr()).unwrap_or_else(|| panic!("receipt"));
    assert_that!(said.text()).is_equal_to("Fond bleu nuit");
    assert_that!(said.lang()).is_equal_to("fr");
    assert_that!(said.kind()).is_equal_to(SpokenKind::Receipt);
}

#[test]
fn spoken_text_clipped_to_spoken_max() {
    let options: Vec<String> = (0..12).map(|n| format!("{n} {}", "x".repeat(55))).collect();
    let refs: Vec<&str> = options.iter().map(String::as_str).collect();
    let said = SpokenText::question("Laquelle ?", &refs, &fr());
    assert_that!(said.text().chars().count()).is_equal_to(SPOKEN_MAX);
}

#[rstest]
#[case(None)]
#[case(Some("Oui, le faire"))]
fn spoken_agent_confirm_reads_question_then_how_to_answer(#[case] verb: Option<&str>) {
    let l = fr();
    let said = SpokenText::agent_confirm("Sûr ?", verb, &l);
    let how = SpokenText::confirm(verb, &l);
    assert_that!(said.text().to_owned()).is_equal_to(format!("Sûr ? {}", how.text()));
    assert_that!(said.kind()).is_equal_to(SpokenKind::Question);
}

#[rstest]
#[case(SpokenFailure::Refused, "voice-say-agent-refused")]
#[case(SpokenFailure::Stopped, "voice-say-agent-stopped")]
#[case(SpokenFailure::NoStart, "voice-say-agent-no-start")]
#[case(SpokenFailure::NoTools, "voice-say-agent-no-tools")]
fn spoken_error_reads_its_template_in_locale(#[case] failure: SpokenFailure, #[case] key: &str) {
    let l = fr();
    let said = SpokenText::error(failure, &l);
    assert_that!(said.text().to_owned()).is_equal_to(l.tr(key));
    assert_that!(said.text().to_owned()).is_not_equal_to(key.to_owned());
    assert_that!(said.kind()).is_equal_to(SpokenKind::Error);
}

#[rstest]
#[case::command("Titre [[slnc 5000]] fin")]
#[case::nested("[[[[rate 10]]]]")]
#[case::odd("a [[[ b ]]] c")]
fn spoken_receipt_never_holds_synthesizer_commands(#[case] title: &str) {
    let said = SpokenText::receipt(title, &[], &fr()).unwrap_or_else(|| panic!("receipt"));
    assert_that!((said.text().contains("[["), said.text().contains("]]"))).is_equal_to((false, false));
}

#[test]
fn spoken_question_option_never_holds_synthesizer_commands() {
    let said = SpokenText::question("Laquelle ?", &["[[volm 0.1]] Bleu"], &fr());
    assert_that!((said.text().contains("[["), said.text().contains("]]"))).is_equal_to((false, false));
}
