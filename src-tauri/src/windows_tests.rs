use speculoos::prelude::*;
use tauri::{PhysicalPosition, PhysicalSize};

use super::*;

#[test]
fn drop_corner_offsets_the_point_by_the_constant() {
    let (x, y) = drop_corner((500.0, 300.0));
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

#[test]
fn physical_rect_keeps_the_desktop_pixels() {
    let r = physical_rect(PhysicalPosition::new(1920, -40), PhysicalSize::new(2160, 1350));
    assert_that!(r).is_equal_to(Rect { x: 1920.0, y: -40.0, w: 2160.0, h: 1350.0 });
}

#[test]
fn host_rect_in_points_is_the_logical_rect() {
    let r = host_rect(true, PhysicalPosition::new(72, 76), PhysicalSize::new(2880, 1800), 2.0);
    assert_that!(r).is_equal_to(Rect { x: 36.0, y: 38.0, w: 1440.0, h: 900.0 });
}

#[test]
fn host_rect_in_pixels_is_the_physical_rect() {
    let r = host_rect(false, PhysicalPosition::new(72, 76), PhysicalSize::new(2880, 1800), 2.0);
    assert_that!(r).is_equal_to(Rect { x: 72.0, y: 76.0, w: 2880.0, h: 1800.0 });
}

/// t-160 : Windows, écran principal à 100 % (0..1920), second écran à 150 %
/// à sa droite. Curseur (`GetCursorPos`) et fenêtres (`GetWindowRect`) sont
/// en pixels physiques du bureau : comparés tels quels (repère de l'hôte
/// hors macOS), le lâcher tombe dans la fenêtre du second écran. Les
/// ramener en points avec des échelles différentes la ratait.
#[test]
fn host_rect_regression_t160_drop_on_a_window_of_a_denser_screen_lands_in_it() {
    let w1 = host_rect(false, PhysicalPosition::new(100, 100), PhysicalSize::new(1440, 900), 1.0);
    let w2 = host_rect(false, PhysicalPosition::new(2000, 100), PhysicalSize::new(2160, 1350), 1.5);
    let cursor = (3900.0, 500.0);
    assert_that!(w2.contains(cursor.0, cursor.1)).is_true();
    assert_that!(w1.contains(cursor.0, cursor.1)).is_false();
}

#[test]
fn page_point_host_in_points_subtracts_the_content_corner() {
    let content = Rect { x: 259.0, y: -1261.0, w: 1440.0, h: 901.0 };
    assert_that!(page_point_host(true, (345.0, -419.0), &content, 2.0)).is_equal_to((86.0, 842.0));
}

#[test]
fn page_point_host_in_pixels_divides_the_offset_by_the_window_scale() {
    let content = Rect { x: 2000.0, y: 130.0, w: 2160.0, h: 1320.0 };
    assert_that!(page_point_host(false, (3500.0, 430.0), &content, 1.5)).is_equal_to((1000.0, 200.0));
}

/// Fenêtre de 1350 px de haut à 150 %, décorations comprises : le pointeur
/// tombe à 27 px (18 points) au-dessus du bas, dans la barre d'onglets.
#[test]
fn drop_corner_physical_puts_the_pointer_in_the_tab_bar() {
    assert_that!(drop_corner_physical((3900.0, 1500.0), 1.5, 1400.0)).is_equal_to((3780.0, 127.0));
}

#[test]
fn cursor_readable_at_the_origin_on_linux_is_false() {
    assert_that!(cursor_readable(PhysicalPosition::new(0.0, 0.0), true)).is_false();
}

#[test]
fn cursor_readable_elsewhere_on_linux_is_true() {
    assert_that!(cursor_readable(PhysicalPosition::new(0.0, 12.0), true)).is_true();
}

#[test]
fn cursor_readable_at_the_origin_off_linux_is_true() {
    assert_that!(cursor_readable(PhysicalPosition::new(0.0, 0.0), false)).is_true();
}
