use std::sync::{Arc, Mutex};

use rstest::rstest;
use speculoos::prelude::*;

use super::{Sessions, TabGate, TabOpen, TabSession};
use crate::test_support::{fresh_dir, session, session_on};
use crate::test_support_packs::pack_tab_of;

#[derive(Debug)]
struct Closed;

impl Sessions for Closed {
    fn with(&self, tab: &str, _f: &mut dyn FnMut(&mut crate::Session)) -> crate::Result<()> {
        Err(crate::error::UnknownTabSnafu { id: tab }.build().into())
    }

    fn open_pack(
        &self,
        tab: &str,
        _pack: uuid::Uuid,
        _widget: Option<&str>,
        _gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> crate::Result<TabOpen> {
        Err(crate::error::UnknownTabSnafu { id: tab }.build().into())
    }

    fn open_theme(
        &self,
        tab: &str,
        _theme: uuid::Uuid,
        _gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> crate::Result<TabOpen> {
        Err(crate::error::UnknownTabSnafu { id: tab }.build().into())
    }

    fn document_kind(&self, tab: &str) -> crate::Result<crate::DocumentKind> {
        Err(crate::error::UnknownTabSnafu { id: tab }.build().into())
    }
}

#[test]
fn tab_session_with_runs_on_the_session_and_returns_its_value() {
    let tab = TabSession::new("t-1", Arc::new(Mutex::new(session())));
    let slides = tab.with(|s| s.deck().slides.len());
    assert_that!(slides).is_ok().is_greater_than(0);
    assert_that!(tab.tab()).is_equal_to("t-1");
}

#[test]
fn tab_session_with_unknown_tab_returns_the_error_without_calling_f() {
    let tab = TabSession::new("t-9", Arc::new(Closed));
    let mut called = false;
    let r = tab.with(|_| called = true);
    assert_that!(r).is_err();
    assert_that!(called).is_false();
}

#[test]
fn open_elsewhere_defaults_to_false_for_a_single_session_host() {
    let tab = TabSession::new("t-1", Arc::new(Mutex::new(session())));
    assert_that!(tab.open_elsewhere(std::path::Path::new("/tmp/a.oku-slides.pdf"))).is_false();
}

#[test]
fn open_tab_defaults_to_single_document_for_a_single_session_host() {
    let tab = TabSession::new("t-1", Arc::new(Mutex::new(session())));
    let r = tab.open_tab(std::path::Path::new("/tmp/a.oku-slides.pdf")).map_err(|e| e.kind());
    assert_that!(r).is_err().is_equal_to(crate::SessionErrorKind::SingleDocument);
}

#[test]
fn single_document_host_opens_its_own_pack_widget_here() {
    let data = fresh_dir("oku-slides-tabs-");
    let host = Mutex::new(pack_tab_of(data.path(), crate::BASE_PACK));
    let opened =
        host.open_pack("t-1", crate::BASE_PACK, Some("card"), &mut |_| panic!("no gate in a single-document host"));
    let s = host.lock().unwrap();
    let here = (s.editing_view().map(|e| e.pack), s.ui.widget.clone());
    assert_that!((opened.map_err(|e| e.kind()), here))
        .is_equal_to((Ok(TabOpen::Here), (Some(crate::BASE_PACK), Some("card".to_owned()))));
}

/// A deck session, or a pack tab of another pack, opens no pack tab of its own.
#[rstest]
#[case::deck(None)]
#[case::other_pack_tab(Some(crate::ESSENTIEL_PACK))]
fn single_document_host_refuses_another_pack(#[case] edited: Option<uuid::Uuid>) {
    let data = fresh_dir("oku-slides-tabs-");
    let s = edited.map_or_else(|| session_on(data.path()), |pack| pack_tab_of(data.path(), pack));
    let host = Mutex::new(s);
    let opened = host.open_pack("t-1", crate::BASE_PACK, None, &mut |_| panic!("no gate in a single-document host"));
    let editing = host.lock().unwrap().editing_view().map(|e| e.pack);
    assert_that!((opened.map_err(|e| e.kind()), editing))
        .is_equal_to((Err(crate::SessionErrorKind::SingleDocument), edited));
}

#[test]
fn single_document_host_refuses_to_open_a_theme() {
    let host = Mutex::new(session());
    let opened =
        host.open_theme("t-1", crate::theme::PAPIER_THEME, &mut |_| panic!("no gate in a single-document host"));
    assert_that!(opened.map_err(|e| e.kind())).is_equal_to(Err(crate::SessionErrorKind::SingleDocument));
}

#[test]
fn single_document_host_opens_its_own_theme_here() {
    let data = fresh_dir("oku-slides-tabs-");
    let host = Mutex::new(crate::theme_tab::theme_tab_tests::theme_tab(data.path(), crate::theme::PAPIER_THEME));
    let opened =
        host.open_theme("t-1", crate::theme::PAPIER_THEME, &mut |_| panic!("no gate in a single-document host"));
    assert_that!(opened.map_err(|e| e.kind())).is_equal_to(Ok(TabOpen::Here));
}
