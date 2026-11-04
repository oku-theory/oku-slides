//! What each tab's page last reported: its context and whether it presents.
//!
//! A page reports with a page id (one per mounted page) and a number. Reports are ordered by
//! `(page, seq)` per kind: an older or equal number is ignored, and a new page id starts over,
//! its first report accepted. Only the window that holds the tab may report
//! ([`ReportFrom::Holder`], checked by the host); one for a tab no window holds any more is
//! ignored ([`ReportFrom::Unheld`]). The host forgets a tab's reports when its page
//! id changes, the tab closes or its window is destroyed. Reports never reach the tab's agent:
//! any command during a turn would cancel it. Whether a tab presents is ordered here only; the
//! stack copies it to the tab's hub ([`PageReports::presenting_now`]) for the `present` tool.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use oku_slides_mcp::PresentMode;
use oku_slides_session::Workspace;
use oku_slides_session::agent::AgentUi;

use crate::AgentCommand;
use crate::command_error::{CommandError, ReportElsewhereSnafu, SeqExhaustedSnafu, UiTooLargeSnafu};

/// The longest page id a report may carry, in bytes.
pub(crate) const PAGE_ID_MAX: usize = 64;

/// Who sent a page report, as the host checked it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportFrom {
    /// The window that holds the report's tab, by label.
    Holder {
        /// The window's label (`w-<n>`; the dev server's page names its own).
        window: String,
    },
    /// The report's tab is held by no window any more (closed while it flew): ignored.
    Unheld,
    /// Any other sender: its reports are refused.
    Other,
}

impl ReportFrom {
    /// Who sends `cmd` from window `window`: its tab's holder, nobody's tab, else another sender.
    ///
    /// The one holder check of the app (the window's label) and the dev server (the label the
    /// page names). Only a page report locks the workspace; any other command is `Other`.
    pub fn of(workspace: &Mutex<Workspace>, cmd: &AgentCommand, window: Option<&str>) -> Self {
        let Some(window) = window.filter(|_| cmd.is_page_report()) else { return Self::Other };
        match Workspace::lock(workspace).tab_window(cmd.tab()) {
            Some(holder) if holder == window => Self::Holder { window: window.to_owned() },
            Some(_) => Self::Other,
            None => Self::Unheld,
        }
    }
}

/// A tab's last reported context.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PageContext {
    /// What the page shows: slide, selection, field.
    pub ui: AgentUi,
    /// The page said it has the focus (`document.hasFocus()`).
    pub has_focus: bool,
}

/// One page's reports for a tab, each with the number it came with.
#[derive(Debug)]
struct TabReports {
    window: String,
    page: String,
    context: Option<(u64, PageContext)>,
    presenting: Option<(u64, Option<PresentMode>)>,
}

/// Every tab's last page reports, and the number of the last presenting read.
#[derive(Debug, Default)]
struct Reports {
    tabs: HashMap<String, TabReports>,
    /// Numbers [`PageReports::presenting_now`]: a later read has a higher number.
    reads: u64,
}

/// Every tab's last page reports.
#[derive(Debug, Default)]
pub(crate) struct PageReports {
    tabs: Mutex<Reports>,
}

/// A report's sender, page id and number checked; the window that sent it, `None` to ignore it.
fn checked<'a>(from: &'a ReportFrom, page: &str, seq: u64) -> Result<Option<&'a str>, CommandError> {
    // A label no window can have (`w-<n>`) holds no tab: refused like another window's.
    let window = match from {
        ReportFrom::Holder { window } if window.len() <= PAGE_ID_MAX => window,
        ReportFrom::Unheld => return Ok(None),
        _ => return Err(ReportElsewhereSnafu.build().into()),
    };
    if seq == u64::MAX {
        return Err(SeqExhaustedSnafu { seq }.build().into());
    }
    if page.is_empty() || page.len() > PAGE_ID_MAX {
        return Err(UiTooLargeSnafu.build().into());
    }
    Ok(Some(window))
}

