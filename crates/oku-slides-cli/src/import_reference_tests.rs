use oku_slides_render::RenderError;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn span(first: u32, last: u32) -> SlideSpan {
    SlideSpan { first, last }
}

#[test]
fn spans_cut_the_slides_in_groups_of_size() {
    assert_that!(spans(&[1, 2, 3, 4, 5], 2)).is_equal_to(vec![span(1, 2), span(3, 4), span(5, 5)]);
}

#[test]
fn spans_cover_the_slides_the_import_omitted_in_between() {
    assert_that!(spans(&[1, 2, 5, 6], 3)).is_equal_to(vec![span(1, 5), span(6, 6)]);
}

#[test]
fn spans_of_no_slide_are_empty() {
    assert_that!(spans(&[], SPAN)).is_empty();
}

#[rstest]
#[case(RenderError::timeout(10), ReferenceCause::Timeout)]
#[case(RenderError::out_of_memory(2048), ReferenceCause::Memory)]
#[case(RenderError::worker_lost("fermé"), ReferenceCause::Failed)]
#[case(RenderError::stale(), ReferenceCause::Failed)]
fn cause_of_a_render_error_names_the_bound(#[case] e: RenderError, #[case] expected: ReferenceCause) {
    assert_that!(cause(&e)).is_equal_to(expected);
}
