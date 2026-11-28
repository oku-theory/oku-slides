//! t-1794: a query whose worker timed out was failed for its key, and it and its readers never
//! computed again before an edit. A lost or timed-out worker now runs the query again, bounded.
//!
//! Binary without harness: run with `--render-worker`, it is the query worker; otherwise it is
//! the test. A worker started before the deadline its marker file holds hangs before its
//! handshake, so the first run times out; the run again starts after the deadline and succeeds.
//! The sandboxed worker can read the marker, never write: the compute lasting at least one
//! timeout proves the first run was lost.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use oku_slides_core::Op;
use oku_slides_data::DataId;
use oku_slides_platform::RenderLaunch;
use oku_slides_session::{DeckResults, Fonts, Platform, Session, Settings};
use serde_json::json;
use speculoos::prelude::*;

const TEST: &str = "query_worker_timeout_computes_again";
/// The marker file: a worker started before the deadline it holds (ms since the epoch) hangs.
const HANG_ENV: &str = "OKU_SLIDES_ISSUE_1794_HANG";
/// The query bound: wide enough for a cold worker on a loaded machine.
const TIMEOUT_MS: u64 = 15_000;
/// The deadline, after the marker is written: well after the first worker starts, well before
/// its run times out.
const HANG_WINDOW: Duration = Duration::from_millis(TIMEOUT_MS / 2);

fn main() {
    // Before `exit_if_worker`, which would answer the handshake: the hang must precede it.
    hang_if_marked();
    oku_slides_render_worker::exit_if_worker();
    // Nextest discovery protocol: without a harness, the single test is announced.
    if std::env::args().any(|a| a == "--list") {
        if !std::env::args().any(|a| a == "--ignored") {
            println!("{TEST}: test");
        }
        return;
    }
    query_worker_timeout_computes_again();
    println!("issue_1794_query_worker_timeout_computes_again: ok");
}

/// Milliseconds since the epoch.
fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis())
}

/// As a worker started before the marker's deadline, hangs until its host kills it on timeout.
fn hang_if_marked() {
    let Some(marker) = std::env::var_os(HANG_ENV).map(PathBuf::from) else { return };
    let deadline = std::fs::read_to_string(marker).ok().and_then(|d| d.trim().parse::<u128>().ok());
    if deadline.is_some_and(|d| now_ms() < d) && std::env::args().any(|a| a == "--render-worker") {
        std::thread::sleep(Duration::from_millis(TIMEOUT_MS * 4));
        std::process::exit(1);
    }
}

fn op(v: serde_json::Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op: {e}"))
}

fn query_worker_timeout_computes_again() {
    let data = tempfile::Builder::new().prefix("oku-slides-1794-").tempdir().unwrap_or_else(|e| panic!("{e}"));
    let marker = data.path().join("hang");
    // SAFETY: set before any thread of this process starts; the workers inherit it.
    #[expect(unsafe_code, reason = "the test's workers read the marker path from their environment")]
    unsafe {
        std::env::set_var(HANG_ENV, &marker);
    }
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    settings.render.query_timeout_ms = TIMEOUT_MS;
    let program = std::env::current_exe().unwrap_or_else(|e| panic!("{e}"));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    let queries = platform.clone().with_render_launch(RenderLaunch::Process { program });

    // The deck is built in-process: only the query workers are launched, after the marker.
    let local = platform.with_render_launch(RenderLaunch::InProcess);
    let mut s = Session::new(Fonts::default(), &local).unwrap_or_else(|e| panic!("session: {}", e.one_line()));
    let table = json!({ "generation": "g1", "columns": [{ "id": "c1", "name": "a" }, { "id": "c2", "name": "b" }],
        "rows": [{ "id": "r0", "cells": ["x", 1.0] }, { "id": "r1", "cells": ["y", -1.0] }] });
    let ops = vec![
        op(json!({ "op": "add_data", "id": "m", "name": "ventes", "table": table })),
        op(json!({ "op": "add_data", "id": "q", "name": "top", "sql": "SELECT a, b FROM ventes WHERE b > 0" })),
        op(json!({ "op": "add_data", "id": "q2", "name": "noms", "sql": "SELECT a FROM top" })),
    ];
    s.apply("données", ops, None).unwrap_or_else(|e| panic!("apply: {}", e.one_line()));
    let deck = s.deck().clone();
    let store = queries.bases().unwrap_or_else(|e| panic!("bases: {e}"));

    let deadline = now_ms() + HANG_WINDOW.as_millis();
    std::fs::write(&marker, deadline.to_string()).unwrap_or_else(|e| panic!("marker: {e}"));
    let started = Instant::now();
    let results =
        DeckResults::compute(&deck, &BTreeMap::new(), &queries.render_launch, &queries.settings.render, &store);
    let took = started.elapsed();

    // A query of two rows takes milliseconds: only a timed-out first run lasts this long.
    assert_that!(took).named("compute time").is_greater_than_or_equal_to(Duration::from_millis(TIMEOUT_MS));
    assert_that!(results.issues(&deck)).is_empty();
    let computed = results.results(&deck);
    for data in ["q", "q2"] {
        let id = DataId::try_from(data.to_owned()).unwrap_or_else(|e| panic!("{e:?}"));
        let rows = computed.get(&id).and_then(Option::as_ref).map(|t| t.rows.len());
        assert_that!(rows).named(data).is_equal_to(Some(1));
    }
}
