use oku_slides_i18n::available;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::en("en", "yes, merge", ConfirmVerb::Merge)]
#[case::en_order("en", "merge yes please", ConfirmVerb::Merge)]
#[case::fr("fr", "oui, fusionner", ConfirmVerb::Merge)]
#[case::fr_accents("fr", "Oui, remplacez", ConfirmVerb::Replace)]
#[case::de("de", "ja, zusammenführen", ConfirmVerb::Merge)]
#[case::ja("ja", "はい、統合", ConfirmVerb::Merge)]
#[case::ja_unspaced("ja", "はい統合", ConfirmVerb::Merge)]
#[case::zh_unspaced("zh-Hans", "是合并", ConfirmVerb::Merge)]
#[case::th_unspaced("th", "ใช่ลบ", ConfirmVerb::Delete)]
fn yes_no_yes_plus_verb_returns_yes(#[case] locale: &str, #[case] said: &str, #[case] verb: ConfirmVerb) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.yes_no(said, Some(verb))).is_equal_to(YesNo::Yes);
}

#[rstest]
#[case::fr_ne_pas("fr", "ne fusionne pas", ConfirmVerb::Merge)]
#[case::fr_non("fr", "non, fusionner", ConfirmVerb::Merge)]
#[case::fr_pas("fr", "pas fusionner", ConfirmVerb::Merge)]
#[case::fr_oui_ne_pas("fr", "oui, ne fusionne pas", ConfirmVerb::Merge)]
#[case::en_dont("en", "don't merge", ConfirmVerb::Merge)]
#[case::en_yes_dont("en", "yes, don't merge", ConfirmVerb::Merge)]
#[case::en_not("en", "do not merge", ConfirmVerb::Merge)]
#[case::de_nicht("de", "nicht zusammenführen", ConfirmVerb::Merge)]
#[case::de_ja_nicht("de", "ja, nicht zusammenführen", ConfirmVerb::Merge)]
#[case::ja_no("ja", "いいえ、統合", ConfirmVerb::Merge)]
#[case::ja_negated("ja", "はい統合しない", ConfirmVerb::Merge)]
#[case::ja_negated_alone("ja", "統合しない", ConfirmVerb::Merge)]
#[case::ja_polite_negated("ja", "はい統合しません", ConfirmVerb::Merge)]
fn yes_no_negated_verb_never_returns_yes(#[case] locale: &str, #[case] said: &str, #[case] verb: ConfirmVerb) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.yes_no(said, Some(verb))).is_not_equal_to(YesNo::Yes);
}

#[rstest]
#[case::en("en", "I would merge it", ConfirmVerb::Merge)]
#[case::en_bare("en", "merge", ConfirmVerb::Merge)]
#[case::fr("fr", "je vais fusionner ça", ConfirmVerb::Merge)]
#[case::fr_bare("fr", "fusionner", ConfirmVerb::Merge)]
#[case::de("de", "zusammenführen", ConfirmVerb::Merge)]
#[case::ja("ja", "統合", ConfirmVerb::Merge)]
#[case::yes_without_the_verb("en", "yes", ConfirmVerb::Merge)]
fn yes_no_bare_verb_in_sentence_is_unclear(#[case] locale: &str, #[case] said: &str, #[case] verb: ConfirmVerb) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.yes_no(said, Some(verb))).is_equal_to(YesNo::Unclear);
}

#[rstest]
#[case::en("en", "yes and no")]
#[case::fr("fr", "oui non")]
#[case::de("de", "ja nein")]
#[case::ja("ja", "はい、いいえ")]
fn yes_no_yes_and_no_is_unclear(#[case] locale: &str, #[case] said: &str) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.yes_no(said, None)).is_equal_to(YesNo::Unclear);
}

#[rstest]
#[case::en_yes("en", "Yes!", YesNo::Yes)]
#[case::fr_yes("fr", "oui, s'il vous plaît", YesNo::Yes)]
#[case::de_no("de", "nein", YesNo::No)]
#[case::ja_no("ja", "いいえ", YesNo::No)]
#[case::en_chatter("en", "what do you mean", YesNo::Unclear)]
#[case::empty("en", "", YesNo::Unclear)]
fn yes_no_without_verb_reads_the_whole_utterance(#[case] locale: &str, #[case] said: &str, #[case] want: YesNo) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.yes_no(said, None)).is_equal_to(want);
}

