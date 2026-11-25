//! The playbook's host-only examples: components the agent never sends (`agent` false).

use serde_json::{Value, json};

use super::entries::{lit, palette};
use super::{Example, example};
use crate::action::{Action, Dynamic};
use crate::component::{Component, ComponentKind as K};
use crate::data::{DataChoice, DataRef, DataRow, DataUser, SeriesSwatch};
use crate::props::{PromptImage, SlideRef, StepStatus, TimelineStep, ToolCallDetail};

/// The action a host data component sends after writing `/edit`.
fn data_action() -> Action {
    Action::event("data", [("edit", Dynamic::path("/edit"))])
}

pub(super) fn slide_thumb() -> Vec<Example> {
    vec![example(
        "default",
        "A slide rendered by the host; a click sends the action.",
        false,
        K::SlideThumb {
            svg: r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90"><rect width="160" height="90" fill="#1f4e79"/></svg>"##.into(),
            label: lit("4 · Quarterly review"),
            action: Some(Action::named("slide")),
        },
        vec![],
        json!({}),
    )]
}

pub(super) fn theme_choice() -> Vec<Example> {
    let choice = |svg: Option<String>, active: bool| K::ThemeChoice {
        svg,
        name: "Paper".into(),
        slide: "s-4".into(),
        swatches: vec!["#ffffff".into(), "#2f5fb3".into()],
        active,
        action: Some(Action::event("theme", [("theme", Dynamic::Literal(json!("papier")))])),
    };
    let thumb = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90"><rect width="160" height="90" fill="#ffffff"/><rect x="12" y="12" width="90" height="10" fill="#2f5fb3"/></svg>"##;
    vec![
        example(
            "active",
            "The deck's current theme, with its thumbnail and swatches.",
            false,
            choice(Some(thumb.into()), true),
            vec![],
            json!({}),
        ),
        example(
            "no-thumbnail",
            "A theme whose render failed: still selectable.",
            false,
            choice(None, false),
            vec![],
            json!({}),
        ),
    ]
}

pub(super) fn prompt() -> Vec<Example> {
    vec![example(
        "with-image",
        "The user's prompt opening a turn, with a pasted image chip.",
        false,
        K::Prompt {
            text: lit("Use this photo as the background of slide four."),
            images: vec![PromptImage { mime: "image/png".into(), width: 1280, height: 720 }],
        },
        vec![],
        json!({}),
    )]
}

pub(super) fn timeline() -> Vec<Example> {
    let mut note = TimelineStep::new("Night blue chosen.", StepStatus::Completed);
    note.detail = Some("The title turns white.".into());
    let mut tools = TimelineStep::new("apply", StepStatus::Completed);
    tools.count = Some(2);
    tools.calls = vec![ToolCallDetail {
        label: "apply".into(),
        status: StepStatus::Completed,
        input: Some("{\n  \"slide\": 4\n}".into()),
        output: Some("background set".into()),
    }];
    let steps = vec![note, tools, TimelineStep::new("render_slide", StepStatus::InProgress)];
    // Typed literal steps: the host binds them to its model, the catalog accepts both.
    let kind = |collapsed: bool| K::Timeline {
        steps: Dynamic::Literal(steps.clone()),
        collapsed: Dynamic::Literal(collapsed),
        summary: lit("3 tools · 1 note · 12 s"),
    };
    vec![
        example("expanded", "A turn's steps while it runs.", false, kind(false), vec![], json!({})),
        example("collapsed", "A finished turn folded into its summary line.", false, kind(true), vec![], json!({})),
    ]
}

pub(super) fn receipt() -> Vec<Example> {
    vec![example(
        "with-slide",
        "A turn's conclusion; with no deck its thumbnail shows as missing.",
        false,
        K::Receipt {
            title: lit("Slide 4: night blue background"),
            facts: vec!["title in white".into(), "light blue kicker".into()],
            slide: Some(SlideRef { id: "s-4".into(), hash: "0123456789abcdef0123456789abcdef".into() }),
            swatch: Some("#00154e".into()),
        },
        vec![],
        json!({}),
    )]
}

pub(super) fn notice() -> Vec<Example> {
    vec![example(
        "with-button",
        "A host banner with its buttons and a close cross.",
        false,
        K::Notice {
            text: lit("A new version is ready."),
            children: vec!["restart".into()],
            close: Action::named("dismiss"),
            close_label: lit("Dismiss"),
        },
        vec![Component::button("restart", "Restart", false, Action::named("restart"))],
        json!({}),
    )]
}

