use speculoos::prelude::*;

use oku_slides_render::RenderError;

use super::{CROP_PIXEL_PER_PT, crop_box, drawn_box};
use crate::widget_preview::PreviewBox;

/// Un PNG RGBA de `w` × `h`, blanc, avec les pixels `dots` noirs.
fn png(w: u32, h: u32, dots: &[(u32, u32)]) -> Vec<u8> {
    let mut data = vec![255u8; (w * h * 4) as usize];
    for &(x, y) in dots {
        let i = ((y * w + x) * 4) as usize;
        data[i..i + 3].copy_from_slice(&[0, 0, 0]);
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&data).unwrap();
    out
}

const WITHIN: PreviewBox = PreviewBox { x: 0.0, y: 0.0, w: 40.0, h: 40.0 };

#[test]
fn drawn_box_bounds_the_pixels_that_change_in_points() {
    let bare = png(20, 20, &[]);
    let with = png(20, 20, &[(3, 4), (6, 9)]);
    let b = drawn_box(&with, &bare, WITHIN, 0.5).unwrap();
    assert_that!(b).is_equal_to(PreviewBox { x: 6.0, y: 8.0, w: 8.0, h: 12.0 });
}

#[test]
fn drawn_box_ignores_changes_outside_the_element_box() {
    let bare = png(20, 20, &[]);
    let with = png(20, 20, &[(2, 2), (15, 15)]);
    let within = PreviewBox { x: 0.0, y: 0.0, w: 10.0, h: 10.0 };
    let b = drawn_box(&with, &bare, within, 0.5).unwrap();
    assert_that!(b).is_equal_to(PreviewBox { x: 4.0, y: 4.0, w: 2.0, h: 2.0 });
}

#[test]
fn drawn_box_is_none_when_nothing_changes_or_an_image_is_unreadable() {
    let bare = png(20, 20, &[]);
    assert_that!(drawn_box(&bare, &bare, WITHIN, 0.5)).is_none();
    assert_that!(drawn_box(b"pas un png", &bare, WITHIN, 0.5)).is_none();
    assert_that!(drawn_box(&png(10, 10, &[(1, 1)]), &bare, WITHIN, 0.5)).is_none();
}

#[test]
fn drawn_box_counts_a_last_pixel_partly_inside_the_element_box() {
    let bare = png(20, 20, &[]);
    let with = png(20, 20, &[(4, 4)]);
    let within = PreviewBox { x: 0.0, y: 0.0, w: 9.0, h: 9.0 };
    let b = drawn_box(&with, &bare, within, 0.5).unwrap();
    assert_that!(b).is_equal_to(PreviewBox { x: 8.0, y: 8.0, w: 1.0, h: 1.0 });
}

#[test]
fn crop_box_keeps_the_cell_when_a_render_fails() {
    let cell = PreviewBox { x: 0.0, y: 0.0, w: 80.0, h: 80.0 };
    let ok = || Ok(png(20, 20, &[(1, 1)]));
    let failed = || Err(RenderError::stale());
    assert_that!(crop_box(cell, ok, failed)).is_equal_to(cell);
    assert_that!(crop_box(cell, failed, ok)).is_equal_to(cell);
}

#[test]
fn crop_box_bounds_what_the_element_draws() {
    let cell = PreviewBox { x: 0.0, y: 0.0, w: 80.0, h: 80.0 };
    let b = crop_box(cell, || Ok(png(20, 20, &[(2, 3)])), || Ok(png(20, 20, &[])));
    let side = 1.0 / f64::from(CROP_PIXEL_PER_PT);
    assert_that!(b).is_equal_to(PreviewBox { x: 2.0 * side, y: 3.0 * side, w: side, h: side });
}