#[rstest]
#[case::en_zero("en", "zero", 5)]
#[case::digit_zero("en", "0", 5)]
#[case::en_out_of_range("en", "fourth", 3)]
#[case::digit_out_of_range("en", "7", 3)]
#[case::fr_out_of_range("fr", "cinquième", 4)]
#[case::de_out_of_range("de", "dritte", 2)]
#[case::ja_out_of_range("ja", "三番目", 2)]
#[case::chatter("en", "the first of many", 5)]
#[case::huge("en", "99999999999999999999999", 5)]
fn ordinal_zero_or_out_of_range_is_none(#[case] locale: &str, #[case] said: &str, #[case] count: usize) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.ordinal(said, count)).is_none();
}

#[rstest]
#[case::en("en", "second", 5, 1)]
#[case::en_please("en", "the third, please", 5, 2)]
#[case::fr("fr", "deuxième", 5, 1)]
#[case::de("de", "erste", 5, 0)]
#[case::ja("ja", "三番目", 5, 2)]
#[case::ko("ko", "첫 번째", 5, 0)]
#[case::digit("en", "4", 5, 3)]
#[case::last("en", "five", 5, 4)]
fn ordinal_in_range_is_zero_based(#[case] locale: &str, #[case] said: &str, #[case] count: usize, #[case] want: usize) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.ordinal(said, count)).is_equal_to(Some(want));
}

#[test]
fn label_ambiguous_match_is_none() {
    let lexicon = Lexicon::for_locales("en", None);
    assert_that!(lexicon.label("blue", &["Dark blue", "Light blue"])).is_none();
    assert_that!(lexicon.label("dark blue", &["Dark blue", "Light blue"])).is_equal_to(Some(0));
    assert_that!(lexicon.label("dark blue", &["Dark blue", "Dark blue"])).is_none();
    assert_that!(lexicon.label("red", &["Dark blue", "Light blue"])).is_none();
    assert_that!(lexicon.label("", &["Dark blue"])).is_none();
}

#[rstest]
#[case::fr("fr", "ecran", &["Écran", "Plan"], 0)]
#[case::fr_upper("fr", "ÉCRAN", &["ecran", "plan"], 0)]
#[case::de("de", "grosse", &["Klein", "Größe"], 1)]
fn label_match_ignores_accents(#[case] locale: &str, #[case] said: &str, #[case] labels: &[&str], #[case] want: usize) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.label(said, labels)).is_equal_to(Some(want));
}

#[rstest]
#[case::fr("fr", "Oku, Écris le titre", "Écris le titre")]
#[case::fr_no_comma("fr", "oku écris le titre", "écris le titre")]
#[case::en("en", "Hey Oku, Add a slide", "Add a slide")]
#[case::alone("en", "Oku", "")]
fn wake_returns_original_slice(#[case] locale: &str, #[case] said: &str, #[case] want: &str) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.wake(said)).is_equal_to(Some(want));
}

#[rstest]
#[case::fr("fr", "ok, où en étions-nous")]
#[case::en("en", "okay, where were we")]
#[case::middle("en", "write oku in the title")]
#[case::empty("en", "")]
#[case::de("de", "okay, wo waren wir")]
fn wake_false_positive_is_none(#[case] locale: &str, #[case] said: &str) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.wake(said)).is_none();
}

#[rstest]
#[case::german("de", "ÖKU, Ärger", "Ärger")]
#[case::japanese("ja", "Oku、次へ", "次へ")]
#[case::eszett("de", "oku, Straße Größe", "Straße Größe")]
#[case::katakana("ja", "オク、次へ", "次へ")]
fn wake_slice_after_multibyte_folding_is_on_char_boundary(
    #[case] locale: &str,
    #[case] said: &str,
    #[case] want: &str,
) {
    let lexicon = Lexicon::for_locales(locale, None);
    let got = lexicon.wake(said);
    assert_that!(got).is_equal_to(Some(want));
    assert_that!(got.is_some_and(|rest| said.ends_with(rest))).is_true();
}

#[rstest]
#[case::en("en", "stop")]
#[case::en_please("en", "Stop, please!")]
#[case::fr("fr", "arrête")]
#[case::de("de", "stopp")]
#[case::ja("ja", "ストップ")]
fn stop_whole_utterance_returns_true(#[case] locale: &str, #[case] said: &str) {
    assert_that!(Lexicon::for_locales(locale, None).stop(said)).is_true();
}

