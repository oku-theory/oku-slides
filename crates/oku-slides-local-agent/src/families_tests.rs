use rstest::rstest;
use speculoos::prelude::*;

use super::{Family, families_of, parse_route, route_grammar};

#[test]
fn families_every_mcp_tool_has_a_family() {
    let orphans: Vec<String> = oku_slides_mcp::tool_names().into_iter().filter(|t| families_of(t).is_empty()).collect();
    assert_that!(orphans).is_empty();
}

#[rstest]
#[case("edit", vec![Family::Edit])]
#[case("configure, then troubleshoot", vec![Family::Configure, Family::Troubleshoot])]
#[case("edit edit files present", vec![Family::Edit, Family::Files])]
#[case("Troubleshoot.", vec![Family::Troubleshoot])]
#[case("ask_report edit", vec![Family::Edit])]
#[case("nothing useful", vec![])]
fn parse_route_keeps_two_known_families(#[case] reply: &str, #[case] expected: Vec<Family>) {
    assert_that!(parse_route(reply)).is_equal_to(expected);
}

#[test]
fn families_summaries_stay_under_fifteen_words() {
    for f in Family::ROUTABLE {
        assert_that!(f.summary().split_whitespace().count()).is_less_than(15);
    }
}

#[test]
fn route_grammar_names_every_routable_family() {
    let grammar = route_grammar();
    assert_that!(grammar.as_str()).starts_with("root ::=");
    for f in Family::ROUTABLE {
        assert_that!(grammar.as_str()).contains(format!("\"{}\"", f.key()).as_str());
    }
}

#[rstest]
#[case("packs")]
#[case("edit_pack")]
#[case("save_pack")]
#[case("close_pack")]
#[case("delete_pack")]
fn families_of_a_pack_tool_is_packs(#[case] tool: &str) {
    assert_that!(families_of(tool)).is_equal_to(&[Family::Packs][..]);
}

#[rstest]
#[case("show_packs", &[Family::Edit][..])]
#[case("add_pack", &[Family::Edit][..])]
#[case("remove_pack", &[Family::Edit][..])]
#[case("set_theme_extends", &[Family::Packs][..])]
fn families_of_the_deck_pack_tools_follow_their_neighbours(#[case] tool: &str, #[case] expected: &[Family]) {
    assert_that!(families_of(tool)).is_equal_to(expected);
}

/// Les layouts donnent les kinds de slide : utiles pour éditer comme pour l'atelier.
#[test]
fn families_of_layouts_is_edit_and_packs() {
    assert_that!(families_of("layouts")).is_equal_to(&[Family::Edit, Family::Packs][..]);
}

#[test]
fn families_no_mcp_tool_name_contains_template() {
    let named: Vec<String> = oku_slides_mcp::tool_names().into_iter().filter(|t| t.contains("template")).collect();
    assert_that!(named).is_empty();
}
