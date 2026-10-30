use speculoos::prelude::*;

use super::super::packs::workspace_packs_fixtures::{packs, packs_resumed};
use crate::agent::Sessions;
use crate::{DocumentKind, SessionErrorKind, Workspace};

#[test]
fn document_kind_of_a_dormant_pack_tab_is_pack_and_keeps_it_dormant() {
    let f = packs_resumed();
    let tab = f.pack_tab();
    let f = f.restart(|_, _| {}); // tabs come back dormant
    assert_that!(Workspace::document_kind(&f.ws, &tab)).is_ok().is_equal_to(DocumentKind::Pack);
    assert_that!(f.is_dormant(&tab)).is_true();
}

#[test]
fn document_kind_of_an_unknown_tab_is_refused() {
    let f = packs();
    let err = Workspace::document_kind(&f.ws, "t-99").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownTab);
}

#[test]
fn document_kind_after_settle_is_refused() {
    let f = packs();
    Workspace::settle(&f.ws);
    let err = Workspace::document_kind(&f.ws, "t-1").unwrap_err();
    assert_that!(err.kind()).is_equal_to(SessionErrorKind::UnknownWindow);
}

#[test]
fn document_kind_through_the_sessions_trait_reads_the_workspace() {
    let f = packs();
    let tab = f.pack_tab();
    let sessions: &dyn Sessions = &*f.ws;
    assert_that!(sessions.document_kind(&tab)).is_ok().is_equal_to(DocumentKind::Pack);
}
