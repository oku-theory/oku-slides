use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::PresentMode;
use oku_slides_session::agent::{AgentUi, UI_TEXT_MAX};
use rstest::rstest;
use speculoos::prelude::*;

use super::{PAGE_ID_MAX, PageContext, PageReports, ReportFrom};

fn holder() -> ReportFrom {
    ReportFrom::Holder { window: "w-1".into() }
}

fn on_slide(slide: &str) -> PageContext {
    PageContext { ui: AgentUi { slide: Some(slide.into()), ..AgentUi::default() }, has_focus: true }
}

fn slide_of(r: &PageReports) -> Option<String> {
    r.last_context("t-1").and_then(|c| c.ui.slide)
}

#[test]
fn context_older_seq_is_ignored() {
    let r = PageReports::default();
    assert_that!(r.context("t-1", &holder(), "p-1", 5, on_slide("s-5")).ok()).is_equal_to(Some(true));
    assert_that!(r.context("t-1", &holder(), "p-1", 4, on_slide("s-4")).ok()).is_equal_to(Some(false));
    assert_that!(r.context("t-1", &holder(), "p-1", 5, on_slide("s-again")).ok()).is_equal_to(Some(false));
    assert_that!(slide_of(&r)).is_equal_to(Some("s-5".to_owned()));
}

#[test]
fn context_new_page_id_resets_seq() {
    let r = PageReports::default();
    r.context("t-1", &holder(), "p-1", 9, on_slide("old")).unwrap();
    r.presenting("t-1", &holder(), "p-1", 9, Some(PresentMode::Screen)).unwrap();
    assert_that!(r.context("t-1", &holder(), "p-2", 0, on_slide("new")).ok()).is_equal_to(Some(true));
    assert_that!(slide_of(&r)).is_equal_to(Some("new".to_owned()));
    assert_that!(r.presenting_now("t-1").0).is_none();
}

#[test]
fn context_from_other_window_is_refused() {
    let r = PageReports::default();
    let e = r.context("t-1", &ReportFrom::Other, "p-1", 1, on_slide("s-1")).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("agent-error-report-elsewhere"));
    assert_that!(r.last_context("t-1")).is_none();
}

#[test]
fn context_from_overlong_window_label_is_refused_as_elsewhere() {
    let r = PageReports::default();
    let from = ReportFrom::Holder { window: "w".repeat(PAGE_ID_MAX + 1) };
    let e = r.context("t-1", &from, "p-1", 1, on_slide("s-1")).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("agent-error-report-elsewhere"));
    assert_that!(r.last_context("t-1")).is_none();
}

#[rstest]
#[case::slide(PageContext {
    ui: AgentUi { slide: Some("s".repeat(UI_TEXT_MAX + 1)), ..AgentUi::default() },
    has_focus: true,
}, "p-1")]
#[case::page_id(on_slide("s-1"), &"p".repeat(PAGE_ID_MAX + 1))]
#[case::empty_page_id(on_slide("s-1"), "")]
fn context_oversized_ui_is_refused(#[case] context: PageContext, #[case] page: &str) {
    let r = PageReports::default();
    let e = r.context("t-1", &holder(), page, 1, context).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("agent-error-ui-too-large"));
    assert_that!(r.last_context("t-1")).is_none();
}

#[test]
fn context_last_seq_is_refused() {
    let r = PageReports::default();
    let e = r.context("t-1", &holder(), "p-1", u64::MAX, on_slide("s-1")).unwrap_err();
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("agent-error-seq-exhausted"));
}

#[test]
fn presenting_older_seq_is_ignored() {
    let r = PageReports::default();
    r.presenting("t-1", &holder(), "p-1", 3, Some(PresentMode::Window)).unwrap();
    assert_that!(r.presenting("t-1", &holder(), "p-1", 2, None).ok()).is_equal_to(Some(false));
    assert_that!(r.presenting_now("t-1").0).is_some();
}

#[test]
fn presenting_none_ends_the_presentation() {
    let r = PageReports::default();
    r.presenting("t-1", &holder(), "p-1", 1, Some(PresentMode::Screen)).unwrap();
    r.presenting("t-1", &holder(), "p-1", 2, None).unwrap();
    assert_that!(r.presenting_now("t-1").0).is_none();
}

#[test]
fn presenting_from_other_window_is_refused() {
    let r = PageReports::default();
    assert_that!(r.presenting("t-1", &ReportFrom::Other, "p-1", 1, Some(PresentMode::Screen)).is_err()).is_true();
    assert_that!(r.presenting_now("t-1").0).is_none();
}

#[test]
fn presenting_now_later_read_has_higher_number() {
    let r = PageReports::default();
    r.presenting("t-1", &holder(), "p-1", 1, Some(PresentMode::Window)).unwrap();
    let (first, second) = (r.presenting_now("t-1"), r.presenting_now("t-1"));
    assert_that!((first.0, second.0)).is_equal_to((Some(PresentMode::Window), Some(PresentMode::Window)));
    assert_that!(second.1).is_greater_than(first.1);
}

