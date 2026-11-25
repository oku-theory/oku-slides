//! Data requests whose slow part runs off the tab's lock: a fold, a base's rows.

use std::sync::Mutex;

use snafu::{OptionExt, ResultExt};

use super::Workspace;
use super::deck_packs::Forward;
use crate::data_compact::folding;
use crate::data_list::SheetRead;
use crate::data_panel::PaneRead;
use crate::error::{ResponseSnafu, Result, UnknownTabSnafu};
use crate::{Request, Session};

impl Workspace {
    /// A request that may ask a fold (`Request::may_fold`): the fold runs with the tab's lock
    /// released, then the request once more ([`folding`]).
    pub(super) fn request_folding(ws: &Mutex<Self>, tab: &str, req: &Request) -> (Result<String>, bool) {
        let mut changed = false;
        let with = |g: &mut dyn FnMut(&mut Session)| {
            let (out, more) = Self::acquire(ws, tab, Forward::Yes, |s| g(s));
            changed |= more;
            out
        };
        let reply = folding(with, |s| s.handle(req.clone()));
        // `acquire` calls `f` whenever it succeeds.
        let reply = reply.and_then(|r| r.context(UnknownTabSnafu { id: tab })?);
        (reply, changed)
    }

    /// `req`, the `Request::DataPanel` of `element` on `slide`: the pane taken under the tab's
    /// lock, its rows read with none held.
    pub(super) fn data_panel(
        ws: &Mutex<Self>,
        tab: &str,
        req: &Request,
        slide: &str,
        element: &str,
    ) -> (Result<String>, bool) {
        let (pane, changed) =
            Self::acquire(ws, tab, Forward::Yes, |s| s.ensure_fits(req).map(|()| s.data_pane(slide, element)));
        let reply = pane.and_then(|pane| {
            let surface = pane?.and_then(PaneRead::surface);
            Ok(serde_json::to_string(&surface).context(ResponseSnafu)?)
        });
        (reply, changed)
    }

    /// `req`, the `Request::DataSheet` of `data` from row `offset`: the page taken under the
    /// tab's lock, its rows read with none held.
    pub(super) fn data_sheet(
        ws: &Mutex<Self>,
        tab: &str,
        req: &Request,
        data: &str,
        offset: usize,
    ) -> (Result<String>, bool) {
        let (sheet, changed) =
            Self::acquire(ws, tab, Forward::Yes, |s| s.ensure_fits(req).map(|()| s.data_sheet(data, offset)));
        let reply = sheet.and_then(|sheet| {
            let surface = sheet?.and_then(SheetRead::surface);
            Ok(serde_json::to_string(&surface).context(ResponseSnafu)?)
        });
        (reply, changed)
    }
}
