use std::path::Path;

use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::replica_full;
use crate::test_support::{fresh_dir, session_with, settings_on};
use crate::{Localize, SessionErrorKind};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

#[rstest]
#[case::at_89_percent(89, false)]
#[case::at_90_percent(90, true)]
#[case::past_the_cap(101, true)]
fn replica_full_starts_at_90_percent(#[case] percent: u64, #[case] full: bool) {
    assert_that!(replica_full(percent * 1000, 100_000)).is_equal_to(full);
}

#[test]
fn growing_ops_refused_near_the_replica_cap_fr() {
    let dir = fresh_dir("oku-slides-replica-full-");
    let mut s = session_with(settings_on(dir.path()));
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    let size = s.doc.sources.replica().snapshot().unwrap_or_else(|e| panic!("{e}")).len();
    // A cap the replica already fills to 90 %: only removals pass.
    s.source_limits.replica_bytes = size * 10 / 9;
    let add = json!({ "op": "add_slide", "slide": { "id": "s-neuve", "kind": "essentiel/content" } });
    let e = s
        .apply("ajout", vec![serde_json::from_value(add).unwrap_or_else(|e| panic!("{e}"))], None)
        .err()
        .unwrap_or_else(|| panic!("a growing op passed"));
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::ReplicaFull);
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let max = (size * 10 / 9 / (1024 * 1024)) as u64;
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr_with("op-replica-full", &[("max", max.into())]));
}

#[test]
fn remove_data_passes_at_the_replica_cap() {
    let dir = fresh_dir("oku-slides-replica-full-");
    let mut s = session_with(settings_on(dir.path()));
    s.open(Path::new(FIXTURE)).unwrap_or_else(|e| panic!("open: {}", e.one_line()));
    let table = json!({
        "generation": "g1",
        "columns": [{ "id": "c1", "name": "Mois" }, { "id": "c2", "name": "ventes" }],
        "rows": [{ "id": "r1", "cells": ["Jan", 10] }]
    });
    let add = json!({ "op": "add_data", "id": "d9", "name": "ventes", "table": table });
    s.apply("données", vec![serde_json::from_value(add).unwrap_or_else(|e| panic!("{e}"))], None)
        .unwrap_or_else(|e| panic!("add: {}", e.one_line()));
    let size = s.doc.sources.replica().snapshot().unwrap_or_else(|e| panic!("{e}")).len();
    s.source_limits.replica_bytes = size * 10 / 9;
    // Forget the last measure, as on open: the guard measures the replica exactly.
    s.replica_gauge = super::ReplicaGauge::default();
    let remove = json!({ "op": "remove_data", "data": "d9" });
    s.apply("retrait", vec![serde_json::from_value(remove).unwrap_or_else(|e| panic!("{e}"))], None)
        .unwrap_or_else(|e| panic!("remove_data refused at the cap: {}", e.one_line()));
    assert_that!(s.doc.sources.deck().data.is_empty()).is_true();
}