#[test]
fn presenting_cleared_on_tab_close() {
    let r = PageReports::default();
    r.presenting("t-1", &holder(), "p-1", 1, Some(PresentMode::Window)).unwrap();
    r.forget_tab("t-1");
    assert_that!(r.presenting_now("t-1").0).is_none();
}

#[test]
fn presenting_cleared_on_window_destroy() {
    let r = PageReports::default();
    r.presenting("t-1", &holder(), "p-1", 1, Some(PresentMode::Window)).unwrap();
    r.presenting("t-2", &ReportFrom::Holder { window: "w-2".into() }, "p-2", 1, Some(PresentMode::Screen)).unwrap();
    assert_that!(r.forget_window("w-1")).is_equal_to(vec!["t-1".to_owned()]);
    assert_that!((r.presenting_now("t-1").0, r.presenting_now("t-2").0)).is_equal_to((None, Some(PresentMode::Screen)));
}

#[test]
fn presenting_cleared_on_new_page_id() {
    let r = PageReports::default();
    r.presenting("t-1", &holder(), "p-1", 7, Some(PresentMode::Window)).unwrap();
    r.context("t-1", &holder(), "p-2", 1, on_slide("s-1")).unwrap();
    assert_that!(r.presenting_now("t-1").0).is_none();
}

#[test]
fn forget_tab_drops_its_reports() {
    let r = PageReports::default();
    r.context("t-1", &holder(), "p-1", 1, on_slide("s-1")).unwrap();
    r.forget_tab("t-1");
    assert_that!(r.last_context("t-1")).is_none();
}

#[test]
fn forget_window_drops_only_its_pages_reports() {
    let r = PageReports::default();
    r.context("t-1", &holder(), "p-1", 1, on_slide("s-1")).unwrap();
    r.context("t-2", &ReportFrom::Holder { window: "w-2".into() }, "p-2", 1, on_slide("s-2")).unwrap();
    r.forget_window("w-1");
    assert_that!(r.last_context("t-1")).is_none();
    assert_that!(r.last_context("t-2").and_then(|c| c.ui.slide)).is_equal_to(Some("s-2".to_owned()));
}

/// A page report for tab `tab`, and any other command for it.
fn report_and_other(tab: &str) -> (crate::AgentCommand, crate::AgentCommand) {
    let parse = |v: serde_json::Value| serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"));
    let report = parse(serde_json::json!({ "cmd": "context", "tab": tab, "page": "p-1", "seq": 1, "has_focus": true }));
    (report, parse(serde_json::json!({ "cmd": "cancel", "tab": tab })))
}

#[test]
fn report_from_only_the_window_holding_the_tab_is_holder() {
    use oku_slides_session::{Fonts, Platform, Settings, Workspace, WorkspaceRequest};
    let dir = tempfile::Builder::new().prefix("oku-slides-agent-report-").tempdir().unwrap_or_else(|e| panic!("{e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(dir.path().to_owned());
    settings.paths.decks = Some(dir.path().join("decks"));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    let ws = Workspace::new(Fonts::default(), &platform, None).unwrap_or_else(|e| panic!("workspace: {e}"));
    let ws = std::sync::Mutex::new(ws);
    Workspace::apply(&ws, WorkspaceRequest::NewWindow).unwrap_or_else(|e| panic!("{e}"));
    let ((t1, cancel), (t2, _)) = (report_and_other("t-1"), report_and_other("t-2"));
    let holder = |w: &str| ReportFrom::Holder { window: w.into() };
    assert_that!(ReportFrom::of(&ws, &t1, Some("w-1"))).is_equal_to(holder("w-1"));
    assert_that!(ReportFrom::of(&ws, &t1, Some("w-2"))).is_equal_to(ReportFrom::Other);
    assert_that!(ReportFrom::of(&ws, &t2, Some("w-2"))).is_equal_to(holder("w-2"));
    assert_that!(ReportFrom::of(&ws, &t1, Some("agent"))).is_equal_to(ReportFrom::Other);
    assert_that!(ReportFrom::of(&ws, &t1, None)).is_equal_to(ReportFrom::Other);
    assert_that!(ReportFrom::of(&ws, &cancel, Some("w-1"))).is_equal_to(ReportFrom::Other);
    // A tab no window holds any more (closed while its report flew): unheld, ignored later.
    let (gone, _) = report_and_other("t-9");
    assert_that!(ReportFrom::of(&ws, &gone, Some("w-1"))).is_equal_to(ReportFrom::Unheld);
    assert_that!(ReportFrom::of(&ws, &gone, None)).is_equal_to(ReportFrom::Other);
}

#[test]
fn report_for_unheld_tab_is_ignored_without_error() {
    let r = PageReports::default();
    assert_that!(r.context("t-1", &ReportFrom::Unheld, "p-1", 1, on_slide("s-1")).ok()).is_equal_to(Some(false));
    assert_that!(r.presenting("t-1", &ReportFrom::Unheld, "p-1", 1, None).ok()).is_equal_to(Some(false));
    assert_that!(slide_of(&r)).is_none();
}
