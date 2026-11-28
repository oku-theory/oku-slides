//! Query data in the one-shot CLI: `check`, `save` and `render` compute the results their file
//! does not cache, and report a failed query (R25).
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod common;

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Output;

use common::{assert_success, oku_slides, workdir};
use speculoos::prelude::*;

fn run(cmd: &mut std::process::Command) -> Output {
    cmd.output().unwrap_or_else(|e| panic!("oku-slides does not start: {e}"))
}

/// Without the Unicode isolation marks Fluent puts around arguments.
fn plain(text: &[u8]) -> String {
    String::from_utf8_lossy(text).replace(['\u{2068}', '\u{2069}'], "")
}

/// A JSON deck in `dir` with the manual data `d` (`ventes`), the query data `q` (`top`, `sql`) and
/// a chart showing `q`.
fn deck_with_query(dir: &Path, sql: &str) -> PathBuf {
    let chart = serde_json::json!({ "id": "c", "kind": oku_slides_core::kinds::base::CHART, "params": { "option": {
        "data": "q", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } });
    let slides = serde_json::json!([{ "id": "s", "kind": oku_slides_core::kinds::essentiel::CONTENT,
        "params": { "title": "T", "blocks": [chart] } }]);
    let json = serde_json::json!({ "version": oku_slides_core::DECK_VERSION, "slides": slides, "data": {
        "d": { "name": "ventes", "kind": "manual", "table": {
            "generation": "g", "columns": [{ "id": "m", "name": "a", "type": "text" },
                { "id": "v", "name": "b", "type": "number" }],
            "rows": ["r1", "r2"], "cells": { "r1/m": "Jan", "r1/v": 12, "r2/m": "Fév", "r2/v": -3 } } },
        "q": { "name": "top", "kind": "query", "sql": sql } } });
    let deck = dir.join("deck.json");
    std::fs::write(&deck, serde_json::to_vec(&json).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    deck
}

/// The names of the cache entries in the sources of `pdf`.
fn cached(pdf: &Path) -> Vec<String> {
    let bytes = std::fs::read(pdf).unwrap_or_else(|e| panic!("{e}"));
    let zip = oku_slides_pdf::extract_sources(&bytes)
        .unwrap_or_else(|e| panic!("{e}"))
        .unwrap_or_else(|| panic!("no sources"))
        .zip;
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap_or_else(|e| panic!("{e}"));
    za.file_names()
        .map(|n| n.unwrap_or_else(|e| panic!("{e}")).into_owned())
        .filter(|n| n.starts_with("data/"))
        .collect()
}

/// The first page `render` draws of `pdf` with the data folder `data`.
fn first_page(dir: &Path, data: &Path, pdf: &Path) -> String {
    let out_dir = dir.join("out");
    let out = run(oku_slides(dir, "en")
        .env("OKU_SLIDES_PATHS__DATA", data)
        .arg("render")
        .arg(pdf)
        .arg("--out")
        .arg(&out_dir));
    assert_success("render", &out, &[]);
    let mut page = String::new();
    std::fs::File::open(out_dir.join("page-01.svg"))
        .and_then(|mut f| f.read_to_string(&mut page))
        .unwrap_or_else(|e| panic!("page-01.svg: {e}"));
    page
}

/// → `save` computes the query and caches its result; `render` on a fresh data folder draws the
/// chart over it: an image, not the pending placeholder.
#[test]
fn save_caches_query_result_and_render_draws_its_view() {
    let dir = workdir("query-render");
    let deck = deck_with_query(&dir, "SELECT a, b FROM ventes WHERE b > 0");
    let pdf = dir.join("deck.oku-slides.pdf");
    assert_success("save", &run(oku_slides(&dir, "en").arg("save").arg(&deck).arg(&pdf)), &[]);
    assert_that!(cached(&pdf)).has_length(1);

    let page = first_page(&dir, &dir.join("fresh-data"), &pdf);

    // A drawn chart is an embedded image; a pending or failure frame is not.
    assert_that!(page).contains("<image");
}

/// → `check` names the failed query with its localized issue and does not call the deck valid.
#[test]
fn check_reports_failed_query_with_its_localized_issue() {
    let dir = workdir("query-check");
    let deck = deck_with_query(&dir, "SELECT zzz FROM ventes");

    let out = run(oku_slides(&dir, "fr").arg("check").arg(&deck));

    assert_that!(out.status.success()).is_true();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    // The reason is the engine's; the issue's text up to it names the query.
    let issue = fr.tr_with("issue-data-query-failed", &[("data", "top".into()), ("reason", "REASON".into())]);
    let head = plain(issue.as_bytes()).split("REASON").next().unwrap_or_default().to_owned();
    assert_that!(plain(&out.stderr)).contains(head.as_str());
    assert_that!(plain(&out.stdout)).is_empty();
}
