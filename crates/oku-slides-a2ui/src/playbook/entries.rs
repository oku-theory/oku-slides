//! The playbook's examples, in catalog order; `agent` is true only where `render` accepts it.

use serde_json::{Value, json};

use super::host_entries::{chart_data, notice, prompt, receipt, slide_thumb, table_data, theme_choice, timeline};
use super::{Example, example};
use crate::action::{Action, Dynamic};
use crate::component::{Component, ComponentKind as K};
use crate::list::{ColumnKind, ListColumn, SubColumn, SubList};
use crate::props::{ButtonIcon, ChoiceOption, DiffKind, DiffSegment, TextVariant, Tone};

/// The palette offered for highlighting and colors.
const PALETTE: [&str; 3] = ["#1f4e79", "#c2573a", "#2f855a"];

/// A literal text.
pub(super) fn lit(s: &str) -> Dynamic<String> {
    Dynamic::Literal(s.to_owned())
}

/// A text component with a literal value.
fn text(id: &str, s: &str, variant: Option<TextVariant>) -> Component {
    Component::text(id, s, variant)
}

/// The only action `render` lets the agent send: the button's label as the next prompt.
fn suggest(label: &str) -> Action {
    Action::event("suggest", [("prompt", Dynamic::Literal(Value::String(label.to_owned())))])
}

/// A choice option.
fn option(id: &str, label: &str, hint: Option<&str>) -> ChoiceOption {
    ChoiceOption { id: id.to_owned(), label: label.to_owned(), hint: hint.map(str::to_owned) }
}

/// A list column with no options and no nested list.
fn column(key: &str, label: &str, kind: ColumnKind) -> ListColumn {
    ListColumn { key: key.to_owned(), label: label.to_owned(), kind, options: Vec::new(), list: None }
}

/// The palette offered for highlighting and colors, as a prop value.
pub(super) fn palette() -> Vec<String> {
    PALETTE.map(str::to_owned).to_vec()
}

