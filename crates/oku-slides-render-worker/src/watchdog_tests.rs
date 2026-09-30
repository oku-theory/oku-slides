use std::time::{Duration, Instant};

use speculoos::prelude::*;

use super::*;

fn limits() -> Limits {
    Limits::from_host(Some(1_000), Some(1_000))
}

/// Un relevé au repos, sans mesure, à `at`.
fn sample(at: Instant) -> Sample {
    Sample { at, busy_for: None, rss: None, parent_gone: false }
}

#[test]
fn limits_from_host_double_the_timeout() {
    assert_that!(limits().request).is_equal_to(Some(Duration::from_millis(2_000)));
    assert_that!(limits().memory).is_equal_to(Some(1_000));
}

#[test]
fn guard_idle_within_bounds_is_none() {
    let s = Sample { rss: Some(10), ..sample(Instant::now()) };
    assert_that!(Guard::new(limits()).check(&s)).is_none();
}

#[test]
fn guard_request_over_twice_the_timeout_stops() {
    let t0 = Instant::now();
    let under = Sample { busy_for: Some(Duration::from_millis(1_999)), ..sample(t0) };
    let over = Sample { busy_for: Some(Duration::from_millis(2_001)), ..sample(t0) };
    let mut guard = Guard::new(limits());
    assert_that!(guard.check(&under)).is_none();
    assert_that!(guard.check(&over).unwrap_or_default().as_str()).contains("request");
}

#[test]
fn guard_memory_over_the_limit_stops_after_the_grace_only() {
    let t0 = Instant::now();
    let over = |dt: Duration| Sample { rss: Some(1_001), ..sample(t0 + dt) };
    let mut guard = Guard::new(limits());
    // L'hôte vivant a d'abord le temps de tuer le travailleur lui-même.
    assert_that!(guard.check(&over(Duration::ZERO))).is_none();
    assert_that!(guard.check(&over(MEMORY_GRACE / 2))).is_none();
    assert_that!(guard.check(&over(MEMORY_GRACE)).unwrap_or_default().as_str()).contains("memory");
}

#[test]
fn guard_memory_back_under_the_limit_restarts_the_grace() {
    let t0 = Instant::now();
    let mut guard = Guard::new(limits());
    assert_that!(guard.check(&Sample { rss: Some(1_001), ..sample(t0) })).is_none();
    assert_that!(guard.check(&Sample { rss: Some(10), ..sample(t0 + MEMORY_GRACE / 2) })).is_none();
    assert_that!(guard.check(&Sample { rss: Some(1_001), ..sample(t0 + MEMORY_GRACE) })).is_none();
}

#[test]
fn guard_parent_gone_stops_even_at_rest() {
    let s = Sample { parent_gone: true, ..sample(Instant::now()) };
    assert_that!(Guard::new(limits()).check(&s).unwrap_or_default().as_str()).contains("parent");
}

#[test]
fn guard_without_limits_only_watches_the_parent() {
    let t0 = Instant::now();
    let mut guard = Guard::new(Limits::default());
    for dt in [Duration::ZERO, Duration::from_secs(3_600)] {
        let s = Sample { busy_for: Some(Duration::from_secs(3_600)), rss: Some(u64::MAX), ..sample(t0 + dt) };
        assert_that!(guard.check(&s)).is_none();
    }
}

#[test]
fn busy_tracks_the_request_in_progress() {
    let busy = Busy::default();
    let t0 = Instant::now();
    assert_that!(busy.busy_for(t0)).is_none();
    busy.start(t0);
    assert_that!(busy.busy_for(t0 + Duration::from_millis(500))).is_equal_to(Some(Duration::from_millis(500)));
    busy.end();
    assert_that!(busy.busy_for(t0 + Duration::from_secs(10))).is_none();
}

#[test]
fn watch_returns_the_reason_once_a_sample_is_over() {
    // Horloge injectée : chaque relevé avance d'une seconde, la requête reste en cours.
    let busy = Busy::default();
    let t0 = Instant::now();
    busy.start(t0);
    let mut tick = 0u64;
    let reason = watch(limits(), Duration::ZERO, || {
        tick += 1;
        let at = t0 + Duration::from_secs(tick);
        Sample { busy_for: busy.busy_for(at), ..sample(at) }
    });
    assert_that!(tick).is_equal_to(3);
    assert_that!(reason.as_str()).contains("request");
}
