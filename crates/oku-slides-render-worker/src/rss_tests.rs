use speculoos::prelude::*;

use super::*;

#[test]
fn resident_bytes_of_this_process_is_positive() {
    let mut sys = sysinfo::System::new();
    assert_that!(resident_bytes(&mut sys, std::process::id()).unwrap_or(0)).is_greater_than(0);
}

#[test]
fn resident_bytes_of_missing_process_is_none() {
    let mut sys = sysinfo::System::new();
    assert_that!(resident_bytes(&mut sys, u32::MAX - 1)).is_none();
}
