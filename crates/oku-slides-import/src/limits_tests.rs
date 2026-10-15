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

#[test]
fn default_limits_output_budget_matches_reopenable_sources() {
    assert_that!(ReadLimits::default().max_output_bytes).is_equal_to(256 << 20);
}

#[test]
fn default_limits_replica_budget_matches_reopenable_history() {
    assert_that!(ReadLimits::default().max_replica_bytes).is_equal_to(32 << 20);
}

#[test]
fn default_limits_image_pixels_bound_a_decoded_image_to_256_mib() {
    assert_that!(ReadLimits::default().max_image_pixels * 4).is_equal_to(256 << 20);
}
