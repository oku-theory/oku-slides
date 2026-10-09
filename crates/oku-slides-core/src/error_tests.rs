use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use snafu::IntoError;
use speculoos::prelude::*;

use super::{InvalidValueSnafu, OpError, OpErrorKind, ParseValueSnafu, ValueReason};

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

/// Chaque langue a sa propre traduction de chaque raison.
#[test]
fn every_value_reason_key_exists_in_every_locale() {
    let reasons = [
        ValueReason::NoValue,
        ValueReason::NoDataset,
        ValueReason::GridTracks,
        ValueReason::BadItemId,
        ValueReason::MissingItemId,
        ValueReason::TooDeep,
        ValueReason::EmptyPath,
        ValueReason::NotText,
        ValueReason::EmptyFrame,
        ValueReason::BlocksOffGrid,
        ValueReason::OutsideGrid,
        ValueReason::NotTokens,
        ValueReason::NotASlot,
        ValueReason::OutOfBounds,
        ValueReason::Unreadable,
    ];
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        assert!(l.has("op-invalid-value-because"), "{locale} : op-invalid-value-because absent");
        for reason in reasons {
            assert!(l.has(reason.key()), "{locale} : {} absent", reason.key());
        }
    }
}
