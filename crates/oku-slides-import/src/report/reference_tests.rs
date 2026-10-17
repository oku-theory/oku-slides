use oku_slides_i18n::{Localize, Localizer, english};
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case(ReferenceCause::Failed)]
#[case(ReferenceCause::Timeout)]
#[case(ReferenceCause::Memory)]
#[case(ReferenceCause::Missing)]
#[case(ReferenceCause::Pages { reference: 7, deck: 8 })]
fn cause_localize_in_every_locale_formats_its_own_message(#[case] cause: ReferenceCause) {
    for tag in oku_slides_i18n::available() {
        let msg = cause.localize(&Localizer::new(Some(tag)));
        assert_that!(msg.is_empty() || msg.contains('{')).named(tag).is_false();
    }
}

#[test]
fn cause_pages_gives_both_counts_in_order() {
    let msg = ReferenceCause::Pages { reference: 7, deck: 8 }.localize(english());
    let (seven, eight) = (msg.find('7'), msg.find('8'));
    assert_that!(seven.zip(eight).is_some_and(|(s, e)| s < e)).is_true();
}

#[test]
fn causes_give_five_distinct_messages_in_french() {
    let fr = Localizer::new(Some("fr"));
    let causes = [
        ReferenceCause::Failed,
        ReferenceCause::Timeout,
        ReferenceCause::Memory,
        ReferenceCause::Missing,
        ReferenceCause::Pages { reference: 7, deck: 8 },
    ];
    let messages: std::collections::BTreeSet<String> = causes.iter().map(|c| c.localize(&fr)).collect();
    assert_that!(messages.len()).is_equal_to(5);
}

#[rstest]
#[case(ReferenceCause::Failed, "failed")]
#[case(ReferenceCause::Timeout, "timeout")]
#[case(ReferenceCause::Memory, "memory")]
#[case(ReferenceCause::Missing, "missing")]
#[case(ReferenceCause::Pages { reference: 7, deck: 8 }, "pages (7 rendered, 8 expected)")]
fn cause_display_names_it_with_its_counts(#[case] cause: ReferenceCause, #[case] expected: &str) {
    assert_that!(cause.to_string()).is_equal_to(expected.to_owned());
}