#[rstest]
#[case::sentence("en", "do not stop the music")]
#[case::nav("en", "next")]
#[case::empty("en", "")]
fn stop_otherwise_returns_false(#[case] locale: &str, #[case] said: &str) {
    assert_that!(Lexicon::for_locales(locale, None).stop(said)).is_false();
}

#[rstest]
#[case::en_next("en", "next slide", Nav::Next)]
#[case::en_prev("en", "back", Nav::Previous)]
#[case::fr_first("fr", "première diapo", Nav::First)]
#[case::de_last("de", "letzte Folie", Nav::Last)]
#[case::ja_next("ja", "次へ", Nav::Next)]
#[case::en_exit("en", "exit", Nav::Exit)]
#[case::fr_quit("fr", "quitter", Nav::Exit)]
#[case::en_quit("en", "quit", Nav::Exit)]
fn nav_whole_utterance_returns_its_command(#[case] locale: &str, #[case] said: &str, #[case] want: Nav) {
    assert_that!(Lexicon::for_locales(locale, None).nav(said)).is_equal_to(Some(want));
}

#[test]
fn nav_unknown_or_mixed_is_none() {
    let lexicon = Lexicon::for_locales("en", None);
    assert_that!(lexicon.nav("next and back")).is_none();
    assert_that!(lexicon.nav("add a slide")).is_none();
    assert_that!(lexicon.nav("")).is_none();
}

#[test]
fn verb_forms_every_confirm_verb_defined() {
    for locale in available() {
        let lexicon = Lexicon::for_locales(locale, None);
        for verb in ConfirmVerb::ALL {
            asserting(&format!("{locale}: {verb:?}")).that(&lexicon.verb(verb).is_empty()).is_false();
        }
    }
}

/// The danger button shows the verb the user must say: its label is one of its spoken forms.
#[test]
fn verb_button_label_is_a_spoken_form_in_every_locale() {
    for locale in available() {
        let lexicon = Lexicon::for_locales(locale, None);
        let l = oku_slides_i18n::Localizer::new(Some(locale));
        let yes = voice_forms(locale, "voice-yes").into_iter().next().unwrap_or_default();
        for verb in ConfirmVerb::ALL {
            let said = format!("{yes} {}", l.tr(verb.label_key()));
            asserting(&format!("{locale}: {said}")).that(&lexicon.yes_no(&said, Some(verb))).is_equal_to(YesNo::Yes);
        }
    }
}

#[test]
fn confirm_verb_all_is_in_discriminant_order() {
    for (index, verb) in ConfirmVerb::ALL.iter().enumerate() {
        assert_that!(*verb as usize).is_equal_to(index);
    }
}

#[test]
fn verb_forms_are_folded_token_sequences() {
    let lexicon = Lexicon::for_locales("fr", None);
    assert_that!(lexicon.verb(ConfirmVerb::Merge).to_vec()).is_equal_to(vec![
        vec!["fusionner".to_owned()],
        vec!["fusionne".to_owned()],
        vec!["fusionnez".to_owned()],
    ]);
}

#[test]
fn for_locales_merges_ui_and_dictation() {
    let merged = Lexicon::for_locales("fr", Some("en-US"));
    assert_that!(merged.yes_no("oui", None)).is_equal_to(YesNo::Yes);
    assert_that!(merged.yes_no("yes", None)).is_equal_to(YesNo::Yes);
    assert_that!(merged.stop("arrête")).is_true();
    assert_that!(merged.stop("stop")).is_true();
    let ui_only = Lexicon::for_locales("fr", None);
    assert_that!(ui_only.yes_no("yes", None)).is_equal_to(YesNo::Unclear);
    assert_that!(merged.verb(ConfirmVerb::Merge).len()).is_greater_than(ui_only.verb(ConfirmVerb::Merge).len());
}

#[test]
fn for_locales_same_locale_twice_does_not_duplicate_forms() {
    let once = Lexicon::for_locales("fr", None);
    let twice = Lexicon::for_locales("fr", Some("fr-FR"));
    assert_that!(twice.verb(ConfirmVerb::Quit).len()).is_equal_to(once.verb(ConfirmVerb::Quit).len());
}

