use std::sync::{Arc, Mutex};

use speculoos::prelude::*;

use super::{PackGate, PackOpen, Sessions, TabSession};
use crate::test_support::{fresh_dir, session, session_on};

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
        _gate: &mut dyn FnMut(PackGate) -> bool,
    ) -> crate::Result<PackOpen> {
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
fn single_document_host_opens_a_pack_in_place() {
    let data = fresh_dir("oku-slides-tabs-");
    let host = Mutex::new(session_on(data.path()));
    let opened = host.open_pack("t-1", crate::BASE_PACK, None, &mut |_| panic!("no gate in a single-document host"));
    let editing = host.lock().unwrap().editing_view().map(|e| e.pack);
    assert_that!((opened.map_err(|e| e.kind()), editing)).is_equal_to((Ok(PackOpen::Here), Some(crate::BASE_PACK)));
}
