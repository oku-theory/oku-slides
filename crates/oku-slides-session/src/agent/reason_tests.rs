use oku_slides_core::{Deck, Op, WidgetRegistry, apply_all};
use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::AgentFormReason::{self, *};

fn at(path: &str) -> String {
    path.to_owned()
}

/// Une transaction refusée : la slide `s-x` n'existe pas.
fn refused() -> oku_slides_core::TransactionError {
    let mut deck: Deck = serde_json::from_value(json!({ "slides": [] })).unwrap();
    let registry = WidgetRegistry::from_probe(json!({ "packs": [] })).unwrap();
    let op = Op::RemoveSlide { slide: "s-x".into() };
    apply_all(&mut deck, vec![op], &registry).unwrap_err()
}

/// Chaque raison dite en anglais et en français, avec ses arguments.
#[rstest]
#[case::too_long(
    TooLong { at: at("title"), count: 121, max: 120 },
    "title: 121 characters, at most 120",
    "title : 121 caractères, 120 au plus"
)]
#[case::hidden_characters(
    HiddenCharacters { at: at("facts[0]") },
    "facts[0]: invisible or bidirectional control characters are not allowed",
    "facts[0] : les caractères de contrôle invisibles ou bidirectionnels sont interdits"
)]
#[case::count(
    Count { at: at("options"), min: 2, max: 6, count: 1 },
    "options: expected 2 to 6 entries, got 1",
    "options : de 2 à 6 entrées attendues, nombre reçu : 1"
)]
#[case::too_many(
    TooMany { at: at("components"), max: 40, count: 41 },
    "components: at most 40 entries, got 41",
    "components : 40 entrées au plus, nombre reçu : 41"
)]
#[case::too_large(
    TooLarge { at: at("components"), max: 65536, size: 70000 },
    "components: at most 65536 bytes, got 70000",
    "components : 65536 octets au plus, taille reçue : 70000"
)]
#[case::empty(Empty { at: at("facts") }, "facts: must not be empty", "facts : ne doit pas être vide")]
#[case::missing(Missing { at: at("paragraphs") }, "paragraphs: missing", "paragraphs : manquant")]
#[case::duplicate_id(
    DuplicateId { at: at("options[2]"), id: "a".into() },
    "options[2]: duplicate id “a”",
    "options[2] : identifiant « a » en double"
)]
#[case::not_hex_color(
    NotHexColor { at: at("swatch"), value: "bleu".into() },
    "swatch: “bleu” is not a hex color (#rgb, #rgba, #rrggbb or #rrggbbaa)",
    "swatch : « bleu » n'est pas une couleur hexadécimale (#rgb, #rgba, #rrggbb ou #rrggbbaa)"
)]
#[case::not_paragraphs(
    NotParagraphs { at: at("fields[0]") },
    "fields[0]: a rich value must be a list of paragraphs",
    "fields[0] : une valeur riche doit être une liste de paragraphes"
)]
#[case::not_finite(
    NotFinite { at: at("fields[1]") },
    "fields[1]: min and max must be finite numbers",
    "fields[1] : min et max doivent être des nombres finis"
)]
#[case::bounds_inverted(
    BoundsInverted { at: at("fields[1]"), min: 10.0, max: 2.5 },
    "fields[1]: min (10) must be less than max (2.5)",
    "fields[1] : min (10) doit être inférieur à max (2.5)"
)]
#[case::malformed(
    Malformed { at: at("components[0]"), detail: "unknown variant `Video`".into() },
    "components[0]: not in the expected form (unknown variant `Video`)",
    "components[0] : forme inattendue (unknown variant `Video`)"
)]
#[case::refused(
    Refused { at: at("variants[1]"), source: Box::new(refused()) },
    "variants[1]: Operation 0 refused: Slide not found: s-x",
    "variants[1] : Opération 0 refusée : Slide introuvable : s-x"
)]
#[case::slide_removed(
    SlideRemoved { at: at("variants[0]"), slide: "s-1".into() },
    "variants[0]: the variant removes slide s-1",
    "variants[0] : la variante supprime la slide s-1"
)]
#[case::not_available(
    NotAvailable { at: at("fields[0]"), name: "list".into(), tool: "show_params" },
    "fields[0]: list is not available in show_params",
    "fields[0] : list n'est pas disponible dans show_params"
)]
#[case::setting_in_render(
    SettingInRender { at: at("components[0]"), name: "Slider".into() },
    "components[0]: Slider is not available in render; use show_params for settings",
    "components[0] : Slider n'est pas disponible dans render ; les réglages passent par show_params"
)]
#[case::reserved(
    Reserved { at: at("components[0]"), name: "Prompt".into() },
    "components[0]: Prompt is reserved to Oku Slides",
    "components[0] : Prompt est réservé à Oku Slides"
)]
#[case::reserved_style(
    ReservedStyle { at: at("components[0]") },
    "components[0]: the primary, danger and icon styles are reserved to Oku Slides",
    "components[0] : les styles primary, danger et icon sont réservés à Oku Slides"
)]
#[case::not_literal(
    NotLiteral { at: at("components[0]") },
    "components[0]: values must be literal, without data bindings (path)",
    "components[0] : les valeurs doivent être littérales, sans liaison de données (path)"
)]
#[case::unknown_property(
    UnknownProperty { at: at("components[0]"), name: "weight".into() },
    "components[0]: unknown property “weight”",
    "components[0] : propriété inconnue « weight »"
)]
#[case::bad_action(
    BadAction { at: at("components[0]") },
    "components[0]: only a button may carry an action, a suggest action whose prompt is its own label",
    "components[0] : seul un bouton porte une action, une action suggest dont la consigne est son propre libellé"
)]
#[case::no_root(NoRoot, "no component has the id “root”", "aucun composant n'a l'identifiant « root »")]
#[case::too_deep(
    TooDeep { max: 6, level: 7, id: "c6".into() },
    "the component “c6” is at level 7, at most 6",
    "le composant « c6 » est au niveau 7, 6 au plus"
)]
#[case::shared_component(
    SharedComponent { id: "a".into() },
    "the component “a” is used more than once (the components must form a tree)",
    "le composant « a » est utilisé plusieurs fois (les composants doivent former un arbre)"
)]
#[case::unknown_child(UnknownChild { id: "nope".into() }, "unknown child “nope”", "enfant inconnu « nope »")]
#[case::unreachable(
    Unreachable { id: "x".into() },
    "the component “x” cannot be reached from “root”",
    "le composant « x » n'est pas atteignable depuis « root »"
)]
#[case::session_not_called(
    SessionNotCalled { tab: "t-3".into() },
    "the tab t-3 did not answer",
    "l'onglet t-3 n'a pas répondu"
)]
fn every_reason_is_said_in_the_user_language(#[case] reason: AgentFormReason, #[case] en: &str, #[case] fr: &str) {
    assert_that!(reason.localize(english())).is_equal_to(en.to_owned());
    assert_that!(reason.localize(&Localizer::new(Some("fr")))).is_equal_to(fr.to_owned());
}

/// Toutes les langues ont une traduction à elles, jamais le secours anglais ni la clé brute.
#[test]
fn every_reason_key_exists_in_every_locale() {
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for key in KEYS {
            assert!(l.has(&format!("session-agent-form-{key}")), "{locale} : session-agent-form-{key} absent");
        }
    }
}

const KEYS: [&str; 28] = [
    "too-long",
    "hidden-characters",
    "count",
    "too-many",
    "too-large",
    "empty",
    "missing",
    "duplicate-id",
    "not-hex-color",
    "not-paragraphs",
    "not-finite",
    "bounds-inverted",
    "malformed",
    "refused",
    "slide-removed",
    "not-available",
    "setting-in-render",
    "reserved",
    "reserved-style",
    "not-literal",
    "unknown-property",
    "bad-action",
    "no-root",
    "too-deep",
    "shared-component",
    "unknown-child",
    "unreachable",
    "session-not-called",
];

/// `Display` reste l'anglais des journaux, informatif.
#[test]
fn display_keeps_the_english_reason_for_the_logs() {
    let reason = DuplicateId { at: at("options[2]"), id: "a".into() };
    assert_that!(reason.to_string()).is_equal_to("options[2]: duplicate id \"a\"".to_owned());
}