#[test]
fn for_locales_unshipped_dictation_adds_nothing() {
    let lexicon = Lexicon::for_locales("fr", Some("xx"));
    assert_that!(lexicon.yes_no("oui", None)).is_equal_to(YesNo::Yes);
}

#[test]
fn yes_no_unsegmentable_unspaced_token_is_unclear() {
    let lexicon = Lexicon::for_locales("ja", None);
    assert_that!(lexicon.yes_no("統合しません", Some(ConfirmVerb::Merge))).is_equal_to(YesNo::Unclear);
    assert_that!(lexicon.yes_no("統合しない", Some(ConfirmVerb::Merge))).is_equal_to(YesNo::No);
}

#[rstest]
#[case::ja_katakana("ja", "オク次へ", "次へ")]
#[case::ja_punctuated("ja", "オク、次へ", "次へ")]
#[case::zh_prefix("zh-Hans", "奥库写标题", "写标题")]
#[case::zh_hant_prefix("zh-Hant", "嘿奧庫寫標題", "寫標題")]
#[case::filler_first_fr("fr", "Euh, Oku, écris le titre", "écris le titre")]
#[case::filler_first_en("en", "Um, hey Oku, add a slide", "add a slide")]
fn wake_works_on_unspaced_scripts_and_behind_filler(#[case] locale: &str, #[case] said: &str, #[case] want: &str) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.wake(said)).is_equal_to(Some(want));
}

#[rstest]
#[case::fr_article("fr", "la deuxième", 5, 1)]
#[case::fr_article_elided("fr", "l'un", 5, 0)]
#[case::en_option("en", "option two", 5, 1)]
#[case::fr_option_digits("fr", "option 2", 5, 1)]
#[case::en_number_digits("en", "number 3", 5, 2)]
#[case::de_article("de", "die dritte", 5, 2)]
#[case::zh_option("zh-Hans", "选项二", 5, 1)]
fn ordinal_accepts_articles_and_the_option_word(
    #[case] locale: &str,
    #[case] said: &str,
    #[case] count: usize,
    #[case] want: usize,
) {
    let lexicon = Lexicon::for_locales(locale, None);
    assert_that!(lexicon.ordinal(said, count)).is_equal_to(Some(want));
}

#[test]
fn ordinal_option_word_alone_is_none() {
    assert_that!(Lexicon::for_locales("en", None).ordinal("option", 5)).is_none();
}

#[rstest]
#[case::de("de", "die nächste Folie", Nav::Next)]
#[case::fr("fr", "la diapo suivante", Nav::Next)]
#[case::es("es", "la diapositiva anterior", Nav::Previous)]
fn nav_accepts_articles(#[case] locale: &str, #[case] said: &str, #[case] want: Nav) {
    assert_that!(Lexicon::for_locales(locale, None).nav(said)).is_equal_to(Some(want));
}

#[rstest]
#[case::en("en", "choose the Zebra theme", &["Zebra", "Tiger"])]
#[case::fr("fr", "mets le titre en sombre", &["Sombre", "Clair"])]
fn label_in_a_sentence_is_none(#[case] locale: &str, #[case] said: &str, #[case] labels: &[&str]) {
    assert_that!(Lexicon::for_locales(locale, None).label(said, labels)).is_none();
}

#[test]
fn label_with_a_refusal_is_none_unless_it_names_a_label() {
    let fr = Lexicon::for_locales("fr", None);
    assert_that!(fr.label("pas Écran", &["Écran", "Plan"])).is_none();
    assert_that!(fr.label("non", &["Oui", "Non"])).is_equal_to(Some(1));
    let en = Lexicon::for_locales("en", None);
    assert_that!(en.label("don't choose Zebra", &["Zebra", "Tiger"])).is_none();
}

/// Words of every form of a kind, one token each.
fn token_set(forms: &[Vec<String>]) -> std::collections::BTreeSet<&str> {
    forms.iter().flatten().map(String::as_str).collect()
}

fn form_set(forms: &[Vec<String>]) -> std::collections::BTreeSet<&Vec<String>> {
    forms.iter().collect()
}

