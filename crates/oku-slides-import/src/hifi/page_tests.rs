use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn frame(page: &Page, (x, y, w, h): (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
    let f = page.frame(&Frame::new(x, y, w, h));
    (f.x, f.y, f.w, f.h)
}

#[test]
fn page_of_a_16_9_source_doubles_without_offset_or_loss() {
    let page = Page::new(Size { w: 960.0, h: 540.0 });
    assert_that!(page.len(1.0)).is_equal_to(2.0);
    assert_that!(frame(&page, (0.0, 0.0, 960.0, 540.0))).is_equal_to((0.0, 0.0, 1920.0, 1080.0));
    assert_that!(frame(&page, (36.5, 20.25, 100.0, 50.0))).is_equal_to((73.0, 40.5, 200.0, 100.0));
    assert_that!(page.loss()).is_none();
}

#[test]
fn page_of_a_4_3_source_is_centered_horizontally_and_reported() {
    // 720 × 540 : échelle 2, 1440 pt de large, 240 pt de bande de chaque côté.
    let page = Page::new(Size { w: 720.0, h: 540.0 });
    assert_that!(page.len(1.0)).is_equal_to(2.0);
    assert_that!(frame(&page, (0.0, 0.0, 720.0, 540.0))).is_equal_to((240.0, 0.0, 1440.0, 1080.0));
    assert_that!(page.loss()).is_equal_to(Some(LossKind::PageFitted { width: 720, height: 540 }));
}

#[test]
fn page_of_a_wider_source_is_centered_vertically() {
    // 960 × 400, plus large que 16:9 : échelle 2, 1920 × 800, 140 pt de bande en haut et en bas.
    let page = Page::new(Size { w: 960.0, h: 400.0 });
    assert_that!(page.len(1.0)).is_equal_to(2.0);
    assert_that!(frame(&page, (0.0, 0.0, 960.0, 400.0))).is_equal_to((0.0, 140.0, 1920.0, 800.0));
    assert_that!(page.loss()).is_equal_to(Some(LossKind::PageFitted { width: 960, height: 400 }));
}

#[rstest]
#[case(1.0 / 3.0, 0.67)]
#[case(10.004, 20.01)]
#[case(-0.001, 0.0)]
fn page_len_rounds_to_the_hundredth(#[case] pt: f64, #[case] expected: f64) {
    let page = Page::new(Size { w: 960.0, h: 540.0 });
    assert_that!(page.len(pt)).is_equal_to(expected);
    assert_that!(page.len(pt).is_sign_negative()).is_false();
}

#[test]
fn page_of_an_a4_source_rounds_its_reported_size() {
    let page = Page::new(Size { w: 841.89, h: 595.28 });
    assert_that!(page.loss()).is_equal_to(Some(LossKind::PageFitted { width: 842, height: 595 }));
    let f = page.frame(&Frame::new(0.0, 0.0, 841.89, 595.28));
    assert_that!((f.y, f.h)).is_equal_to((0.0, 1080.0));
}

#[test]
fn page_of_a_degenerate_size_is_the_oku_page() {
    let page = Page::new(Size { w: 0.0, h: f64::NAN });
    assert_that!(page.len(1.0)).is_equal_to(1.0);
    assert_that!(page.loss()).is_none();
}

#[test]
fn page_cover_is_the_whole_oku_page() {
    let page = Page::new(Size { w: 720.0, h: 540.0 });
    let f = page.frame(&page.cover());
    assert_that!((f.x, f.y, f.w, f.h)).is_equal_to((0.0, 0.0, PAGE_W, PAGE_H));
}

#[test]
fn source_rect_of_a_4_3_source_is_the_centered_band() {
    let r = Page::new(Size { w: 720.0, h: 540.0 }).source_rect();
    assert_that!((r.x, r.y, r.w, r.h)).is_equal_to((240.0, 0.0, 1440.0, 1080.0));
}
