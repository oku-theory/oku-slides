//! t-630 : une adresse de lien commençant par `<` s'écrivait `[a](<x)`, lu par
//! CommonMark comme une destination entre chevrons non fermée ; une adresse
//! finissant par `\` s'écrivait `[a](x\)`, où `\)` ne ferme pas la destination.
//! Le `<` initial et tout `\` sont désormais encodés (`%3C`, `%5C`).

use oku_slides_core::{Paragraph, Run};
use oku_slides_session::paragraphs_to_text;
use speculoos::prelude::*;

fn linked(url: &str) -> String {
    let run = Run { link: Some(url.into()), ..Run::plain("a") };
    paragraphs_to_text(&[Paragraph { runs: vec![run], list: None, heading: None }])
}

#[test]
fn link_starting_with_angle_bracket_encodes_it() {
    assert_that!(linked("<x")).is_equal_to("[a](%3Cx)".to_owned());
}

#[test]
fn link_ending_with_backslash_encodes_it() {
    assert_that!(linked("x\\")).is_equal_to("[a](x%5C)".to_owned());
}
