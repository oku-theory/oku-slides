//! Cœur d'Oku Slides : modèle du deck, opérations, replica Loro,
//! validation et zip des sources. Aucune dépendance à Typst ni à Tauri.
//!
//! - [`Deck`] et son schéma JSON : slides, éléments, opérations ([`Op`],
//!   [`apply_all`]), validation ([`validate`]).
//! - [`DeckReplica`] : le deck dans un document Loro (CRDT), réconcilié par
//!   diff depuis un `Deck` et relu ; annuler et rétablir passent par son
//!   `UndoManager` ; [`ReplicaError`] pour ses échecs.
//! - [`SourceBundle`] : zip des sources (deck, assets) ; sa vue du deck est
//!   la relecture du replica, sous forme canonique ([`canonicalize`]).

mod canonical;
pub mod error;
pub mod locate;
pub mod model;
pub mod ops;
pub mod replica;
pub mod sources;
pub mod validate;

pub use canonical::canonicalize;
pub use error::{
    OpError, OpErrorKind, ReplicaError, ReplicaErrorKind, SourcesError, SourcesErrorKind, TransactionError,
};
pub use locate::{Located, Walk};
pub use model::*;
pub use ops::{Op, SlideField, apply_all, ops_schema};
pub use replica::{DeckRead, DeckReplica};
pub use sources::{SourceBundle, SourceLimits, asset_id};
pub use validate::{Issue, validate};
