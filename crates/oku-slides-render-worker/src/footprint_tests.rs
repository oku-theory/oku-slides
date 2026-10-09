use speculoos::prelude::*;

use super::*;

#[test]
fn phys_footprint_counts_memory_this_process_touches() {
    let before = phys_footprint(std::process::id()).unwrap_or(u64::MAX);
    let touched = vec![1u8; 64 * 1024 * 1024];
    let after = phys_footprint(std::process::id()).unwrap_or(0);
    assert_that!(after.saturating_sub(before)).is_greater_than_or_equal_to(48 * 1024 * 1024);
    drop(std::hint::black_box(touched));
}

#[test]
fn phys_footprint_of_a_reaped_process_is_none() {
    // Un pid valide (dans les bornes de `c_int`) dont le processus n'existe
    // plus : c'est `proc_pid_rusage` qui échoue.
    let mut child = std::process::Command::new("/usr/bin/true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    assert_that!(phys_footprint(pid)).is_none();
}
