//! Modal questions: confirmations and permissions, with the line of what was heard.
//!
//! Split out of `forms`, which re-exports them.

use oku_slides_a2ui::{Action, Component, TextVariant};
use oku_slides_i18n::Localizer;
use serde_json::json;

use super::forms::{Place, Surface, lit, question, surface};

/// The id of the line under the buttons: what was heard, or how to answer.
pub const HEARD: &str = "heard";

/// A confirmation modal: confirm or cancel.
///
/// `danger_label`, when given, is the danger button's text: the verb the user must say.
/// `line`, when given, shows under the buttons as plain text (what was heard, or how to
/// answer); the caller clips and cleans heard text first.
pub fn confirm(id: &str, message: &str, danger_label: Option<String>, line: Option<&str>, l: &Localizer) -> Surface {
    let danger = danger_label.is_some();
    let yes = danger_label.unwrap_or_else(|| l.tr("agent-surface-confirm"));
    let answer = |v: bool| Action::event("confirm", [("confirmed", lit(json!(v)))]);
    let mut body = vec!["message", "buttons"];
    body.extend(line.map(|_| HEARD));
    let mut components = vec![
        Component::card("root", "body"),
        Component::column("body", &body),
        Component::text("message", message.to_owned(), Some(TextVariant::Body)),
        Component::row("buttons", &["yes", "no"]),
        Component::button_builder("yes", yes, answer(true)).primary(true).danger(danger).build(),
        Component::button("no", l.tr("agent-surface-cancel"), false, answer(false)),
    ];
    if let Some(line) = line {
        components.push(Component::text(HEARD, line, Some(TextVariant::Caption)));
    }
    question(surface(id, Place::Modal, None, components, json!({})))
}

/// The kind of a permission option (ACP `PermissionOptionKind`).
///
/// Only "allow once" can be the primary button. "Always allow" and an unknown kind are marked
/// as committing; refusing, even forever, grants nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    /// Allow this time only.
    AllowOnce,
    /// Allow and remember it.
    AllowAlways,
    /// Refuse this time.
    RejectOnce,
    /// Refuse and remember it.
    RejectAlways,
    /// A kind this version does not know: treated as committing (it could allow for good).
    /// Revisit when the ACP schema changes.
    Other,
}

impl PermissionKind {
    /// Display rank: the safest first, the most committing last (Tab never lands first on
    /// "always allow").
    fn rank(self) -> u8 {
        match self {
            Self::AllowOnce => 0,
            Self::RejectOnce => 1,
            Self::RejectAlways => 2,
            Self::Other => 3,
            Self::AllowAlways => 4,
        }
    }

    fn engages(self) -> bool {
        matches!(self, Self::AllowAlways | Self::Other)
    }
}

/// A permission option, as the agent offers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionChoice {
    /// Label, written by the agent.
    pub name: String,
    /// ACP kind: it alone decides the order, the primary button and the style.
    pub kind: PermissionKind,
}

/// An ACP permission asked by the agent: one button per option (Escape: `dismiss`).
///
/// The labels come from the agent; nothing else does. Buttons run from the safest to the most
/// committing; the primary is the first "allow once" option (none without one); "always allow"
/// is marked committing and gets a mention the app translates ("forever"), so a misleading
/// agent label shows. Each button returns the option's index in the agent's order.
///
/// `note`, when given, shows under the title (the received guides the turn carries); `line`
/// under the buttons (what was heard, or how to answer).
pub fn permission(
    id: &str,
    title: &str,
    options: &[PermissionChoice],
    note: Option<&str>,
    line: Option<&str>,
    l: &Localizer,
) -> Surface {
    let mut order: Vec<usize> = (0..options.len()).collect();
    order.sort_by_key(|&i| options[i].kind.rank());
    let primary = options.iter().position(|o| o.kind == PermissionKind::AllowOnce);
    let ids: Vec<String> = order.iter().map(|i| format!("option-{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let title = if title.trim().is_empty() { l.tr("agent-surface-permission-title") } else { title.to_owned() };
    let mut body = vec!["title"];
    body.extend(note.map(|_| "note"));
    body.push("buttons");
    body.extend(line.map(|_| HEARD));
    let mut components = vec![
        Component::card("root", "body"),
        Component::column("body", &body),
        Component::text("title", title, Some(TextVariant::H4)),
        Component::row("buttons", &refs),
    ];
    if let Some(note) = note {
        components.push(Component::text("note", note, None));
    }
    if let Some(line) = line {
        components.push(Component::text(HEARD, line, Some(TextVariant::Caption)));
    }
    for (&i, bid) in order.iter().zip(&ids) {
        let option = &options[i];
        let label = if option.kind == PermissionKind::AllowAlways {
            format!("{} · {}", option.name, l.tr("agent-surface-permission-always"))
        } else {
            option.name.clone()
        };
        let action = Action::event("permission", [("index", lit(json!(i)))]);
        let button = Component::button_builder(bid, label, action).primary(primary == Some(i));
        components.push(button.danger(option.kind.engages()).build());
    }
    question(surface(id, Place::Modal, None, components, json!({})))
}
