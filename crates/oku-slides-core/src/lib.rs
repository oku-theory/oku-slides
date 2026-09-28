//! Cœur d'Oku Slides : modèle du deck, opérations, replica Loro,
//! validation et zip des sources. Aucune dépendance à Typst ni à Tauri.
//!
//! - [`Deck`] et son schéma JSON : slides, éléments, opérations ([`Op`],
//!   [`apply_all`]), validation ([`validate`]).
//! - [`DeckReplica`] : le deck dans un document Loro (CRDT), réconcilié par
//!   diff depuis un `Deck` et relu ; annuler et rétablir passent par son
//!   `UndoManager` ; [`ReplicaError`] pour ses échecs ; [`DeckReplica::merge`]
//!   fusionne l'historique d'un autre replica et rend un [`MergeReport`] ;
//!   [`DeckReplica::history`] rend un [`HistoryPreview`] (ses [`Version`]s, la
//!   lecture d'une version) et [`DeckReplica::restore`] y ramène le deck
//!   (réconciliation avec la version relue, en un commit) ;
//!   [`slide_changes`] compare une version au deck courant, en [`SlideChanges`].
//! - [`SourceBundle`] : zip des sources (deck, assets) ; sa vue du deck est
//!   la relecture du replica, sous forme canonique ([`canonicalize`]) ;
//!   [`SourceBundle::merge`] y ajoute l'union des assets et des polices.
//! - [`WidgetRegistry`] : schéma des widgets déclarés en Typst, lu depuis la sonde du rendu.

mod canonical;
pub mod error;
pub mod locate;
pub mod model;
pub mod ops;
pub mod replica;
pub mod sources;
pub mod validate;
pub mod widget;

pub use canonical::canonicalize;
pub use error::{
    OpError, OpErrorKind, ReplicaError, ReplicaErrorKind, SchemaFault, SourcesError, SourcesErrorKind,
    TransactionError, WidgetError, WidgetErrorKind,
};
pub use locate::{Located, Walk};
pub use model::*;
pub use ops::{Op, SlideField, apply_all, ops_schema};
pub use replica::{DeckRead, DeckReplica, HistoryPreview, MergeReport, SlideChanges, Version, slide_changes};
pub use sources::{SourceBundle, SourceLimits, asset_id};
pub use validate::{Issue, validate};
pub use widget::{Category, HostType, ParamSchema, ParamType, SlotMode, WidgetRegistry, WidgetSchema};
