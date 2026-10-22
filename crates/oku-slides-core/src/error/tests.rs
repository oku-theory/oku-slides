use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use snafu::IntoError;
use speculoos::prelude::*;

use super::{
    ExpectedType, InvalidValueSnafu, OpError, OpErrorKind, ParamTypeSnafu, ParseValueSnafu, Shape, ValueReason,
};

fn invalid(reason: ValueReason) -> OpError {
    InvalidValueSnafu { field: "grid", reason }.build().into()
}

/// `op-invalid-value` dit pourquoi la valeur est refusée, dans la langue de l'utilisateur.
#[rstest]
#[case::no_value(ValueReason::NoValue, "the parameter has no value", "le paramètre n'a pas de valeur")]
#[case::no_dataset(ValueReason::NoDataset, "the chart has no dataset", "le graphique n'a pas de jeu de données")]
#[case::grid_tracks(
    ValueReason::GridTracks,
    "a grid has 1 to 24 columns and rows",
    "une grille a de 1 à 24 colonnes et lignes"
)]
#[case::bad_item_id(
    ValueReason::BadItemId,
    "an item id is a non-empty text without “/”",
    "un identifiant d'item est un texte non vide, sans « / »"
)]
#[case::missing_item_id(ValueReason::MissingItemId, "every item needs an id", "chaque item doit avoir un identifiant")]
#[case::too_deep(ValueReason::TooDeep, "elements nest too deeply", "les éléments s'imbriquent trop profondément")]
#[case::empty_path(ValueReason::EmptyPath, "the path is empty", "le chemin est vide")]
#[case::not_text(ValueReason::NotText, "a text or null is expected", "un texte ou null est attendu")]
#[case::empty_frame(
    ValueReason::EmptyFrame,
    "the width and height must be greater than 0",
    "la largeur et la hauteur doivent être supérieures à 0"
)]
#[case::blocks_off_grid(
    ValueReason::BlocksOffGrid,
    "flow slots are laid out by the slide and a grid hides them; move each element to a free slot first",
    "les slots en flux sont disposés par la slide et une grille les cache ; placez d'abord chaque élément dans un slot libre"
)]
#[case::outside_grid(ValueReason::OutsideGrid, "the cell is outside the grid", "la cellule est hors de la grille")]
#[case::not_tokens(ValueReason::NotTokens, "an object or null is expected", "un objet ou null est attendu")]
#[case::not_a_slot(ValueReason::NotASlot, "this parameter is not a slot", "ce paramètre n'est pas un slot")]
#[case::out_of_bounds(
    ValueReason::OutOfBounds,
    "the number is outside its allowed range",
    "le nombre est hors des limites permises"
)]
#[case::valign(ValueReason::Valign, "top, middle or bottom is expected", "top, middle ou bottom est attendu")]
#[case::path(
    ValueReason::Path,
    "the path is made of M, L, C, Q and Z commands followed by their numbers, starting with M",
    "le chemin se compose de commandes M, L, C, Q et Z suivies de leurs nombres, en commençant par M"
)]
fn invalid_value_says_why_in_the_user_language(#[case] reason: ValueReason, #[case] en: &str, #[case] fr: &str) {
    let err = invalid(reason);
    assert_that!(err.kind()).is_equal_to(OpErrorKind::InvalidValue);
    assert_that!(err.value_reason()).is_equal_to(Some(reason));
    assert_that!(err.localize(english())).is_equal_to(format!("Invalid value for grid: {en}"));
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_equal_to(format!("Valeur invalide pour grid : {fr}"));
}

