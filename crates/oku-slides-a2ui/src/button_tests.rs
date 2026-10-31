use serde_json::json;
use speculoos::prelude::*;

use crate::{Action, ButtonIcon, Component};

#[test]
fn button_builder_without_flags_equals_a_plain_button() {
    let built = Component::button_builder("b", "Ok", Action::named("x")).build();
    assert_that!(built).is_equal_to(Component::button("b", "Ok", false, Action::named("x")));
}

#[test]
fn button_builder_danger_serializes_its_flag() {
    let c = Component::button_builder("always", "Toujours autoriser", Action::named("x")).danger(true).build();
    assert_that!(serde_json::to_value(&c).unwrap()["danger"].clone()).is_equal_to(json!(true));
}

#[test]
fn button_builder_danger_off_writes_no_flag() {
    let c = Component::button_builder("once", "Une fois", Action::named("x")).primary(true).danger(false).build();
    let json = serde_json::to_value(&c).unwrap();
    assert_that!((json.get("danger").is_none(), json["primary"].clone())).is_equal_to((true, json!(true)));
}

#[test]
fn button_builder_icon_serializes_lowercase() {
    let c = Component::button_builder("up", "Monter", Action::named("extends")).icon(ButtonIcon::Up).build();
    assert_that!(serde_json::to_value(&c).unwrap()["icon"].clone()).is_equal_to(json!("up"));
}
