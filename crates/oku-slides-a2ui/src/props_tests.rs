use serde_json::json;
use speculoos::prelude::*;

use crate::{ButtonIcon, ChoiceOption, StepStatus, TimelineStep, ToolCallDetail};

/// Rust → JSON → Rust round trip: what the host emits reads back identically.
fn round_trip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    serde_json::from_value(serde_json::to_value(value).unwrap()).unwrap()
}

#[test]
fn timeline_step_with_note_detail_and_calls_round_trips() {
    let call = ToolCallDetail {
        label: "widgets".into(),
        status: StepStatus::Failed,
        input: Some("{\n  \"slide\": 4\n}".into()),
        output: Some("aucun widget".into()),
    };
    let steps = vec![
        TimelineStep {
            detail: Some("Corrections en une transaction.\n\nPuis la grille.".into()),
            ..TimelineStep::new("Corrections", StepStatus::Completed)
        },
        TimelineStep { count: Some(1), calls: vec![call], ..TimelineStep::new("widgets", StepStatus::Failed) },
    ];
    assert_that!(round_trip(&steps)).is_equal_to(&steps);
}

#[test]
fn timeline_step_without_detail_or_calls_omits_them() {
    let v = serde_json::to_value(TimelineStep::new("apply", StepStatus::Completed)).unwrap();
    assert_that!(v).is_equal_to(json!({"label": "apply", "status": "completed"}));
}

#[test]
fn timeline_step_with_unknown_key_is_refused() {
    let v = json!({"label": "apply", "status": "completed", "svg": "<svg/>"});
    assert_that!(serde_json::from_value::<TimelineStep>(v)).is_err();
}

#[test]
fn choice_option_with_unknown_key_is_refused() {
    assert_that!(serde_json::from_value::<ChoiceOption>(json!({"id": "a", "label": "A", "color": "red"}))).is_err();
}

#[test]
fn button_icon_down_and_remove_read_back_lowercase() {
    let icons: Vec<ButtonIcon> = serde_json::from_value(json!(["down", "remove"])).unwrap();
    assert_that!(icons).is_equal_to(vec![ButtonIcon::Down, ButtonIcon::Remove]);
}
