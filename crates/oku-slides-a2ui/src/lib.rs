//! Oku Slides A2UI surfaces: the v0.9 messages the session sends to the UI.
//!
//! - [`ServerMessage`]: `createSurface`, `updateComponents`,
//!   `updateDataModel`, `deleteSurface`, in v0.9 format.
//! - [`Component`] / [`ComponentKind`]: the [`CATALOG_ID`] catalog (Column,
//!   Row, Card, Text, `TextField`, `RichTextField`, Stepper, Button, `ChoiceChips`,
//!   Slider, `ColorField`, Select, `ListTable`, `SlideThumb`, `ThemeChoice`, `DiffText`, Prompt,
//!   Timeline, Receipt, Disclosure, Badge, Markdown, Notice, `ChartData`, `TableData`), flat, linked by
//!   identifiers.
//! - [`SeriesSwatch`]: a colorable series of a `ChartData`.
//! - [`TimelineStep`] / [`ToolCallDetail`]: the steps of a turn and the detail of their tool calls.
//! - [`Dynamic`]: a literal value or a path (JSON Pointer) into the
//!   surface's data model.
//! - [`Action`]: the event a component sends; the UI handles it.
//!
//! This crate only produces: the UI (`@a2ui/web_core`) validates the
//! messages against the catalog, described in zod in `ui/src/a2ui`.
//! Specification: <https://a2ui.org/specification/v0.9-a2ui/>.

mod action;
mod component;
mod data;
mod list;
mod message;
mod props;

pub use action::{Action, Dynamic, Event};
pub use component::{Component, ComponentKind};
pub use data::SeriesSwatch;
pub use list::{ColumnKind, ListColumn, SubColumn, SubList, SubListFault};
pub use message::{
    CATALOG_ID, CreateSurface, DeleteSurface, ServerBody, ServerMessage, UpdateComponents, UpdateDataModel, Version,
};
pub use props::{
    ButtonIcon, ChoiceOption, DiffKind, DiffSegment, PromptImage, SlideRef, StepStatus, TextVariant, TimelineStep,
    Tone, ToolCallDetail,
};
