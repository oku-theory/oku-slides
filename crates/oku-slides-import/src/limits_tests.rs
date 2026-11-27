use rstest::rstest;
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
    assert_that!(ReadLimits::default().max_output_bytes).is_equal_to(1024 << 20);
}

#[test]
fn default_limits_replica_budget_matches_reopenable_history() {
    assert_that!(ReadLimits::default().max_replica_bytes).is_equal_to(64 << 20);
}

#[test]
fn default_limits_image_pixels_bound_a_decoded_image_to_256_mib() {
    assert_that!(ReadLimits::default().max_image_pixels * 4).is_equal_to(256 << 20);
}

#[rstest]
#[case::small_worker(256, 64 << 20, 32 << 20, 1_000_000)]
#[case::default_worker(512, 128 << 20, 64 << 20, 1_000_000)]
#[case::large_worker(2048, 512 << 20, 64 << 20, 1_000_000)]
fn for_extract_derives_bounds_from_memory(
    #[case] memory_mib: u64,
    #[case] total: u64,
    #[case] part: u64,
    #[case] nodes: u32,
) {
    let l = ReadLimits::for_extract(1 << 20, memory_mib << 20);
    assert_that!((l.max_input_bytes, l.max_total_bytes, l.max_part_bytes, l.max_xml_nodes)).is_equal_to((
        1 << 20,
        total,
        part,
        nodes,
    ));
    assert_that!(l.max_total_bytes).is_less_than_or_equal_to((memory_mib << 20) / 4);
    assert_that!(l.max_part_bytes).is_less_than_or_equal_to((memory_mib << 20) / 8);
    let d = ReadLimits::default();
    assert_that!((l.max_parts, l.max_slides, l.max_image_pixels, l.max_output_bytes, l.max_replica_bytes))
        .is_equal_to((d.max_parts, d.max_slides, d.max_image_pixels, d.max_output_bytes, d.max_replica_bytes));
}

#[test]
fn read_limits_round_trip_through_json() {
    let l = ReadLimits::for_extract(42, 512 << 20);
    let back: ReadLimits = serde_json::from_str(&serde_json::to_string(&l).unwrap()).unwrap();
    assert_that!(back).is_equal_to(l);
}
