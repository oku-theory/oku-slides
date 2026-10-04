//! Surfaces A2UI d'Oku Slides : les messages v0.9 que la session envoie à l'interface.
//!
//! - [`ServerMessage`] : `createSurface`, `updateComponents`,
//!   `updateDataModel`, `deleteSurface`, au format v0.9.
//! - [`Component`] / [`ComponentKind`] : le catalogue [`CATALOG_ID`] (Column,
//!   Row, Card, Text, TextField, RichTextField, Stepper, Button, ChoiceChips, Slider,
//!   ColorField, Select, SlideThumb, DiffText, Prompt, Timeline, Receipt, Disclosure, Badge, Markdown), à plat, relié par identifiants.
//! - [`Dynamic`] : une valeur littérale ou un chemin (JSON Pointer) dans le
//!   modèle de données de la surface.
//! - [`Action`] : l'évènement qu'un composant envoie ; l'interface le traite.
//!
//! Ce crate ne fait que produire : l'interface (`@a2ui/web_core`) valide les
//! messages contre le catalogue, décrit en zod dans `ui/src/a2ui`.
//! Spécification : <https://a2ui.org/specification/v0.9-a2ui/>.

mod message;

pub use message::{
    Action, CATALOG_ID, ChoiceOption, Component, ComponentKind, CreateSurface, DeleteSurface, DiffKind, DiffSegment,
    Dynamic, Event, ServerBody, ServerMessage, SlideRef, StepStatus, TextVariant, TimelineStep, Tone, UpdateComponents,
    UpdateDataModel, Version,
};
