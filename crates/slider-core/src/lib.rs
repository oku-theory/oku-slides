//! Cœur d'Oku Slides : modèle du deck, opérations réversibles, historique,
//! validation et zip des sources. Aucune dépendance à Typst ni à Tauri.

pub mod error;
pub mod history;
pub mod locate;
pub mod model;
pub mod ops;
pub mod sources;
pub mod validate;

pub use error::{OpError, OpErrorKind, SourcesError, SourcesErrorKind, TransactionError};
pub use history::History;
pub use locate::{Located, Walk};
pub use model::*;
pub use ops::{Op, SlideField, apply_all, ops_schema};
pub use sources::{SourceBundle, asset_id};
pub use validate::{Issue, validate};
