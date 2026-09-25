//! Cœur d'Écho : modèle du deck, opérations réversibles, historique,
//! validation et zip des sources. Aucune dépendance à Typst ni à Tauri.

pub mod history;
pub mod locate;
pub mod model;
pub mod ops;
pub mod sources;
pub mod validate;

pub use history::History;
pub use locate::{Located, Walk};
pub use model::*;
pub use ops::{Op, OpError, SlideField, apply_all, ops_schema};
pub use sources::{SourceBundle, SourcesError, asset_id};
pub use validate::{Issue, validate};
