use speculoos::prelude::*;

use super::{Step, step};

#[test]
fn step_detached_without_window_opens_it() {
    assert_that!(step(true, false)).is_equal_to(Step::Open);
}

#[test]
fn step_attached_with_window_closes_it() {
    assert_that!(step(false, true)).is_equal_to(Step::Close);
}

#[test]
fn step_detached_with_window_keeps_it() {
    assert_that!(step(true, true)).is_equal_to(Step::Keep);
}

#[test]
fn step_attached_without_window_keeps_it() {
    assert_that!(step(false, false)).is_equal_to(Step::Keep);
}
