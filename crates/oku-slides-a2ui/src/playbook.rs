//! The playbook: every catalog component shown in its meaningful variants.
//!
//! [`examples`] feeds the dev gallery (`ui/playbook.html`, through `playbook.json`) and the
//! agent reference (`agent_reference`), so both show the same surfaces.

mod entries;
mod host_entries;
pub(crate) mod reference;

use serde::Serialize;
use serde_json::Value;

use crate::component::{Component, ComponentKind};

pub use reference::{
    ComponentReference, ExampleReference, PlaybookError, PropReference, agent_reference, render_components,
};

/// Where an example sits in the gallery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    /// Column, Row, Card: they place other components.
    Layout,
    /// A text, a badge, a button, a disclosure: one visible piece.
    Atom,
    /// An input bound to the data model; the agent shows it through `show_params`.
    Field,
    /// A component only the host builds (`Component::agent_allowed` is false).
    HostOnly,
}

/// One playbook example: a component in one variant, as a small surface.
#[derive(Debug, Clone, PartialEq)]
pub struct Example {
    /// The v0.9 component name, such as `Button`.
    pub component: &'static str,
    /// The variant, unique within the component, such as `danger`.
    pub variant: &'static str,
    /// One English sentence: when to use it.
    pub note: &'static str,
    /// Its place in the gallery, from the root's kind.
    pub group: Group,
    /// Whether `render` accepts it (a session test checks it).
    pub agent: bool,
    /// The tree, root first, under the id `root`.
    pub components: Vec<Component>,
    /// The surface's data model; `{}` when every value is literal.
    pub data: Value,
}

/// Every example, in catalog order.
#[must_use]
pub fn examples() -> Vec<Example> {
    entries::all()
}

/// The v0.9 name and the group of a kind; no `_` arm, so a new variant must be named here.
fn kind_info(kind: &ComponentKind) -> (&'static str, Group) {
    use ComponentKind as K;
    use Group::{Atom, Field, HostOnly, Layout};
    match kind {
        K::Column { .. } => ("Column", Layout),
        K::Card { .. } => ("Card", Layout),
        K::Text { .. } => ("Text", Atom),
        K::TextField { .. } => ("TextField", Field),
        K::RichTextField { .. } => ("RichTextField", Field),
        K::Stepper { .. } => ("Stepper", Field),
        K::Row { .. } => ("Row", Layout),
        K::Button { .. } => ("Button", Atom),
        K::ChoiceChips { .. } => ("ChoiceChips", Field),
        K::Slider { .. } => ("Slider", Field),
        K::ColorField { .. } => ("ColorField", Field),
        K::Select { .. } => ("Select", Field),
        K::ListTable { .. } => ("ListTable", Field),
        K::SlideThumb { .. } => ("SlideThumb", HostOnly),
        K::ThemeChoice { .. } => ("ThemeChoice", HostOnly),
        K::DiffText { .. } => ("DiffText", Atom),
        K::Prompt { .. } => ("Prompt", HostOnly),
        K::Timeline { .. } => ("Timeline", HostOnly),
        K::Receipt { .. } => ("Receipt", HostOnly),
        K::Disclosure { .. } => ("Disclosure", Atom),
        K::Badge { .. } => ("Badge", Atom),
        K::Markdown { .. } => ("Markdown", Atom),
        K::Notice { .. } => ("Notice", HostOnly),
        K::ChartData { .. } => ("ChartData", HostOnly),
        K::TableData { .. } => ("TableData", HostOnly),
    }
}

/// An example rooted at `root`, followed by `rest`.
fn example(
    variant: &'static str,
    note: &'static str,
    agent: bool,
    root: ComponentKind,
    rest: Vec<Component>,
    data: Value,
) -> Example {
    let (component, group) = kind_info(&root);
    let mut components = vec![Component::new("root", root)];
    components.extend(rest);
    Example { component, variant, note, group, agent, components, data }
}

#[cfg(test)]
#[path = "playbook_tests.rs"]
mod tests;