/// Every example, in catalog order: one function per component, in `ComponentKind` order.
pub(super) fn all() -> Vec<Example> {
    [
        column_layout(),
        card(),
        text_atom(),
        text_field(),
        rich_text_field(),
        stepper(),
        row(),
        button(),
        choice_chips(),
        slider(),
        color_field(),
        select(),
        switch(),
        list_table(),
        slide_thumb(),
        theme_choice(),
        diff_text(),
        prompt(),
        timeline(),
        receipt(),
        disclosure(),
        badge(),
        markdown(),
        notice(),
        chart_data(),
        table_data(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn column_layout() -> Vec<Example> {
    vec![example(
        "default",
        "Stacks children top to bottom: the usual root of a card's body.",
        true,
        K::Column { children: vec!["title".into(), "body".into()] },
        vec![
            text("title", "Three palettes", Some(TextVariant::H4)),
            text("body", "Pick one to apply it to every slide.", None),
        ],
        json!({}),
    )]
}

fn card() -> Vec<Example> {
    let body = || vec![text("t", "A card groups related content.", None)];
    vec![
        example(
            "default",
            "Frames one child; the neutral card of a thread.",
            true,
            K::Card { child: "t".into(), tone: None },
            body(),
            json!({}),
        ),
        example(
            "accent",
            "Highlights a conclusion or a suggestion.",
            true,
            K::Card { child: "t".into(), tone: Some(Tone::Accent) },
            body(),
            json!({}),
        ),
        example(
            "danger",
            "Marks a host warning; render refuses the danger tone.",
            false,
            K::Card { child: "t".into(), tone: Some(Tone::Danger) },
            body(),
            json!({}),
        ),
    ]
}

fn text_atom() -> Vec<Example> {
    let variant = |name: &'static str, note: &'static str, v: TextVariant, s: &str| {
        example(name, note, true, K::Text { text: lit(s), variant: Some(v) }, vec![], json!({}))
    };
    vec![
        variant("h4", "A section heading inside a card.", TextVariant::H4, "Palette"),
        variant(
            "body",
            "Body text, the default style.",
            TextVariant::Body,
            "The title turns white on the dark background.",
        ),
        variant("caption", "A smaller line under a heading or a figure.", TextVariant::Caption, "Source: Insee, 2024"),
        variant("note", "A dimmed line, truncated when too long.", TextVariant::Note, "Applied to slides 2 to 9"),
        example(
            "bound",
            "Reads its text from the data model; render accepts literals only.",
            false,
            K::Text { text: Dynamic::path("/title"), variant: None },
            vec![],
            json!({ "title": "Quarterly review" }),
        ),
    ]
}

fn text_field() -> Vec<Example> {
    vec![example(
        "bound",
        "Edits a model value; the action fires on Enter or blur.",
        false,
        K::TextField { label: lit("Title"), value: Dynamic::path("/title"), action: Some(Action::named("commit")) },
        vec![],
        json!({ "title": "Quarterly review" }),
    )]
}

fn rich_text_field() -> Vec<Example> {
    vec![example(
        "bound",
        "Edits paragraphs as markdown; writes the markdown and the paragraphs, then acts.",
        false,
        K::RichTextField {
            label: lit("Body"),
            value: Dynamic::path("/body"),
            source: Dynamic::path("/body-source"),
            palette: palette(),
            action: Some(Action::named("commit")),
        },
        vec![],
        json!({ "body": "**Bold** start", "body-source": [{ "runs": [{ "t": "Bold", "bold": true }, { "t": " start" }] }] }),
    )]
}

fn stepper() -> Vec<Example> {
    vec![example(
        "bounded",
        "A small integer with − and + buttons, between min and max.",
        false,
        K::Stepper {
            label: lit("Columns"),
            value: Dynamic::path("/columns"),
            min: Some(1),
            max: Some(4),
            action: Some(Action::named("step")),
        },
        vec![],
        json!({ "columns": 2 }),
    )]
}

fn row() -> Vec<Example> {
    vec![example(
        "default",
        "Places children side by side, such as badges or buttons.",
        true,
        K::Row { children: vec!["a".into(), "b".into()] },
        vec![
            Component::new("a", K::Badge { text: lit("12 slides"), tone: None }),
            Component::new("b", K::Badge { text: lit("Saved"), tone: Some(Tone::Accent) }),
        ],
        json!({}),
    )]
}

fn button() -> Vec<Example> {
    let host =
        |name: &'static str, note: &'static str, b: Component| example(name, note, false, b.kind, vec![], json!({}));
    vec![
        example(
            "suggest",
            "Sends its label as the user's next prompt: the only button render allows.",
            true,
            K::Button {
                label: lit("Make it shorter"),
                primary: None,
                danger: None,
                icon: None,
                action: suggest("Make it shorter"),
            },
            vec![],
            json!({}),
        ),
        host(
            "primary",
            "The surface's main action (host only).",
            Component::button("root", "Apply", true, Action::named("apply")),
        ),
        host(
            "danger",
            "A committing action, not undoable (host only).",
            Component::button_builder("root", "Delete the pack", Action::named("delete")).danger(true).build(),
        ),
        host(
            "icon",
            "A pictogram; the label becomes its accessible name (host only).",
            Component::button_builder("root", "Close", Action::named("close")).icon(ButtonIcon::Close).build(),
        ),
    ]
}

fn choice_chips() -> Vec<Example> {
    vec![example(
        "default",
        "A few short choices as chips; the chosen id goes to the model.",
        false,
        K::ChoiceChips {
            options: vec![
                option("a", "Two columns", None),
                option("b", "Quote", None),
                option("c", "Full image", None),
            ],
            value: Dynamic::path("/layout"),
            action: Some(Action::named("choose")),
        },
        vec![],
        json!({ "layout": "b" }),
    )]
}

fn slider() -> Vec<Example> {
    vec![example(
        "stepped",
        "A bounded number; the action fires on release.",
        false,
        K::Slider {
            label: lit("Opacity"),
            value: Dynamic::path("/opacity"),
            min: 0.0,
            max: 100.0,
            step: Some(5.0),
            action: Some(Action::named("slide")),
        },
        vec![],
        json!({ "opacity": 60 }),
    )]
}

fn color_field() -> Vec<Example> {
    vec![example(
        "palette",
        "A hex color, picked from the palette or typed.",
        false,
        K::ColorField {
            label: lit("Accent"),
            value: Dynamic::path("/color"),
            palette: palette(),
            action: Some(Action::named("color")),
        },
        vec![],
        json!({ "color": "#1f4e79" }),
    )]
}

fn select() -> Vec<Example> {
    vec![example(
        "with-hints",
        "A dropdown for longer option lists; a hint shows next to a label.",
        false,
        K::Select {
            label: lit("Theme"),
            value: Dynamic::path("/theme"),
            options: vec![
                option("papier", "Paper", Some("light")),
                option("pro", "Pro", Some("dark")),
                option("neutre", "Neutral", None),
            ],
            action: Some(Action::named("choose")),
        },
        vec![],
        json!({ "theme": "papier" }),
    )]
}

fn switch() -> Vec<Example> {
    vec![example(
        "bound",
        "An on/off setting; it writes a boolean.",
        false,
        K::Switch { label: lit("Highlight"), value: Dynamic::path("/on"), action: Some(Action::named("toggle")) },
        vec![],
        json!({ "on": true }),
    )]
}

fn list_table() -> Vec<Example> {
    let flat = vec![
        column("name", "Name", ColumnKind::Text),
        column("count", "Count", ColumnKind::Number),
        column("done", "Done", ColumnKind::Flag),
    ];
    let mut tone = column("tone", "Tone", ColumnKind::Select);
    tone.options = vec![option("a", "Two columns", None), option("b", "Quote", Some("sober"))];
    let mut tasks = column("tasks", "Tasks", ColumnKind::List);
    tasks.list = Some(Box::new(SubList {
        columns: vec![
            SubColumn { key: "done".into(), label: "Done".into(), kind: ColumnKind::Flag, options: Vec::new() },
            SubColumn { key: "note".into(), label: "Note".into(), kind: ColumnKind::Rich, options: Vec::new() },
        ],
        min: 0,
        max: Some(8),
        item_label: Some("task".into()),
    }));
    let table = |columns: Vec<ListColumn>| K::ListTable {
        label: lit("Steps"),
        rows: Dynamic::path("/rows"),
        change: Dynamic::path("/change"),
        markdown: Dynamic::path("/markdown"),
        columns,
        min: 1,
        max: Some(10),
        item_label: Some("step".into()),
        palette: palette(),
        action: Some(Action::named("list")),
    };
    vec![
        example(
            "flat",
            "A list of items edited in place: text, number and checkbox columns.",
            false,
            table(flat),
            vec![],
            json!({ "rows": [{ "id": "r0", "name": "Intro", "count": 3, "done": true }, { "id": "r1", "name": "Plan", "count": 5, "done": false }],
                    "change": null, "markdown": {} }),
        ),
        example(
            "nested",
            "Rich, select and a one-level nested list column.",
            false,
            table(vec![
                column("date", "Date", ColumnKind::Text),
                column("text", "Text", ColumnKind::Rich),
                tone,
                tasks,
            ]),
            vec![],
            json!({
                "rows": [{ "id": "d0", "date": "2019", "text": [{ "runs": [{ "t": "Founded" }] }], "tone": "a",
                           "tasks": [{ "id": "t1", "done": true, "note": [{ "runs": [{ "t": "Review" }] }] }] }],
                "change": null,
                "markdown": { "d0": { "text": "Founded" }, "d0/tasks/t1": { "note": "Review" } },
            }),
        ),
    ]
}

fn diff_text() -> Vec<Example> {
    let seg = |kind: DiffKind, t: &str| DiffSegment { kind, text: t.to_owned() };
    vec![example(
        "default",
        "Shows a proposed rewrite word by word.",
        true,
        K::DiffText {
            segments: vec![
                seg(DiffKind::Keep, "Sales "),
                seg(DiffKind::Del, "rose"),
                seg(DiffKind::Ins, "grew"),
                seg(DiffKind::Keep, " by 12%."),
            ],
        },
        vec![],
        json!({}),
    )]
}

fn disclosure() -> Vec<Example> {
    let detail = || vec![text("d", "Slides 2, 5 and 9 use the old title size.", None)];
    vec![
        example(
            "collapsed",
            "Hides details behind a summary line.",
            true,
            K::Disclosure { summary: lit("3 slides changed"), child: "d".into(), open: None },
            detail(),
            json!({}),
        ),
        example(
            "open",
            "Shown expanded on first render.",
            true,
            K::Disclosure { summary: lit("3 slides changed"), child: "d".into(), open: Some(true) },
            detail(),
            json!({}),
        ),
    ]
}

fn badge() -> Vec<Example> {
    let badge = |name: &'static str, note: &'static str, agent: bool, tone: Option<Tone>| {
        example(name, note, agent, K::Badge { text: lit("12 slides"), tone }, vec![], json!({}))
    };
    vec![
        badge("neutral", "A count or a status.", true, None),
        badge("accent", "A status worth noticing.", true, Some(Tone::Accent)),
        badge("danger", "A host alert; render refuses the danger tone.", false, Some(Tone::Danger)),
    ]
}

fn markdown() -> Vec<Example> {
    vec![example(
        "default",
        "Formatted text without raw HTML: bold, italic, lists, links.",
        true,
        K::Markdown { text: lit("**Bold**, *italic* and a list:\n\n- first\n- second") },
        vec![],
        json!({}),
    )]
}
