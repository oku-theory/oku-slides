//! The stack's side of page reports: kept, refused with a localized message, or forgotten.

use oku_slides_i18n::Localize;
use oku_slides_mcp::{PresentMode, SurfaceHub};

use crate::Stack;
use crate::command_error::CommandError;
use crate::page_reports::{PageContext, ReportFrom};

impl Stack {
    /// Keeps a page's context report; a kept one also tells the workspace its focus.
    pub(crate) fn context_report(
        &self,
        tab: &str,
        from: &ReportFrom,
        page: &str,
        seq: u64,
        c: PageContext,
    ) -> Result<(), String> {
        let has_focus = c.has_focus;
        let kept = self.reports.context(tab, from, page, seq, c);
        if self.report_kept(tab, kept)? {
            self.workspace.page_focus(tab, has_focus);
            // A new page id forgets that the page presented.
            if let Some(t) = self.tab(tab) {
                self.copy_presenting(&t.hub);
            }
        }
        Ok(())
    }

    /// Keeps a page's presenting report.
    pub(crate) fn presenting_report(
        &self,
        tab: &str,
        from: &ReportFrom,
        page: &str,
        seq: u64,
        mode: Option<PresentMode>,
    ) -> Result<(), String> {
        let kept = self.reports.presenting(tab, from, page, seq, mode);
        // The tab starts (tools, keeper; not the ACP agent): its `present` tool reads the hub.
        if self.report_kept(tab, kept)?
            && let Some(t) = self.open(tab)
        {
            self.copy_presenting(&t.hub);
        }
        Ok(())
    }

    /// Copies whether the hub's tab presents from the reports, numbered so the latest wins.
    pub(crate) fn copy_presenting(&self, hub: &SurfaceHub) {
        let (mode, seq) = self.reports.presenting_now(hub.tab());
        hub.set_presenting(mode, seq);
    }

    /// Whether a page report was kept; its refusal localized, logged without data.
    fn report_kept(&self, tab: &str, kept: Result<bool, CommandError>) -> Result<bool, String> {
        kept.map_err(|e| {
            tracing::info!(tab, error = %snafu::Report::from_error(&e), "page report refused");
            e.localize(&self.platform.live_localizer.current())
        })
    }

    /// Window `window` is destroyed: the reports of its pages go, and their tabs stop presenting.
    pub fn window_destroyed(&self, window: &str) {
        for tab in self.reports.forget_window(window) {
            if let Some(t) = self.tab(&tab) {
                self.copy_presenting(&t.hub);
            }
        }
    }
}
