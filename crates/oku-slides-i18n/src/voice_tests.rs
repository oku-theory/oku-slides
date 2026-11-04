use std::collections::BTreeSet;

use speculoos::prelude::*;

use super::voice_forms;
use crate::{LOCALES, Localizer, available};

const VERBS: [&str; 9] = ["replace", "merge", "discard", "delete", "use-api", "install", "quit", "close", "import"];

/// Every `voice-*` key a locale must define.
fn all_keys() -> Vec<String> {
    let mut keys: Vec<String> =
        ["yes", "no", "negation", "stop", "next", "previous", "first", "last", "exit", "wake", "filler", "option"]
            .iter()
            .map(|k| format!("voice-{k}"))
            .collect();
    keys.extend((1..=9).map(|n| format!("voice-ordinal-{n}")));
    keys.extend(VERBS.iter().map(|v| format!("voice-verb-{v}")));
    keys
}

fn set(locale: &str, keys: &[&str]) -> BTreeSet<String> {
    keys.iter().flat_map(|k| voice_forms(locale, k)).collect()
}

const NAV: [&str; 5] = ["voice-next", "voice-previous", "voice-first", "voice-last", "voice-exit"];
const COMMANDS: [&str; 8] = [
    "voice-yes",
    "voice-no",
    "voice-negation",
    "voice-stop",
    "voice-next",
    "voice-previous",
    "voice-first",
    "voice-last",
];

#[test]
fn voice_forms_every_locale_defines_every_key() {
    for locale in available() {
        let own = Localizer::new(Some(locale));
        for key in all_keys() {
            asserting(&format!("{locale}: {key} missing")).that(&own.has(&key)).is_true();
            asserting(&format!("{locale}: {key} has no form")).that(&voice_forms(locale, &key).is_empty()).is_false();
        }
    }
}

#[test]
fn voice_forms_messages_hold_no_placeable() {
    for (locale, sources) in LOCALES {
        let own = all_keys();
        let spoken = |line: &&str| line.split_once(" = ").is_some_and(|(key, _)| own.iter().any(|k| k == key));
        for line in sources.iter().flat_map(|s| s.lines()).filter(spoken) {
            asserting(&format!("{locale}: {line}")).that(&line.contains(['{', '}'])).is_false();
        }
    }
}

fn assert_kinds_disjoint(label: &str, sets: &dyn Fn(&[&str]) -> BTreeSet<String>) {
    let overlap = |name: &str, a: &BTreeSet<String>, b: &BTreeSet<String>| {
        let shared: Vec<&String> = a.intersection(b).collect();
        asserting(&format!("{label}: {name} overlap {shared:?}")).that(&shared.is_empty()).is_true();
    };
    overlap("yes and no", &sets(&["voice-yes"]), &sets(&["voice-no"]));
    overlap("yes and negation", &sets(&["voice-yes"]), &sets(&["voice-negation"]));
    overlap("stop and nav", &sets(&["voice-stop"]), &sets(&NAV));
    overlap("filler and commands", &sets(&["voice-filler"]), &sets(&COMMANDS));
}

#[test]
fn voice_forms_kinds_are_disjoint_per_locale() {
    for locale in available() {
        assert_kinds_disjoint(locale, &|keys| set(locale, keys));
    }
}

#[test]
fn voice_forms_kinds_are_disjoint_across_locales() {
    assert_kinds_disjoint("all locales", &|keys| available().into_iter().flat_map(|l| set(l, keys)).collect());
}

#[test]
fn voice_forms_are_folded_and_split_on_commas() {
    assert_that!(voice_forms("de", "voice-verb-merge")).contains("zusammenfuhren".to_owned());
    assert_that!(voice_forms("fr", "voice-yes")).contains("oui".to_owned());
}

#[test]
fn voice_forms_unshipped_locale_or_key_is_empty() {
    assert_that!(voice_forms("xx", "voice-yes")).is_empty();
    assert_that!(voice_forms("en", "voice-nope")).is_empty();
}

#[test]
fn voice_forms_region_tag_selects_its_language() {
    assert_that!(voice_forms("fr-CA", "voice-yes")).is_equal_to(voice_forms("fr", "voice-yes"));
}

/// R48: a permission is answered by voice, so its read-aloud names the first yes and no forms.
#[test]
fn voice_say_permission_names_yes_and_no_in_every_locale() {
    for locale in available() {
        let said = crate::fold(&Localizer::new(Some(locale)).tr("voice-say-permission"));
        for key in ["voice-yes", "voice-no"] {
            let form = voice_forms(locale, key).into_iter().next().unwrap_or_default();
            asserting(&format!("{locale}: {key} form {form:?} in {said:?}")).that(&said.contains(&form)).is_true();
        }
    }
}

#[test]
fn voice_say_permission_fr_offers_oui_or_non() {
    let said = Localizer::new(Some("fr")).tr("voice-say-permission");
    assert_that!((said.contains("oui"), said.contains("non"), said.contains("écran"))).is_equal_to((true, true, false));
}
