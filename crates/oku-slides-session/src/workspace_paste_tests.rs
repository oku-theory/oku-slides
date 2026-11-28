use std::sync::{Arc, Barrier, Mutex};

use oku_slides_core::{Deck, Op, PlanOutcome, plan_views};
use oku_slides_data::Data;
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::test_support::{apply_ws, fresh_dir, workspace_on};
use crate::{Localize, Localizer, Request, SessionErrorKind, Workspace, WorkspaceOutcome};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json");

/// A workspace with two tabs on roadmap.json; the first holds `d1` ("ventes") shown by `ch1`.
pub(super) struct Two {
    _data: tempfile::TempDir,
    pub(super) ws: Mutex<Workspace>,
    pub(super) a: String,
    pub(super) b: String,
}

pub(super) fn op(v: Value) -> Op {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("op : {e}"))
}

/// `rows` rows of a text column `c1` and `columns - 1` number columns `c2`…
pub(super) fn table(rows: usize, columns: usize) -> Value {
    let mut cols = vec![json!({ "id": "c1", "name": "Mois", "type": "text" })];
    cols.extend((2..=columns).map(|i| json!({ "id": format!("c{i}"), "name": format!("v{i}"), "type": "number" })));
    let rows: Vec<Value> = (0..rows)
        .map(|r| {
            let mut cells = vec![json!(format!("m{r}"))];
            cells.extend((2..=columns).map(|i| json!(r * i)));
            json!({ "id": format!("r{r}"), "cells": cells })
        })
        .collect();
    json!({ "generation": "g1", "columns": cols, "rows": rows })
}

pub(super) fn chart(id: &str, data: &str) -> Value {
    json!({ "id": id, "frame": { "x": 0, "y": 0, "w": 400, "h": 300 }, "kind": "base/chart", "params": { "option": {
        "data": data, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }]
    } } })
}

fn grid(id: &str, data: &str) -> Value {
    json!({ "id": id, "frame": { "x": 0, "y": 0, "w": 400, "h": 300 }, "kind": "base/table", "params": { "data": { "data": data, "columns": ["c2"] } } })
}

pub(super) fn add(element: Value) -> Value {
    let slot = json!({ "owner": { "slide": "s-libre" }, "param": "elements" });
    let mut op = json!({ "op": "add_element", "slot": slot });
    op["element"] = element;
    op
}

