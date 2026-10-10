use speculoos::prelude::*;

use super::*;

#[test]
fn default_limits_are_conservative_and_non_zero() {
    let l = ReadLimits::default();
    assert_that!((l.max_input_bytes, l.max_part_bytes, l.max_total_bytes)).is_equal_to((
        256 << 20,
        64 << 20,
        512 << 20,
    ));
    assert_that!((l.max_parts, l.max_xml_nodes, l.max_slides)).is_equal_to((10_000, 5_000_000, 2_000));
}