pub(super) fn chart_grid() -> Vec<Example> {
    vec![example(
        "bar",
        "A chart's data table, series colors and stacking.",
        false,
        K::ChartData {
            kind: Some("bar".into()),
            source: vec![
                vec![json!("Month"), json!("Sales"), json!("Target")],
                vec![json!("Jan"), json!(3), json!(4)],
                vec![json!("Feb"), json!(5), json!(4)],
            ],
            row_ids: vec!["jan".into(), "feb".into()],
            column_ids: vec!["month".into(), "sales".into(), "target".into()],
            checkbox_columns: vec![],
            stacked: Some(false),
            series: vec![
                SeriesSwatch { name: "Sales".into(), color: None },
                SeriesSwatch { name: "Target".into(), color: Some("#1f4e79".into()) },
            ],
            palette: palette(),
            data: DataRef { id: "sales".into(), name: "sales".into(), label: Some("Monthly sales".into()), users: 2 },
            choices: vec![
                DataChoice { id: "sales".into(), name: "sales".into(), label: Some("Monthly sales".into()) },
                DataChoice { id: "budget".into(), name: "budget".into(), label: None },
            ],
            edit: Dynamic::path("/edit"),
            action: data_action(),
        },
        vec![],
        json!({ "edit": null }),
    )]
}

pub(super) fn table_grid() -> Vec<Example> {
    vec![example(
        "default",
        "A table's cells and display options.",
        false,
        K::TableData {
            source: vec![vec![json!("A"), json!("B")], vec![json!(1), Value::Null]],
            row_ids: vec!["r1".into()],
            column_ids: vec!["a".into(), "b".into()],
            header_row: true,
            header_column: false,
            striped: true,
            align: vec!["auto".into(), "end".into()],
            data: DataRef { id: "scores".into(), name: "scores".into(), label: None, users: 1 },
            choices: vec![DataChoice { id: "scores".into(), name: "scores".into(), label: None }],
            edit: Dynamic::path("/edit"),
            action: data_action(),
        },
        vec![],
        json!({ "edit": null }),
    )]
}

pub(super) fn data_list() -> Vec<Example> {
    let user = |slide: &str, element: &str, label: &str| DataUser {
        slide: slide.into(),
        element: element.into(),
        label: label.into(),
    };
    let row = |id: &str, label: Option<&str>, users: Vec<DataUser>, error: Option<&str>| DataRow {
        id: id.into(),
        name: id.into(),
        label: label.map(Into::into),
        kind: "manual".into(),
        columns: 3,
        rows: 12,
        users,
        error: error.map(Into::into),
    };
    let list = |rows| K::DataList { rows, edit: Dynamic::path("/edit"), action: data_action() };
    vec![
        example(
            "default",
            "The deck's data: size, views, one unused.",
            false,
            list(vec![
                row(
                    "sales",
                    Some("Monthly sales"),
                    vec![user("s1", "ch1", "Slide 1 · ch1"), user("s3", "tb1", "Slide 3 · tb1")],
                    None,
                ),
                row("budget", None, vec![], None),
            ]),
            vec![],
            json!({ "edit": null }),
        ),
        example(
            "refused",
            "A removal refused: the views that use it, inline.",
            false,
            list(vec![row("sales", None, vec![user("s1", "ch1", "Slide 1 · ch1")], Some("Data used by ch1"))]),
            vec![],
            json!({ "edit": null }),
        ),
    ]
}

pub(super) fn data_sheet() -> Vec<Example> {
    vec![example(
        "default",
        "One data's rows, a page at a time, with its column types.",
        false,
        K::DataSheet {
            data: DataRef { id: "sales".into(), name: "sales".into(), label: None, users: 1 },
            source: vec![
                vec![json!("Month"), json!("Sales")],
                vec![json!("Jan"), json!(3)],
                vec![json!("Feb"), json!(5)],
            ],
            row_ids: vec!["jan".into(), "feb".into()],
            column_ids: vec!["month".into(), "sales".into()],
            types: vec!["text".into(), "number".into()],
            offset: 0,
            total: 2,
            edit: Dynamic::path("/edit"),
            action: data_action(),
        },
        vec![],
        json!({ "edit": null }),
    )]
}
