//! Oku Slides catalog components: the component, its kind and their helpers.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::action::{Action, Dynamic};
use crate::data::SeriesSwatch;
use crate::list::ListColumn;
use crate::props::{ButtonIcon, ChoiceOption, DiffSegment, PromptImage, SlideRef, TextVariant, TimelineStep, Tone};

/// A component: its identifier and what it is. `root` is the root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Component {
    /// Unique identifier within the surface, cited by parents.
    pub id: String,
    /// The component and its properties, flattened under the `component` key.
    #[serde(flatten)]
    pub kind: ComponentKind,
}

/// Components of the [`CATALOG_ID`](crate::CATALOG_ID) catalog. Names and properties follow
/// those of the v0.9 basic catalog; `Stepper`, `ChoiceChips`, `ColorField`,
/// `SlideThumb`, `ThemeChoice`, `DiffText`, `Notice`, `ChartData`, `TableData` are specific to Oku Slides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "component")]
pub enum ComponentKind {
    /// Children stacked vertically.
    Column {
        /// Child identifiers, top to bottom.
        children: Vec<String>,
    },
    /// A card around a single child; `tone` tints it.
    Card {
        /// Child identifier.
        child: String,
        /// Card tone; absent: neutral.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tone: Option<Tone>,
    },
    /// A text.
    Text {
        /// The text shown.
        text: Dynamic<String>,
        /// Text style; absent: body text.
        #[serde(skip_serializing_if = "Option::is_none")]
        variant: Option<TextVariant>,
    },
    /// A text field bound to the model.
    ///
    /// `action` fires when the input is committed (leaving the field or Enter).
    TextField {
        /// Field label.
        label: Dynamic<String>,
        /// Entered text, usually a model path where the UI writes it.
        value: Dynamic<String>,
        /// Sent when the input is committed.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Multiline rich text, edited as markdown.
    ///
    /// `value` reads the markdown shown, `source` the original structured
    /// value; on commit, the UI writes both, then `action` fires.
    RichTextField {
        /// Field label.
        label: Dynamic<String>,
        /// Editing markdown shown in the field.
        value: Dynamic<String>,
        /// Original structured value (paragraphs), rewritten on commit.
        source: Dynamic<Value>,
        /// `#rrggbb` colors offered for highlighting.
        palette: Vec<String>,
        /// Sent after `value` and `source` are written.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// A − value + counter bound to the model; `action` fires on each step.
    Stepper {
        /// Counter label, reused by the buttons' accessible names.
        label: Dynamic<String>,
        /// Current integer value.
        value: Dynamic<i64>,
        /// Smallest value; absent: 1 on the UI side.
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<i64>,
        /// Largest value; absent: 24 on the UI side.
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<i64>,
        /// Sent on each step, after `value` is written.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Children side by side.
    Row {
        /// Child identifiers, left to right.
        children: Vec<String>,
    },
    /// A button.
    ///
    /// `primary`: the surface's main action; `danger`: a committing action
    /// (not undoable, permanent permission).
    Button {
        /// Button label.
        label: Dynamic<String>,
        /// Marks the surface's main action.
        #[serde(skip_serializing_if = "Option::is_none")]
        primary: Option<bool>,
        /// Marks a committing action (not undoable, permanent permission).
        #[serde(skip_serializing_if = "Option::is_none")]
        danger: Option<bool>,
        /// Pictogram only; the label becomes the accessible name and tooltip.
        #[serde(skip_serializing_if = "Option::is_none")]
        icon: Option<ButtonIcon>,
        /// Sent on click.
        action: Action,
    },
    /// Chips, one per option; choosing writes the `id` into `value` then sends `action`.
    ChoiceChips {
        /// The options, one chip each.
        options: Vec<ChoiceOption>,
        /// `id` of the chosen option.
        value: Dynamic<String>,
        /// Sent after the choice is written into `value`.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// A slider; `action` fires on release.
    Slider {
        /// Slider label.
        label: Dynamic<String>,
        /// Current value, between `min` and `max`.
        value: Dynamic<f64>,
        /// Lower bound, strictly below `max`.
        min: f64,
        /// Upper bound.
        max: f64,
        /// Slider step; absent: 1 on the UI side.
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
        /// Sent when the slider is released.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// A color (hex), picked from `palette` or free.
    ColorField {
        /// Field label.
        label: Dynamic<String>,
        /// Chosen color, as `#rrggbb`.
        value: Dynamic<String>,
        /// `#rrggbb` colors offered as chips.
        palette: Vec<String>,
        /// Sent after a color is chosen.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// A dropdown list.
    Select {
        /// List label.
        label: Dynamic<String>,
        /// `id` of the chosen option.
        value: Dynamic<String>,
        /// The options offered, in order.
        options: Vec<ChoiceOption>,
        /// Sent after the choice is written into `value`.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// The table of a list, one row per item and one column per entry.
    ///
    /// Rows are the items of `rows` (objects with an `id`), columns the
    /// `columns` entries. Controlled: the UI writes the change
    /// (a `ListOp`, or `{op: "replace", items}` on paste) into `change`,
    /// then `action` fires; the rows come back through `rows`. `markdown` gives
    /// the editing markdown of `rich` columns, by item id then column.
    ListTable {
        /// Table label.
        label: Dynamic<String>,
        /// The items shown, objects with an `id`.
        rows: Dynamic<Value>,
        /// Where the UI writes the requested change before the action.
        change: Dynamic<Value>,
        /// Editing markdown of `rich` columns, by item id then column.
        markdown: Dynamic<Value>,
        /// The columns, in display order.
        columns: Vec<ListColumn>,
        /// Minimum number of items: below it, removing is no longer possible.
        min: u32,
        /// Maximum number of items; absent: no limit.
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<u32>,
        /// Name of one item, for the add and remove buttons.
        #[serde(skip_serializing_if = "Option::is_none")]
        item_label: Option<String>,
        /// `#rrggbb` colors offered for highlighting in `rich` columns.
        palette: Vec<String>,
        /// Sent after `change` is written.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// Thumbnail of a slide rendered by the session (SVG), clickable.
    SlideThumb {
        /// The slide's SVG render, produced by the host.
        svg: String,
        /// Accessible name of the thumbnail.
        label: Dynamic<String>,
        /// Sent on a click on the thumbnail.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// A theme offered by the host: thumbnail, swatches, name.
    ///
    /// `svg` absent: render bounded, failed or empty deck; the card stays selectable.
    /// `slide`: the slide the thumbnail shows (the scene preview only holds for it).
    ThemeChoice {
        /// SVG thumbnail of a slide under this theme; absent if the render is missing.
        #[serde(skip_serializing_if = "Option::is_none")]
        svg: Option<String>,
        /// Theme name.
        name: String,
        /// Identifier of the slide shown by the thumbnail.
        slide: String,
        /// Hex colors read from the theme's tokens.
        swatches: Vec<String>,
        /// The deck's theme.
        active: bool,
        /// Sent when the theme is chosen.
        #[serde(skip_serializing_if = "Option::is_none")]
        action: Option<Action>,
    },
    /// A text and its changes, word by word.
    DiffText {
        /// The text chunks, in order.
        segments: Vec<DiffSegment>,
    },
    /// The user's prompt and its pasted images (host only).
    Prompt {
        /// The prompt text.
        text: Dynamic<String>,
        /// The pasted images, shown as chips.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        images: Vec<PromptImage>,
    },
    /// The progress of a turn; collapsed into a `summary` line (host only).
    Timeline {
        /// The turn's steps, in order.
        steps: Dynamic<Vec<TimelineStep>>,
        /// Collapsed: only the `summary` line shows.
        collapsed: Dynamic<bool>,
        /// The line shown when the timeline is collapsed.
        summary: Dynamic<String>,
    },
    /// A turn's conclusion, shown as a receipt.
    ///
    /// It holds a title, 1 to 4 facts, an optional slide (as a thumbnail) and a color.
    Receipt {
        /// Receipt title.
        title: Dynamic<String>,
        /// From 1 to 4 facts, one per line.
        facts: Vec<String>,
        /// The slide shown as a thumbnail, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slide: Option<SlideRef>,
        /// A `#rrggbb` color shown as a swatch, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        swatch: Option<String>,
    },
    /// A child collapsed behind `summary`; `open`: expanded on first render.
    Disclosure {
        /// The always-visible line that expands the child.
        summary: Dynamic<String>,
        /// Identifier of the collapsed child.
        child: String,
        /// Expanded on first render; absent: collapsed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        open: Option<bool>,
    },
    /// A counter or a status.
    Badge {
        /// The badge text.
        text: Dynamic<String>,
        /// Badge tone; absent: neutral.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tone: Option<Tone>,
    },
    /// Markdown (rendered without raw HTML).
    Markdown {
        /// The Markdown source.
        text: Dynamic<String>,
    },
    /// An announced banner (host only): `text`, the `children` buttons, then the `close` cross.
    Notice {
        /// The banner message.
        text: Dynamic<String>,
        /// Identifiers of the banner's buttons.
        children: Vec<String>,
        /// Sent by the close cross.
        close: Action,
        /// Accessible name and tooltip of the cross.
        close_label: Dynamic<String>,
    },
    /// A chart's data and settings (host only).
    ///
    /// A change is written into `edit`, then `action` fires.
    ChartData {
        /// Type of the first series; absent without data.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
        /// `dataset.source`: row 0 = headers; numbers, texts, booleans or `null`.
        source: Vec<Vec<Value>>,
        /// Columns edited as checkboxes (the Total of a waterfall).
        checkbox_columns: Vec<usize>,
        /// Stacked series; absent when stacking makes no sense.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stacked: Option<bool>,
        /// The series that can be colored.
        series: Vec<SeriesSwatch>,
        /// The theme's chart palette, as `#rrggbb`.
        palette: Vec<String>,
        /// Where the UI writes the requested change.
        edit: Dynamic<Value>,
        /// Sent after `edit` is written.
        action: Action,
    },
    /// A table's data and display options (host only); like `ChartData`.
    TableData {
        /// `dataset.source`: row 0 = column names.
        source: Vec<Vec<Value>>,
        /// First row as a header.
        header_row: bool,
        /// First column as a header.
        header_column: bool,
        /// Striped rows.
        striped: bool,
        /// Alignment of each column (`auto`, `start`, `center`, `end`); no entry: `auto`.
        align: Vec<String>,
        /// Where the UI writes the requested change.
        edit: Dynamic<Value>,
        /// Sent after `edit` is written.
        action: Action,
    },
}

impl ComponentKind {
    /// The child identifiers, in order.
    pub fn children(&self) -> Vec<&str> {
        // No `_`: a new component must state here whether it has children (`render` validation depends on it).
        match self {
            Self::Column { children } | Self::Row { children } | Self::Notice { children, .. } => {
                children.iter().map(String::as_str).collect()
            }
            Self::Card { child, .. } | Self::Disclosure { child, .. } => vec![child.as_str()],
            Self::Text { .. }
            | Self::TextField { .. }
            | Self::RichTextField { .. }
            | Self::Stepper { .. }
            | Self::Button { .. }
            | Self::ChoiceChips { .. }
            | Self::Slider { .. }
            | Self::ColorField { .. }
            | Self::Select { .. }
            | Self::ListTable { .. }
            | Self::SlideThumb { .. }
            | Self::ThemeChoice { .. }
            | Self::DiffText { .. }
            | Self::Prompt { .. }
            | Self::Timeline { .. }
            | Self::Receipt { .. }
            | Self::Badge { .. }
            | Self::Markdown { .. }
            | Self::ChartData { .. }
            | Self::TableData { .. } => Vec::new(),
        }
    }

    /// The actions the component can send.
    #[must_use]
    pub fn actions(&self) -> Vec<&Action> {
        match self {
            Self::Button { action, .. }
            | Self::Notice { close: action, .. }
            | Self::ChartData { action, .. }
            | Self::TableData { action, .. } => vec![action],
            Self::TextField { action, .. }
            | Self::RichTextField { action, .. }
            | Self::Stepper { action, .. }
            | Self::ChoiceChips { action, .. }
            | Self::Slider { action, .. }
            | Self::ColorField { action, .. }
            | Self::Select { action, .. }
            | Self::ListTable { action, .. }
            | Self::SlideThumb { action, .. }
            | Self::ThemeChoice { action, .. } => action.iter().collect(),
            // No `_`: a new component must state here whether it sends an action (`render` validation depends on it).
            Self::Column { .. }
            | Self::Row { .. }
            | Self::Card { .. }
            | Self::Text { .. }
            | Self::DiffText { .. }
            | Self::Prompt { .. }
            | Self::Timeline { .. }
            | Self::Receipt { .. }
            | Self::Disclosure { .. }
            | Self::Badge { .. }
            | Self::Markdown { .. } => Vec::new(),
        }
    }

    /// The same component, read-only; `None` for a button, a banner or a data grid.
    #[must_use]
    pub fn without_action(self) -> Option<Self> {
        Some(match self {
            // Without an action, a button, a banner or a data grid makes no sense.
            Self::Button { .. } | Self::Notice { .. } | Self::ChartData { .. } | Self::TableData { .. } => return None,
            Self::TextField { label, value, .. } => Self::TextField { label, value, action: None },
            Self::RichTextField { label, value, source, palette, .. } => {
                Self::RichTextField { label, value, source, palette, action: None }
            }
            Self::Stepper { label, value, min, max, .. } => Self::Stepper { label, value, min, max, action: None },
            Self::ChoiceChips { options, value, .. } => Self::ChoiceChips { options, value, action: None },
            Self::Slider { label, value, min, max, step, .. } => {
                Self::Slider { label, value, min, max, step, action: None }
            }
            Self::ColorField { label, value, palette, .. } => Self::ColorField { label, value, palette, action: None },
            Self::Select { label, value, options, .. } => Self::Select { label, value, options, action: None },
            Self::ListTable { action: _, label, rows, change, markdown, columns, min, max, item_label, palette } => {
                Self::ListTable { label, rows, change, markdown, columns, min, max, item_label, palette, action: None }
            }
            Self::SlideThumb { svg, label, .. } => Self::SlideThumb { svg, label, action: None },
            Self::ThemeChoice { svg, name, slide, swatches, active, .. } => {
                Self::ThemeChoice { svg, name, slide, swatches, active, action: None }
            }
            // No `_`: a new component must state here how it freezes.
            kind @ (Self::Column { .. }
            | Self::Row { .. }
            | Self::Card { .. }
            | Self::Text { .. }
            | Self::DiffText { .. }
            | Self::Prompt { .. }
            | Self::Timeline { .. }
            | Self::Receipt { .. }
            | Self::Disclosure { .. }
            | Self::Badge { .. }
            | Self::Markdown { .. }) => kind,
        })
    }
}

impl Component {
    /// `false` for host-only components (`Prompt`, `Timeline`, `Receipt`, `SlideThumb`, `Notice`,
    /// `ThemeChoice`, `ChartData`, `TableData`).
    ///
    /// A receipt goes through the `report` tool (bounds, thumbnail rendered by the host); a thumbnail
    /// is always rendered by the host.
    #[must_use]
    pub fn agent_allowed(&self) -> bool {
        !matches!(
            self.kind,
            ComponentKind::Prompt { .. }
                | ComponentKind::Timeline { .. }
                | ComponentKind::Receipt { .. }
                | ComponentKind::SlideThumb { .. }
                | ComponentKind::Notice { .. }
                | ComponentKind::ThemeChoice { .. }
                | ComponentKind::ChartData { .. }
                | ComponentKind::TableData { .. }
        )
    }

    /// The `kind` component, under the identifier `id`.
    pub fn new(id: impl Into<String>, kind: ComponentKind) -> Self {
        Self { id: id.into(), kind }
    }

    /// A card around `child`.
    pub fn card(id: impl Into<String>, child: &str) -> Self {
        Self::new(id, ComponentKind::Card { child: child.to_owned(), tone: None })
    }

    /// A column of `children`, top to bottom.
    pub fn column(id: impl Into<String>, children: &[&str]) -> Self {
        Self::new(id, ComponentKind::Column { children: children.iter().map(|c| (*c).to_owned()).collect() })
    }

    /// A row of `children`, left to right.
    pub fn row(id: impl Into<String>, children: &[&str]) -> Self {
        Self::new(id, ComponentKind::Row { children: children.iter().map(|c| (*c).to_owned()).collect() })
    }

    /// A literal text, with style `variant`.
    pub fn text(id: impl Into<String>, text: impl Into<String>, variant: Option<TextVariant>) -> Self {
        Self::new(id, ComponentKind::Text { text: Dynamic::Literal(text.into()), variant })
    }

    /// A button with a literal label; `primary` marks it as the main action.
    pub fn button(id: impl Into<String>, label: impl Into<String>, primary: bool, action: Action) -> Self {
        Self::new(
            id,
            ComponentKind::Button {
                label: Dynamic::Literal(label.into()),
                primary: primary.then_some(true),
                danger: None,
                icon: None,
                action,
            },
        )
    }

    /// The same button, marked or not as a committing action.
    ///
    /// Only affects buttons: any other component is returned as is.
    #[must_use]
    pub fn danger(mut self, on: bool) -> Self {
        debug_assert!(matches!(self.kind, ComponentKind::Button { .. }), "danger() ne vaut que pour un bouton");
        if let ComponentKind::Button { danger, .. } = &mut self.kind {
            *danger = on.then_some(true);
        }
        self
    }

    /// The same button, shown by the `icon` pictogram instead of its label.
    ///
    /// Only affects buttons: any other component is returned as is.
    #[must_use]
    pub fn icon(mut self, icon: ButtonIcon) -> Self {
        debug_assert!(matches!(self.kind, ComponentKind::Button { .. }), "icon() ne vaut que pour un bouton");
        if let ComponentKind::Button { icon: slot, .. } = &mut self.kind {
            *slot = Some(icon);
        }
        self
    }
}

#[cfg(test)]
#[path = "component_tests.rs"]
mod tests;
