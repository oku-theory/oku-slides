use rstest::rstest;
use speculoos::prelude::*;

use super::*;

#[rstest]
#[case::inner([100.0, 50.0, 300.0, 150.0], [84, 34, 232, 132])]
#[case::before_the_page([-50.0, -20.0, 150.0, 80.0], [0, 0, 166, 96])]
#[case::after_the_page([900.0, 500.0, 1000.0, 600.0], [884, 484, 76, 56])]
#[case::outside_the_page([2000.0, 900.0, 2100.0, 950.0], [959, 539, 1, 1])]
fn crop_box_takes_the_signed_edges_plus_the_margin_within_the_page(#[case] edges: [f64; 4], #[case] crop: [u32; 4]) {
    assert_that!(crop_box(edges, 16, 960, 540)).is_equal_to(Some(crop));
}

#[test]
fn crop_box_of_an_empty_page_is_none() {
    assert_that!(crop_box([0.0, 0.0, 10.0, 10.0], 16, 0, 540)).is_none();
}

#[test]
fn edges_of_a_frame_starting_before_the_source_page_stay_negative() {
    let g = Geometry { rect: Frame { x: 10.0, y: 0.0, w: 480.0, h: 270.0 }, ppp: 2.0, width: 960, scale: 1.0 };
    let edges = g.edges(Frame { x: 0.0, y: 5.0, w: 100.0, h: 20.0 });
    assert_that!(edges).is_equal_to([-20.0, 10.0, 180.0, 50.0]);
}