#[test]
fn an_unreadable_value_says_its_form_is_unexpected() {
    let source = serde_json::from_str::<u32>("\"x\"").unwrap_err();
    let err: OpError = ParseValueSnafu { field: "grid" }.into_error(source).into();
    assert_that!(err.value_reason()).is_equal_to(Some(ValueReason::Unreadable));
    assert_that!(err.localize(english()))
        .is_equal_to("Invalid value for grid: the value does not have the expected form".to_owned());
    assert_that!(err.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("Valeur invalide pour grid : la valeur n'a pas la forme attendue".to_owned());
}

/// `Display` garde l'anglais des journaux, raison comprise.
#[test]
fn display_keeps_the_english_reason() {
    assert_that!(invalid(ValueReason::OutsideGrid).to_string())
        .is_equal_to("invalid value for grid: outside the grid".to_owned());
}

fn param_type(expected: ExpectedType) -> OpError {
    ParamTypeSnafu { param: "count", expected }.build().into()
}

/// `Display` garde, mot pour mot, l'anglais d'avant `ExpectedType`, pour chaque construction.
#[rstest]
#[case::text(Shape::Text.required(), "a string")]
#[case::text_or_null(Shape::Text.or_null(), "a string or null")]
#[case::choice(Shape::Choice.required(), "one of its values")]
#[case::choice_or_null(Shape::Choice.or_null(), "one of its values or null")]
#[case::paragraphs(Shape::Paragraphs.required(), "a list of paragraphs")]
#[case::paragraphs_or_null(Shape::Paragraphs.or_null(), "a list of paragraphs or null")]
#[case::elements(Shape::Elements.required(), "a list of elements")]
#[case::flag(Shape::Flag.required(), "a boolean")]
#[case::flag_or_null(Shape::Flag.or_null(), "a boolean or null")]
#[case::number(Shape::Number.or_null(), "a number or null")]
#[case::bounded_number(Shape::BoundedNumber.required(), "a number within its bounds")]
#[case::grid(Shape::Grid.required(), "a grid")]
#[case::grid_or_null(Shape::Grid.or_null(), "a grid or null")]
#[case::chart(Shape::ChartOption.required(), "a chart option")]
#[case::table(Shape::TableOption.required(), "a table option")]
#[case::chart_or_table(Shape::ChartOrTableOption.required(), "a chart or table option")]
#[case::path(Shape::PathOption.required(), "a shape path (an object with d)")]
#[case::items(Shape::Items.required(), "a list of items")]
#[case::items_or_null(Shape::Items.or_null(), "a list of items or null")]
#[case::list(Shape::List.required(), "a list")]
#[case::add_element(Shape::AddElementOnly.required(), "elements, through add_element")]
#[case::set_param(Shape::SetParamOnly.required(), "a value, through set_param")]
fn param_type_display_keeps_the_english_expectation(#[case] expected: ExpectedType, #[case] english: &str) {
    assert_that!(param_type(expected).to_string()).is_equal_to(format!("count expects {english}"));
}

/// Le type refusé dit ce qui était attendu, avec ou sans `null`, dans la langue de l'utilisateur.
#[rstest]
#[case::text(Shape::Text.required(), "a string is expected", "il faut un texte")]
#[case::text_or_null(Shape::Text.or_null(), "a string or null is expected", "il faut un texte ou null")]
#[case::number_or_null(Shape::Number.or_null(), "a number or null is expected", "il faut un nombre ou null")]
#[case::bounded_number(
    Shape::BoundedNumber.required(),
    "a number within its bounds is expected",
    "il faut un nombre dans ses bornes"
)]
#[case::chart(Shape::ChartOption.required(), "a chart option is expected", "il faut une option de graphique")]
#[case::path(Shape::PathOption.required(), "a shape path (an object with d) is expected", "il faut un chemin de forme (un objet à clé d)")]
#[case::add_element(
    Shape::AddElementOnly.required(),
    "elements are added through add_element",
    "les éléments s'ajoutent avec add_element"
)]
fn param_type_says_what_is_expected_in_the_user_language(
    #[case] expected: ExpectedType,
    #[case] en: &str,
    #[case] fr: &str,
) {
    let err = param_type(expected);
    assert_that!(err.kind()).is_equal_to(OpErrorKind::ParamType);
    assert_that!(err.expected_type()).is_equal_to(Some(expected));
    assert_that!(err.localize(english())).is_equal_to(format!("Invalid value for count: {en}"));
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_equal_to(format!("Valeur invalide pour count : {fr}"));
}

/// Chaque forme, avec et sans `null`, se traduit dans chaque langue : l'identifiant brut ne
/// remonte jamais, et `op-param-type` (remplacé) a disparu.
#[test]
fn every_shape_key_exists_in_every_locale() {
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        assert!(l.has("op-param-type-because"), "{locale} : op-param-type-because absent");
        assert!(!l.has("op-param-type"), "{locale} : op-param-type devrait être retiré");
        for shape in Shape::ALL {
            assert!(l.has(shape.key()), "{locale} : {} absent", shape.key());
            for expected in [shape.required(), shape.or_null()] {
                let text = param_type(expected).localize(&l);
                assert!(!text.contains("op-param"), "{locale} : {expected} non traduit : {text}");
            }
        }
    }
}

