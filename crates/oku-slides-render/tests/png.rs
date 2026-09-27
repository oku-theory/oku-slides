use oku_slides_core::Deck;
use oku_slides_render::{Fonts, Renderer};
use speculoos::prelude::*;

fn roadmap() -> Deck {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");
    let json = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    Deck::from_json(&json).unwrap_or_else(|e| panic!("{path} : {e}"))
}

#[test]
fn png_first_page_half_scale_is_960_by_540_png() {
    let mut renderer = Renderer::new(Fonts::default()).unwrap();
    let compiled = renderer.render(&roadmap(), &Default::default()).unwrap();
    let png = compiled.png(0, 0.5).unwrap();
    assert_that!(png[..8].to_vec()).is_equal_to(b"\x89PNG\r\n\x1a\n".to_vec());
    // IHDR : largeur puis hauteur, big-endian, octets 16..24.
    let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert_that!((w, h)).is_equal_to((960, 540));
}

#[test]
fn png_missing_page_returns_no_page_error() {
    let mut renderer = Renderer::new(Fonts::default()).unwrap();
    let compiled = renderer.render(&roadmap(), &Default::default()).unwrap();
    let err = compiled.png(999, 0.5).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_render::RenderErrorKind::NoPage);
}

#[test]
fn png_scale_out_of_range_returns_image_error() {
    let mut renderer = Renderer::new(Fonts::default()).unwrap();
    let compiled = renderer.render(&roadmap(), &Default::default()).unwrap();
    let err = compiled.png(0, 100.0).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_render::RenderErrorKind::Image);
}
