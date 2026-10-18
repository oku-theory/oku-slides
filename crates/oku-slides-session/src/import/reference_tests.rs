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
    assert_that!(spans(&[1, 3, 4, 6], 3)).is_equal_to(vec![span(1, 3), span(4, 6)]);
}

#[test]
fn spans_cut_before_covering_more_than_size_source_slides() {
    assert_that!(spans(&[1, 2, 50, 51], 20)).is_equal_to(vec![span(1, 2), span(50, 51)]);
}

#[test]
fn spans_of_one_slide_each_far_apart_are_one_slide_each() {
    assert_that!(spans(&[1, 30, 90], 20)).is_equal_to(vec![span(1, 1), span(30, 30), span(90, 90)]);
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

#[rstest]
#[case::office2pdf(ReferenceWitness::Office2Pdf, Witness::Office2Pdf)]
#[case::soffice(ReferenceWitness::Soffice { program: "soffice".into(), timeout_ms: 1 }, Witness::LibreOffice)]
fn limits_of_names_the_limits_of_each_witness(#[case] witness: ReferenceWitness, #[case] expected: Witness) {
    assert_that!(limits_of(&witness)).is_equal_to(expected);
}
