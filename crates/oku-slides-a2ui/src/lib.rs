//! Surfaces A2UI d'Oku Slides : les messages v0.9 que la session envoie à l'interface.
//!
//! - [`ServerMessage`] : `createSurface`, `updateComponents`,
//!   `updateDataModel`, `deleteSurface`, au format v0.9.
//! - [`Component`] / [`ComponentKind`] : le catalogue [`CATALOG_ID`] (Column,
//!   Row, Card, Text, TextField, RichTextField, Stepper, Button, ChoiceChips,
//!   Slider, ColorField, Select, ListTable, SlideThumb, ThemeChoice, DiffText, Prompt,
//!   Timeline, Receipt, Disclosure, Badge, Markdown, Notice, ChartData, TableData), à plat, relié par
//!   identifiants.
//! - [`SeriesSwatch`] : une série colorable d'un `ChartData`.
//! - [`TimelineStep`] / [`ToolCallDetail`] : les étapes d'un tour et le détail de leurs appels d'outils.
//! - [`Dynamic`] : une valeur littérale ou un chemin (JSON Pointer) dans le
//!   modèle de données de la surface.
//! - [`Action`] : l'évènement qu'un composant envoie ; l'interface le traite.
//!
//! Ce crate ne fait que produire : l'interface (`@a2ui/web_core`) valide les
//! messages contre le catalogue, décrit en zod dans `ui/src/a2ui`.
//! Spécification : <https://a2ui.org/specification/v0.9-a2ui/>.

mod data;
mod message;

pub use data::SeriesSwatch;

pub use message::{
    Action, ButtonIcon, CATALOG_ID, ChoiceOption, ColumnKind, Component, ComponentKind, CreateSurface, DeleteSurface,
    DiffKind, DiffSegment, Dynamic, Event, ListColumn, PromptImage, ServerBody, ServerMessage, SlideRef, StepStatus,
    SubColumn, SubList, SubListFault, TextVariant, TimelineStep, Tone, ToolCallDetail, UpdateComponents,
    UpdateDataModel, Version,
};
