use speculoos::prelude::*;
use tauri::{LogicalPosition, PhysicalPosition, PhysicalSize};

use super::*;

#[test]
fn drop_corner_offsets_the_point_by_the_constant() {
    let (x, y) = drop_corner(LogicalPosition::new(500.0, 300.0));
    assert_that!(x).is_close_to(500.0 - DROP_OFFSET.0, 0.001);
    assert_that!(y).is_close_to(300.0 - DROP_OFFSET.1, 0.001);
}

#[test]
fn logical_rect_on_a_retina_screen_divides_by_its_scale() {
    let r = logical_rect(PhysicalPosition::new(72, 76), PhysicalSize::new(2880, 1800), 2.0);
    assert_that!(r).is_equal_to(Rect { x: 36.0, y: 38.0, w: 1440.0, h: 900.0 });
}

#[test]
fn logical_rect_on_a_standard_screen_keeps_negative_coordinates() {
    let r = logical_rect(PhysicalPosition::new(1800, -120), PhysicalSize::new(1440, 900), 1.0);
    assert_that!(r).is_equal_to(Rect { x: 1800.0, y: -120.0, w: 1440.0, h: 900.0 });
}

#[test]
fn cursor_points_divides_by_the_primary_screen_scale() {
    assert_that!(cursor_points(PhysicalPosition::new(690.0, -838.0), 2.0)).is_equal_to((345.0, -419.0));
}

/// Journal du 2026-09-26 : `screenY` (WKWebView) valait +1401 pour un lâcher
/// sur w-6, rangée au-dessus de l'écran principal (y de -1261 à -360) : pas
/// le repère des fenêtres. Le curseur lu par l'hôte (tao) l'est ; un point
/// de w-6 dans ce repère y tombe.
#[test]
fn cursor_points_regression_drop_on_a_window_above_the_primary_screen_lands_in_it() {
    let w1 = Rect { x: -411.0, y: -1602.0, w: 1440.0, h: 900.0 };
    let w6 = Rect { x: 259.0, y: -1261.0, w: 1440.0, h: 901.0 };
    let (x, y) = cursor_points(PhysicalPosition::new(690.0, -838.0), 2.0);
    assert_that!(w6.contains(x, y)).is_true();
    assert_that!(w1.contains(x, y)).is_false();
}

#[test]
fn page_point_is_relative_to_the_window_content() {
    let content = Rect { x: 259.0, y: -1261.0, w: 1440.0, h: 901.0 };
    assert_that!(page_point((345.0, -419.0), &content)).is_equal_to((86.0, 842.0));
}