pub(super) fn two_with(deck: Option<Deck>) -> Two {
    let data = fresh_dir("oku-slides-paste-");
    let ws = workspace_on(data.path());
    let window = Workspace::lock(&ws).view().windows[0].label.clone();
    apply_ws(&ws, json!({ "cmd": "new_tab", "window": window })).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let tabs: Vec<String> = Workspace::lock(&ws).view().windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    let (a, b) = (tabs[0].clone(), tabs[1].clone());
    let path = match deck {
        Some(deck) => {
            let path = data.path().join("source.json");
            std::fs::write(&path, deck.to_json().unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
            path
        }
        None => FIXTURE.into(),
    };
    for (tab, file) in [(&a, path.as_path()), (&b, std::path::Path::new(FIXTURE))] {
        Workspace::with_session(&ws, tab, |s| s.open(file))
            .0
            .and_then(|r| r)
            .unwrap_or_else(|e| panic!("{}", e.one_line()));
    }
    Two { _data: data, ws, a, b }
}

/// Two tabs; `a` holds `d1` ("ventes", 2 rows) shown by the chart `ch1` and the table `tb1`.
fn two() -> Two {
    let t = two_with(None);
    let ops = vec![
        op(json!({ "op": "add_data", "id": "d1", "name": "ventes", "table": table(2, 2) })),
        op(add(chart("ch1", "d1"))),
        op(add(grid("tb1", "d1"))),
    ];
    t.on(&t.a.clone(), |s| s.apply("données", ops, None)).unwrap_or_else(|e| panic!("{}", e.one_line()));
    t
}

/// Two tabs; `a` holds `d1` ("ventes") of `rows` rows of 10 columns, shown by the table `tb1`.
fn two_large(rows: usize) -> Two {
    let mut deck = Deck::from_json(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let ops = vec![
        op(json!({ "op": "add_data", "id": "d1", "name": "ventes", "table": table(rows, 10) })),
        op(add(grid("tb1", "d1"))),
    ];
    oku_slides_core::apply_all(&mut deck, ops, &crate::test_support::registry()).unwrap_or_else(|e| panic!("{e:?}"));
    two_with(Some(deck))
}

impl Two {
    pub(super) fn on<R>(&self, tab: &str, f: impl FnOnce(&mut crate::Session) -> R) -> R {
        Workspace::with_session(&self.ws, tab, f).0.unwrap_or_else(|e| panic!("{}", e.one_line()))
    }

    pub(super) fn paste(&self, from: &str, to: &str, ops: &[Value]) -> crate::Result<WorkspaceOutcome> {
        apply_ws(&self.ws, json!({ "cmd": "paste_element", "from": from, "to": to, "label": "Coller", "ops": ops }))
    }

    pub(super) fn deck(&self, tab: &str) -> Deck {
        self.on(tab, |s| s.deck().clone())
    }

    /// The names of the data of `tab`, sorted.
    pub(super) fn names(&self, tab: &str) -> Vec<String> {
        let mut names: Vec<String> = self.deck(tab).data.values().map(|d| d.name.clone()).collect();
        names.sort();
        names
    }

    /// The data `tab` names `name`, with its id.
    pub(super) fn named(&self, tab: &str, name: &str) -> (String, Arc<Data>) {
        let deck = self.deck(tab);
        let found = deck.data.iter().find(|(_, d)| d.name == name).map(|(id, d)| (id.as_str().to_owned(), d.clone()));
        found.unwrap_or_else(|| panic!("{name} absente de {tab}"))
    }

    /// The data the view `param` of `element` (on `s-libre`) names.
    pub(super) fn view_ref(&self, tab: &str, element: &str, param: &str) -> String {
        let deck = self.deck(tab);
        let slide = deck.slides.iter().find(|s| s.id == "s-libre").unwrap_or_else(|| panic!("s-libre"));
        let e = slide.locate(element).map_or_else(|| panic!("{element} absent"), |l| l.element.clone());
        let v = serde_json::to_value(&e.params).unwrap_or_else(|e| panic!("{e}"));
        v[param]["data"].as_str().unwrap_or_else(|| panic!("{element} sans données")).to_owned()
    }
}

pub(super) fn fr_text(key: &str, args: &[(&str, oku_slides_i18n::FluentValue<'static>)]) -> String {
    Localizer::new(Some("fr")).tr_with(key, args)
}

pub(super) fn fr(e: &crate::SessionError) -> String {
    e.localize(&Localizer::new(Some("fr")))
}

#[test]
fn paste_in_same_deck_copies_data_as_ventes_2() {
    let t = two();
    t.paste(&t.a, &t.a, &[add(chart("ch2", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(t.names(&t.a)).is_equal_to(vec!["ventes".to_owned(), "ventes_2".to_owned()]);
    let (copy, _) = t.named(&t.a, "ventes_2");
    assert_that!(t.view_ref(&t.a, "ch2", "option")).is_equal_to(copy);
    assert_that!(t.view_ref(&t.a, "ch1", "option")).is_equal_to("d1".to_owned());
}

#[test]
fn paste_twice_gives_ventes_2_then_ventes_3() {
    let t = two();
    t.paste(&t.a, &t.a, &[add(chart("ch2", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    t.paste(&t.a, &t.a, &[add(chart("ch3", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(t.names(&t.a)).is_equal_to(vec!["ventes".to_owned(), "ventes_2".to_owned(), "ventes_3".to_owned()]);
    assert_that!(t.view_ref(&t.a, "ch3", "option")).is_equal_to(t.named(&t.a, "ventes_3").0);
}

#[test]
fn paste_into_other_deck_carries_data_with_types_and_ids() {
    let t = two();
    t.paste(&t.a, &t.b, &[add(chart("ch1", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let (_, source) = t.named(&t.a, "ventes");
    let (copy, carried) = t.named(&t.b, "ventes");
    assert_that!(t.names(&t.b)).is_equal_to(vec!["ventes".to_owned()]);
    assert_that!(t.view_ref(&t.b, "ch1", "option")).is_equal_to(&copy);
    assert_that!(carried.table().unwrap().columns).is_equal_to(&source.table().unwrap().columns);
    assert_that!(carried.table().unwrap().rows).is_equal_to(&source.table().unwrap().rows);
    assert_that!(carried.table().unwrap().cells).is_equal_to(&source.table().unwrap().cells);
}

#[test]
fn paste_keeps_projection_valid() {
    let t = two();
    t.paste(&t.a, &t.b, &[add(grid("tb2", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let plans = t.on(&t.b, |s| plan_views(s.deck(), &s.registry).0);
    let plan = plans.iter().find(|p| p.view.element == "tb2").unwrap_or_else(|| panic!("tb2 sans plan"));
    assert_that!(plan.columns).is_equal_to(vec!["c2".to_owned()]);
    assert_that!(plan.outcome).is_equal_to(PlanOutcome::Resolve { cut: false });
}

#[test]
fn paste_of_two_views_on_one_data_makes_one_copy() {
    let t = two();
    t.paste(&t.a, &t.b, &[add(chart("ch1", "d1")), add(grid("tb1", "d1"))])
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(t.names(&t.b)).is_equal_to(vec!["ventes".to_owned()]);
    let (copy, _) = t.named(&t.b, "ventes");
    assert_that!(t.view_ref(&t.b, "ch1", "option")).is_equal_to(&copy);
    assert_that!(t.view_ref(&t.b, "tb1", "data")).is_equal_to(&copy);
}

#[test]
fn paste_then_undo_removes_element_and_data_in_one_step() {
    let t = two();
    let before = t.deck(&t.b);
    t.paste(&t.a, &t.b, &[add(chart("ch1", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(t.on(&t.b, |s| crate::test_support::undo_label(s))).is_equal_to(Some("Coller".to_owned()));
    Workspace::request(&t.ws, &t.b, Request::Undo).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(t.deck(&t.b)).is_equal_to(&before);
}

#[test]
fn paste_over_deck_bounds_is_refused_and_adds_nothing() {
    // 160,000 cells, twice: over the deck's 300,000 overlay entries.
    let t = two_large(16_000);
    let before = t.deck(&t.a);
    let e = t.paste(&t.a, &t.a, &[add(grid("tb2", "d1"))]).map(|_| ()).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::Op);
    let full = fr_text("op-data-deck-full", &[("what", "overlay".into()), ("max", 300_000.into())]);
    assert_that!(fr(&e)).contains(full.as_str());
    assert_that!(t.deck(&t.a)).is_equal_to(&before);
}

#[test]
fn paste_of_data_over_100000_cells_is_accepted() {
    let t = two_large(11_000);
    t.paste(&t.a, &t.b, &[add(grid("tb1", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let (copy, carried) = t.named(&t.b, "ventes");
    assert_that!(carried.table().unwrap().cells.len()).is_equal_to(110_000);
    assert_that!(t.view_ref(&t.b, "tb1", "data")).is_equal_to(copy);
}

#[test]
fn paste_with_put_view_data_in_ui_ops_is_refused() {
    let t = two();
    let put = json!({ "op": "put_view_data", "target": { "slide": "s-libre", "element": "ch1" }, "param": "option",
        "id": "d9", "name": "autre", "label": null, "table": table(1, 2) });
    let ops = vec![put, add(chart("ch2", "d1"))];
    let before = t.deck(&t.a);
    let e = t.paste(&t.a, &t.a, &ops).map(|_| ()).unwrap_err();
    assert_that!(fr(&e)).is_equal_to(fr_text("op-data-internal", &[]));
    assert_that!(t.deck(&t.a)).is_equal_to(&before);
}

#[test]
fn paste_with_base_in_ui_ops_is_refused() {
    let t = two();
    let mut table = table(0, 2);
    table["base"] = json!({ "id": format!("sha256:{}", "a".repeat(64)), "bytes": 1, "rows": 0, "columns": [] });
    let ops = [json!({ "op": "add_data", "name": "autre", "table": table }), add(chart("ch2", "d1"))];
    let before = t.deck(&t.a);
    let e = t.paste(&t.a, &t.a, &ops).map(|_| ()).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::Op);
    assert_that!(fr(&e)).is_equal_to(fr_text("op-data-internal", &[]));
    assert_that!(t.deck(&t.a)).is_equal_to(&before);
}

#[test]
fn paste_with_closed_source_is_refused_op_data_missing() {
    let t = two();
    apply_ws(&t.ws, json!({ "cmd": "close", "tab": t.a, "discard": true }))
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    let before = t.deck(&t.b);
    let e = t.paste(&t.a, &t.b, &[add(chart("ch1", "d1"))]).map(|_| ()).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::Op);
    assert_that!(fr(&e)).is_equal_to(fr_text("op-data-missing", &[("data", "d1".into())]));
    assert_that!(t.deck(&t.b)).is_equal_to(&before);
}

#[test]
fn paste_after_source_data_removed_is_refused() {
    let t = two();
    // Undoing the step that added `d1` and its views removes it from the source deck.
    Workspace::request(&t.ws, &t.a, Request::Undo).0.unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(t.deck(&t.a).data.contains_key("d1")).is_false();
    let before = t.deck(&t.b);
    let e = t.paste(&t.a, &t.b, &[add(chart("ch1", "d1"))]).map(|_| ()).unwrap_err();
    assert_that!(fr(&e)).is_equal_to(fr_text("op-data-missing", &[("data", "d1".into())]));
    assert_that!(t.deck(&t.b)).is_equal_to(&before);
}

#[test]
fn two_pastes_in_flight_each_copy_their_own_data() {
    let t = two();
    let (read, done) = (Barrier::new(2), Barrier::new(2));
    let first = [add(chart("ch2", "d1"))].map(op).to_vec();
    std::thread::scope(|scope| {
        let held = scope.spawn(|| {
            let source = Workspace::paste_source(&t.ws, &t.a, &first);
            read.wait();
            done.wait();
            Workspace::paste_into(&t.ws, &t.a, "Coller", first.clone(), source)
        });
        read.wait();
        t.paste(&t.a, &t.a, &[add(chart("ch3", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
        done.wait();
        held.join().unwrap_or_else(|_| panic!("collage")).unwrap_or_else(|e| panic!("{}", e.one_line()));
    });
    assert_that!(t.names(&t.a)).is_equal_to(vec!["ventes".to_owned(), "ventes_2".to_owned(), "ventes_3".to_owned()]);
    let (two, three) = (t.view_ref(&t.a, "ch2", "option"), t.view_ref(&t.a, "ch3", "option"));
    assert_that!(two).is_not_equal_to(&three);
    assert_that!([two, three].iter().all(|id| id != "d1")).is_true();
}

#[test]
fn cut_paste_in_same_tab_keeps_data_ref() {
    let t = two();
    let ops = vec![op(json!({ "op": "move_element", "slide": "s-libre", "element": "ch1",
        "to": { "owner": { "slide": "s-constat" }, "param": "blocks" } }))];
    t.on(&t.a.clone(), |s| s.apply("couper", ops, None)).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let deck = t.deck(&t.a);
    let slide = deck.slides.iter().find(|s| s.id == "s-constat").unwrap_or_else(|| panic!("s-constat"));
    let moved = slide.locate("ch1").map(|l| serde_json::to_value(&l.element.params).unwrap());
    assert_that!(moved.map(|v| v["option"]["data"].clone())).is_equal_to(Some(json!("d1")));
    assert_that!(t.names(&t.a)).is_equal_to(vec!["ventes".to_owned()]);
}

#[test]
fn add_element_with_existing_ref_shares_data() {
    let t = two();
    t.on(&t.a.clone(), |s| s.apply("ajout", vec![op(add(chart("ch2", "d1")))], None))
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    assert_that!(t.view_ref(&t.a, "ch2", "option")).is_equal_to("d1".to_owned());
    assert_that!(t.names(&t.a)).is_equal_to(vec!["ventes".to_owned()]);
}

/// → a refused paste rolls back the bases it made known: the target neither knows nor leases
/// them (t-1690).
#[test]
fn paste_refused_forgets_the_bases_it_brought() {
    let t = two();
    let id = t.on(&t.a.clone(), |s| crate::bases::bases_tests::compact_data_onto(s, "d1", b"base des ventes", "e1"));
    let mut misplaced = add(chart("ch1", "d1"));
    misplaced["slot"]["owner"]["slide"] = json!("nowhere");
    assert_that!(t.paste(&t.a, &t.b, &[misplaced]).map(|_| ())).is_err();
    apply_ws(&t.ws, json!({ "cmd": "close", "tab": t.a, "discard": true }))
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    let (store, known) = t.on(&t.b.clone(), |s| (s.bases.store().clone(), s.doc.sources.bases.clone()));
    assert_that!(known.contains(&id)).is_false();
    // Past the grace period: nothing keeps the file.
    let hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let file = std::fs::File::options().write(true).open(store.path(&id)).unwrap_or_else(|e| panic!("{e}"));
    file.set_modified(hour_ago).unwrap_or_else(|e| panic!("{e}"));
    store.gc(&std::collections::BTreeSet::new()).unwrap_or_else(|e| panic!("gc: {e}"));
    assert_that!(store.path(&id).exists()).is_false();
}

#[test]
fn paste_leases_its_bases() {
    let t = two();
    let id = t.on(&t.a.clone(), |s| crate::bases::bases_tests::compact_data_onto(s, "d1", b"base des ventes", "e1"));
    t.paste(&t.a, &t.b, &[add(chart("ch1", "d1"))]).unwrap_or_else(|e| panic!("{}", e.one_line()));
    apply_ws(&t.ws, json!({ "cmd": "close", "tab": t.a, "discard": true }))
        .unwrap_or_else(|e| panic!("{}", e.one_line()));
    let (store, known) = t.on(&t.b.clone(), |s| (s.bases.store().clone(), s.doc.sources.bases.clone()));
    assert_that!(known.contains(&id)).is_true();
    // Past the grace period: only the lease of `b` keeps the file.
    let hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let file = std::fs::File::options().write(true).open(store.path(&id)).unwrap_or_else(|e| panic!("{e}"));
    file.set_modified(hour_ago).unwrap_or_else(|e| panic!("{e}"));
    store.gc(&std::collections::BTreeSet::new()).unwrap_or_else(|e| panic!("gc: {e}"));
    assert_that!(store.path(&id).exists()).is_true();
}
