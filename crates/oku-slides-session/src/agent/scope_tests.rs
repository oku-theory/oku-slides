use speculoos::prelude::*;
use uuid::Uuid;

use super::AgentScope;

const PACK: Uuid = Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);

#[test]
fn scope_json_matches_the_interface_shape() {
    assert_that!(serde_json::to_string(&AgentScope::Slides).unwrap()).is_equal_to(r#"{"kind":"slides"}"#.to_owned());
    let widget = AgentScope::Widget { pack: PACK, name: "frise".into() };
    assert_that!(serde_json::to_string(&widget).unwrap())
        .is_equal_to(r#"{"kind":"widget","pack":"01234567-89ab-cdef-0123-456789abcdef","name":"frise"}"#.to_owned());
    let back: AgentScope =
        serde_json::from_str(r#"{"kind":"atelier","pack":"01234567-89ab-cdef-0123-456789abcdef"}"#).unwrap();
    assert_that!(back).is_equal_to(AgentScope::Atelier { pack: PACK });
}

#[test]
fn scope_file_name_of_a_widget_has_a_fixed_safe_length() {
    let long = AgentScope::Widget { pack: PACK, name: format!("../{}", "é".repeat(300)) }.file_name();
    let short = AgentScope::Widget { pack: PACK, name: "a".into() }.file_name();
    assert_that!(long.len()).is_equal_to(short.len());
    assert_that!(long.chars().all(|c| c.is_ascii_alphanumeric() || c == '.')).is_true();
    assert_that!(long.len()).is_less_than(100);
}

#[test]
fn scope_file_name_differs_per_pack_and_kind() {
    let other = Uuid::from_u128(7);
    assert_that!(AgentScope::Slides.file_name()).is_equal_to("slides.json".to_owned());
    assert_that!(AgentScope::Atelier { pack: PACK }.file_name())
        .is_not_equal_to(AgentScope::Atelier { pack: other }.file_name());
    assert_that!(AgentScope::Widget { pack: PACK, name: "a".into() }.file_name())
        .is_not_equal_to(AgentScope::Widget { pack: other, name: "a".into() }.file_name());
}
