//! t-976: a Mac asleep for hours still checks for updates soon after waking.
//!
//! Timers pause during sleep, so the decision reads the wall clock: after 7 h of wall time
//! but only minutes of timer time since the last check, a check is due.

use std::time::{Duration, SystemTime};

use oku_slides_platform::updates::check_due;
use speculoos::prelude::*;

#[test]
fn check_due_after_sleep_longer_than_the_interval_is_true() {
    let interval = Duration::from_hours(6);
    let last = SystemTime::UNIX_EPOCH + Duration::from_hours(500_000);
    // Ten minutes of awake time, then seven hours asleep.
    let now = last + Duration::from_mins(10) + Duration::from_hours(7);
    assert_that!(check_due(Some(last), now, interval)).is_true();
}
