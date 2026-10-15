use speculoos::prelude::*;

use super::*;

fn hex(s: &str) -> Rgba {
    let v = u32::from_str_radix(s.trim_start_matches('#'), 16).unwrap();
    Rgba::new((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

fn close(x: Oklab, l: f64, a: f64, b: f64) -> bool {
    (x.l - l).abs() < 1e-4 && (x.a - a).abs() < 1e-4 && (x.b - b).abs() < 1e-4
}

#[test]
fn oklab_of_white_black_and_red_matches_the_reference_values() {
    assert_that!(close(oklab(hex("#ffffff")), 1.0, 0.0, 0.0)).is_true();
    assert_that!(close(oklab(hex("#000000")), 0.0, 0.0, 0.0)).is_true();
    assert_that!(close(oklab(hex("#ff0000")), 0.627_955, 0.224_863, 0.125_846)).is_true();
}

#[test]
fn delta_e_between_black_and_white_is_one_hundred() {
    assert_that!(delta_e(oklab(hex("#000000")), oklab(hex("#ffffff")))).is_close_to(100.0, 1e-3);
}

#[test]
fn oklab_chroma_of_a_grey_is_zero() {
    assert_that!(oklab(hex("#808080")).chroma()).is_less_than(1e-6);
}
