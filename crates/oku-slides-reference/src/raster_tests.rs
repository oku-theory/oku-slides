use speculoos::prelude::*;

use super::*;
use crate::ReferenceErrorKind;

/// Une image 3 × 2 : chaque pixel porte son rang dans sa composante rouge.
fn ramp() -> Raster {
    Raster::new(3, 2, (0..6u8).flat_map(|i| [i, 0, 0, 255]).collect()).unwrap()
}

#[test]
fn new_with_the_wrong_byte_count_is_a_size_error() {
    let err = Raster::new(3, 2, vec![0; 23]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Size);
}

#[test]
fn png_round_trip_keeps_pixels() {
    let back = Raster::from_png(&ramp().to_png().unwrap()).unwrap();
    assert_that!(back).is_equal_to(ramp());
}

#[test]
fn crop_takes_the_rectangle() {
    let c = ramp().crop(1, 1, 2, 1);
    assert_that!((c.width(), c.height(), c.rgba().to_vec())).is_equal_to((2, 1, vec![4, 0, 0, 255, 5, 0, 0, 255]));
}

#[test]
fn crop_beyond_the_image_is_bounded() {
    let c = ramp().crop(2, 1, 10, 10);
    assert_that!((c.width(), c.height())).is_equal_to((1, 1));
}

#[test]
fn from_png_garbage_is_a_png_error() {
    let err = Raster::from_png(b"nope").unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Png);
}

#[test]
fn scaled_repeats_each_pixel_in_a_square() {
    let s = ramp().crop(0, 0, 2, 1).scaled(2).unwrap();
    let red: Vec<u8> = s.rgba().as_chunks::<4>().0.iter().map(|px| px[0]).collect();
    assert_that!((s.width(), s.height(), red)).is_equal_to((4, 2, vec![0, 0, 1, 1, 0, 0, 1, 1]));
}

#[test]
fn scaled_beyond_the_maximum_side_is_a_size_error() {
    let err = ramp().scaled(MAX_RASTER_SIDE).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Size);
}

#[test]
fn scaled_of_an_empty_raster_is_empty_at_the_scaled_size() {
    let s = ramp().crop(0, 0, 0, 2).scaled(2).unwrap();
    assert_that!((s.width(), s.height(), s.rgba().len())).is_equal_to((0, 4, 0));
}

#[test]
fn scaled_by_zero_is_a_size_error() {
    let err = ramp().scaled(0).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Size);
}