impl PageReports {
    /// The reports of `tab` from `window`'s page `page`: a new page or window starts over.
    fn entry<'a>(tabs: &'a mut HashMap<String, TabReports>, tab: &str, window: &str, page: &str) -> &'a mut TabReports {
        let fresh = || TabReports { window: window.to_owned(), page: page.to_owned(), context: None, presenting: None };
        let reports = tabs.entry(tab.to_owned()).or_insert_with(fresh);
        if reports.page != page || reports.window != window {
            *reports = fresh();
        }
        reports
    }

    /// Keeps `context` for `tab` if newer than its page's last one; true when kept.
    ///
    /// A report for a tab no window holds any more ([`ReportFrom::Unheld`]) is ignored.
    ///
    /// # Errors
    ///
    /// Refused from another window (`ReportElsewhere`), with the last number (`SeqExhausted`),
    /// or over the size limits (`UiTooLarge`).
    pub(crate) fn context(
        &self,
        tab: &str,
        from: &ReportFrom,
        page: &str,
        seq: u64,
        context: PageContext,
    ) -> Result<bool, CommandError> {
        let Some(window) = checked(from, page, seq)? else { return Ok(false) };
        if !context.ui.within_limits() {
            return Err(UiTooLargeSnafu.build().into());
        }
        let mut tabs = self.lock();
        let reports = Self::entry(&mut tabs.tabs, tab, window, page);
        if reports.context.as_ref().is_some_and(|(last, _)| seq <= *last) {
            return Ok(false);
        }
        reports.context = Some((seq, context));
        Ok(true)
    }

    /// Keeps whether `tab` presents if newer than its page's last report; true when kept.
    ///
    /// # Errors
    ///
    /// As [`Self::context`], the size limits bearing on the page id only.
    pub(crate) fn presenting(
        &self,
        tab: &str,
        from: &ReportFrom,
        page: &str,
        seq: u64,
        mode: Option<PresentMode>,
    ) -> Result<bool, CommandError> {
        let Some(window) = checked(from, page, seq)? else { return Ok(false) };
        let mut tabs = self.lock();
        let reports = Self::entry(&mut tabs.tabs, tab, window, page);
        if reports.presenting.as_ref().is_some_and(|(last, _)| seq <= *last) {
            return Ok(false);
        }
        reports.presenting = Some((seq, mode));
        Ok(true)
    }

    /// The last context `tab`'s page reported.
    pub(crate) fn last_context(&self, tab: &str) -> Option<PageContext> {
        self.lock().tabs.get(tab).and_then(|r| r.context.as_ref()).map(|(_, c)| c.clone())
    }

    /// Where `tab`'s page presents now (`None`: it does not), numbered for the tab's hub.
    ///
    /// Read under the lock that orders the reports: a later read has a higher number and sees
    /// a state at least as new, so the hub keeping the highest number keeps the latest state.
    pub(crate) fn presenting_now(&self, tab: &str) -> (Option<PresentMode>, u64) {
        let mut reports = self.lock();
        // One read per report at most: 2^64 of them never happen.
        reports.reads = reports.reads.saturating_add(1);
        let mode = reports.tabs.get(tab).and_then(|r| r.presenting.as_ref()).and_then(|(_, mode)| *mode);
        (mode, reports.reads)
    }

    /// `tab` closed: its reports go.
    pub(crate) fn forget_tab(&self, tab: &str) {
        self.lock().tabs.remove(tab);
    }

    /// Window `window` is destroyed: the reports its pages sent go; returns their tabs.
    pub(crate) fn forget_window(&self, window: &str) -> Vec<String> {
        let mut reports = self.lock();
        let gone: Vec<String> =
            reports.tabs.iter().filter(|(_, r)| r.window == window).map(|(tab, _)| tab.clone()).collect();
        for tab in &gone {
            reports.tabs.remove(tab);
        }
        gone
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Reports> {
        self.tabs.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "page_reports_tests.rs"]
mod page_reports_tests;