fn assert_lexicon_is_disjoint(label: &str, lexicon: &Lexicon) {
    let nav: Vec<Vec<String>> = lexicon.nav.iter().flat_map(|(_, forms)| forms.clone()).collect();
    let refusals: Vec<Vec<String>> = lexicon.no.iter().chain(&lexicon.negation).cloned().collect();
    let shared = |a: &[Vec<String>], b: &[Vec<String>]| -> Vec<Vec<String>> {
        form_set(a).intersection(&form_set(b)).map(|f| (*f).clone()).collect()
    };
    let verbs: Vec<Vec<String>> = lexicon.verbs.iter().flatten().cloned().collect();
    for (name, hits) in [
        ("yes/no", shared(&lexicon.yes, &lexicon.no)),
        ("yes/negation", shared(&lexicon.yes, &lexicon.negation)),
        ("stop/nav", shared(&lexicon.stop, &nav)),
        ("verb/yes", shared(&verbs, &lexicon.yes)),
        ("verb/refusal", shared(&verbs, &refusals)),
        (
            "filler/command",
            shared(&lexicon.filler, &[lexicon.yes.clone(), refusals.clone(), lexicon.stop.clone(), nav].concat()),
        ),
    ] {
        asserting(&format!("{label}: {name} share {hits:?}")).that(&hits.is_empty()).is_true();
    }
    let fillers = token_set(&lexicon.filler);
    let leaked: Vec<&str> = token_set(&refusals).intersection(&fillers).copied().collect();
    asserting(&format!("{label}: refusal tokens that are fillers {leaked:?}")).that(&leaked.is_empty()).is_true();
    let yes_in_refusal =
        refusals.iter().any(|r| lexicon.yes.iter().any(|y| r.windows(y.len()).any(|w| w == y.as_slice())));
    asserting(&format!("{label}: a refusal contains a yes form")).that(&yes_in_refusal).is_false();
}

#[test]
fn lexicon_kinds_are_disjoint_as_tokens_per_locale() {
    for locale in available() {
        assert_lexicon_is_disjoint(locale, &Lexicon::for_locales(locale, None));
    }
}

#[test]
fn lexicon_kinds_are_disjoint_as_tokens_across_locales() {
    let every = available();
    assert_lexicon_is_disjoint("all locales", &Lexicon::from_locales(&every));
}

#[rstest]
#[case::en_a("en", None, "A", 0)]
#[case::en_b("en", None, "b", 1)]
#[case::pt_o("pt", None, "O", 2)]
fn label_made_of_filler_words_is_picked_by_its_text(
    #[case] locale: &str,
    #[case] dictation: Option<&str>,
    #[case] said: &str,
    #[case] want: usize,
) {
    let lexicon = Lexicon::for_locales(locale, dictation);
    assert_that!(lexicon.label(said, &["A", "B", "O"])).is_equal_to(Some(want));
}

#[test]
fn label_filler_label_needs_the_whole_text() {
    let en = Lexicon::for_locales("en", None);
    assert_that!(en.label("please", &["A", "Blue"])).is_none();
    assert_that!(en.label("a blue", &["A", "Blue"])).is_equal_to(Some(1));
}

#[test]
fn ordinal_pt_um_with_en_filler_is_first() {
    let lexicon = Lexicon::for_locales("en", Some("pt"));
    assert_that!(lexicon.ordinal("um", 3)).is_equal_to(Some(0));
    assert_that!(Lexicon::for_locales("en", None).ordinal("um", 3)).is_none();
}

#[rstest]
#[case::en_hmm("en", "hmm", true)]
#[case::en_line("en", "Um, thank you.", true)]
#[case::fr_euh("fr", "euh… alors", true)]
#[case::fr_hmm("fr", "hmm", true)]
#[case::fr_hum("fr", "Hum…", true)]
#[case::punctuation("en", "...", true)]
#[case::en_words("en", "um, add a slide", false)]
fn is_filler_only_reads_only_filler_words(#[case] locale: &str, #[case] said: &str, #[case] want: bool) {
    assert_that!(Lexicon::for_locales(locale, None).is_filler_only(said)).is_equal_to(want);
}

#[test]
fn tokens_long_unspaced_utterance_segments_in_bounded_time() {
    let lexicon = Lexicon::for_locales("zh-Hans", None);
    let said = "是".repeat(20_000);
    let began = std::time::Instant::now();
    let pieces = lexicon.tokens(&said).len();
    // Quadratic segmentation takes many seconds here; bounded by the longest piece, milliseconds.
    assert_that!((pieces, began.elapsed() < std::time::Duration::from_secs(2))).is_equal_to((20_000, true));
}
