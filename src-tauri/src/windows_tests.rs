use speculoos::prelude::*;
use tauri::LogicalPosition;

use super::*;

#[test]
fn drop_corner_offsets_the_point_by_the_constant() {
    let (x, y) = drop_corner(LogicalPosition::new(500.0, 300.0));
    assert_that!(x).is_close_to(500.0 - DROP_OFFSET.0, 0.001);
    assert_that!(y).is_close_to(300.0 - DROP_OFFSET.1, 0.001);
}
