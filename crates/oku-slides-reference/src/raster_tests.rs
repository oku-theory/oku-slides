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
