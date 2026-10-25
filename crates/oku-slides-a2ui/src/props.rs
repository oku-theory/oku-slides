//! Component property types: tones, steps, options, text chunks.

use serde::{Deserialize, Serialize};

/// Tone of a card or a badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    /// No tone.
    Neutral,
    /// Accent tone (receipts).
    Accent,
    /// Red rule: reserved for the host in `render`.
    Danger,
}

/// Status of a step in a turn's timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Not started.
    Pending,
    /// In progress.
    InProgress,
    /// Finished.
    Completed,
    /// Failed: stays visible in a grouped-tools step.
    Failed,
}

/// An image pasted with the prompt, shown as a chip.
///
/// It carries its type and size, never its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptImage {
    /// Its actual type (`image/png`…).
    pub mime: String,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
}

/// A timeline step: grouped tools, an agent note or a plan entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineStep {
    /// Title of the step's last tool, the agent note or the plan entry.
    pub label: String,
    /// Aggregated status of the step.
    pub status: StepStatus,
    /// Number of tools in the step; absent for a note or a plan entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// The part of a note that `label` does not show.
    ///
    /// That is its rest, or its whole text if cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The calls of a tools step, in order.
    ///
    /// The oldest ones of a long turn drop out: they only count in `count`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<ToolCallDetail>,
}

impl TimelineStep {
    /// A step with no count, detail or calls.
    pub fn new(label: impl Into<String>, status: StepStatus) -> Self {
        Self { label: label.into(), status, count: None, detail: None, calls: Vec::new() }
    }
}

/// A tool call of a step: its arguments and its result.
///
/// The texts are already bounded and filtered by the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCallDetail {
    /// Readable tool name.
    pub label: String,
    /// Last status of the call.
    pub status: StepStatus,
    /// Arguments, as indented JSON.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    /// Result: text returned by the tool, otherwise its raw output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

/// Text style (a subset of the v0.9 variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextVariant {
    /// A section heading.
    H4,
    /// Body text.
    Body,
    /// A caption, smaller.
    Caption,
    /// A dimmed line, truncated.
    Note,
}

/// Button pictogram that replaces its label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ButtonIcon {
    /// A cross: close.
    Close,
    /// An up arrow: move up.
    Up,
    /// A down arrow: move down.
    Down,
    /// A minus sign: remove.
    Remove,
}

/// An option of a choice or a list.
///
/// Also an argument of the agent's tools (`ask_choice`, `select` fields):
/// deserializable, with a schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChoiceOption {
    /// Identifier written into the value when the option is chosen.
    pub id: String,
    /// Label shown.
    pub label: String,
    /// Optional detail, shown with the label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// A slide as rendered at one point: its identifier and its page fingerprint.
/// The UI resolves its thumbnail (its cache is keyed by fingerprint): the message
/// never carries the render itself, so its size does not depend on the slide content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlideRef {
    /// Slide identifier.
    pub id: String,
    /// Page fingerprint (32 hex digits, like `SlideSummary.hash`).
    pub hash: String,
}

/// A chunk of text kept, removed or added.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffSegment {
    /// Kept, removed or added.
    pub kind: DiffKind,
    /// The chunk's text.
    pub text: String,
}

/// Fate of a text chunk in a [`DiffSegment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    /// Unchanged.
    Keep,
    /// Removed.
    Del,
    /// Added.
    Ins,
}

#[cfg(test)]
#[path = "props_tests.rs"]
mod tests;
