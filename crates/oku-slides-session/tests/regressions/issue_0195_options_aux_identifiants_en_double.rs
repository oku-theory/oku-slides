//! t-195 : une liste `select` de `show_params` aux identifiants en double était acceptée.

use oku_slides_a2ui::ChoiceOption;
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_session::SessionErrorKind;
use oku_slides_session::agent::{AgentFormReason, FieldKind, ParamField, ShowParams};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{fixture_path, session};

fn option(id: &str) -> ChoiceOption {
    ChoiceOption { id: id.into(), label: id.to_uppercase(), hint: None }
}

fn select(options: Vec<ChoiceOption>) -> ShowParams {
    let field = ParamField {
        label: "Thème".into(),
        kind: FieldKind::Select,
        value: json!("a"),
        op_template: json!({ "op": "set_var", "key": "theme", "value": "$value" }),
        min: None,
        max: None,
        step: None,
        options,
        columns: Vec::new(),
        item_label: None,
    };
    ShowParams { title: "x".into(), slide: "s-libre".into(), fields: vec![field] }
}

#[test]
fn select_with_a_duplicate_option_id_is_refused() {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{e}"));
    let err = s.params_surface("show-1", &select(vec![option("a"), option("b"), option("a")])).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
    assert!(matches!(
        err.agent_form_reason(),
        Some(AgentFormReason::DuplicateId { at, id }) if at == "fields[0].options[2]" && id == "a"
    ));
    // Dite dans la langue de l'utilisateur, raison comprise (t-650).
    assert_that!(err.localize(&Localizer::new(Some("fr")))).is_equal_to(
        "La demande de l'agent est invalide : fields[0].options[2] : identifiant « a » en double".to_owned(),
    );
}

#[test]
fn select_with_distinct_option_ids_is_accepted() {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(s.params_surface("show-1", &select(vec![option("a"), option("b")]))).is_ok();
}

#[test]
fn select_with_too_many_options_is_refused() {
    let mut s = session();
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{e}"));
    let many = (0..=oku_slides_session::agent::forms::SELECT_OPTIONS_MAX).map(|i| option(&format!("o{i}"))).collect();
    let err = s.params_surface("show-1", &select(many)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::AgentForm);
}