/// Chaque forme, dans chaque langue, se dit autrement avec `null` et le cite.
#[test]
fn a_nullable_shape_differs_from_a_required_one_in_every_locale() {
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for shape in Shape::ALL {
            let (with, without) = (param_type(shape.or_null()).localize(&l), param_type(shape.required()).localize(&l));
            assert!(with != without, "{locale} : {} ne distingue pas null", shape.key());
            assert!(with.contains("null"), "{locale} : {} ne cite pas null : {with}", shape.key());
        }
    }
}

/// t-866 : l'enregistrement refusé pour un arbre trop profond a sa propre clé, traduite.
#[test]
fn replica_too_deep_localizes_with_its_own_key() {
    let err: super::ReplicaError = super::TreeTooDeepSnafu { max: 1024_usize }.build().into();
    assert_that!(err.kind()).is_equal_to(super::ReplicaErrorKind::TooDeep);
    let fr = Localizer::new(Some("fr"));
    assert_that!(fr.has("replica-too-deep")).is_true();
    assert_that!(err.localize(&fr)).is_equal_to(fr.tr("replica-too-deep"));
    assert_that!(err.localize(&fr)).starts_with("Impossible d'enregistrer cette présentation");
}

/// Les refus d'épinglage de pack, chacun avec ses arguments.
fn pack_errors() -> Vec<(OpError, OpErrorKind, &'static str)> {
    vec![
        (
            super::UnknownNamespaceSnafu { namespace: "acme" }.build().into(),
            OpErrorKind::UnknownNamespace,
            "Ce deck n'a pas de pack acme.",
        ),
        (
            super::PackRequiredSnafu { namespace: "acme", by: "base" }.build().into(),
            OpErrorKind::PackRequired,
            "Le pack acme est nécessaire à base ; retirez d'abord celui-ci.",
        ),
        (
            super::PackInUseSnafu { namespace: "acme", users: vec!["s1".to_owned(), "e1".to_owned()] }.build().into(),
            OpErrorKind::PackInUse,
            "Le pack acme est encore utilisé par 2 éléments.",
        ),
        (
            super::PackRequirementMissingSnafu { namespace: "acme", required: "base" }.build().into(),
            OpErrorKind::PackRequirementMissing,
            "Le pack acme a besoin de base, que ce deck n'a pas.",
        ),
        (
            super::PackKindConflictSnafu { namespace: "acme" }.build().into(),
            OpErrorKind::PackKindConflict,
            "Ce deck a déjà un pack acme de l'autre sorte (widgets ou layouts).",
        ),
    ]
}

#[test]
fn pack_errors_have_their_kind_and_subject() {
    for (err, kind, _) in pack_errors() {
        assert_that!(err.kind()).is_equal_to(kind);
        assert_that!(err.subject()).is_equal_to(Some("acme"));
    }
}

#[test]
fn pack_errors_localize_with_their_arguments_in_french() {
    let fr = Localizer::new(Some("fr"));
    for (err, _, text) in pack_errors() {
        assert_that!(err.localize(&fr).replace(['\u{2068}', '\u{2069}'], "")).is_equal_to(text.to_owned());
    }
}

#[test]
fn pack_error_keys_exist_in_every_locale() {
    let keys = [
        "op-unknown-namespace",
        "op-pack-required",
        "op-pack-in-use",
        "op-pack-requirement-missing",
        "op-pack-kind-conflict",
        "issue-pack-kind-mismatch",
        "issue-pack-kind-conflict",
    ];
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for key in keys {
            assert!(l.has(key), "{locale} : {key} absent");
        }
        for (err, _, _) in pack_errors() {
            let text = err.localize(&l);
            assert!(text.contains("acme"), "{locale} : le pack manque dans {text}");
        }
    }
}

#[test]
fn widget_error_bad_schema_names_its_widget_and_fault() {
    let e = super::WidgetError::bad_schema("cover", super::SchemaFault::PackCategory);
    let shown = e.localize(&Localizer::new(Some("fr")));
    assert_that!((e.kind(), e.widget(), shown == e.localize(english()))).is_equal_to((
        super::WidgetErrorKind::BadSchema(super::SchemaFault::PackCategory),
        Some("cover"),
        false,
    ));
}
