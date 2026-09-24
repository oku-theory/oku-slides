//! Cœur de Slider : modèle du deck, opérations réversibles, historique,
//! validation et zip des sources. Aucune dépendance à Typst ni à Tauri.

pub mod history;
pub mod model;
pub mod ops;
pub mod sources;
pub mod validate;

pub use history::{History, Transaction};
pub use model::*;
pub use ops::{Op, OpError, SlideField, Zone, apply_all, element_ids, ops_schema};
pub use sources::{SourceBundle, SourcesError, asset_id};
pub use validate::{Issue, validate};
