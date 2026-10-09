//! Cœur d'Oku Slides : modèle du deck, opérations, replica Loro,
//! validation et zip des sources. Aucune dépendance à Typst ni à Tauri.
//!
//! - [`Deck`] et son schéma JSON : slides et éléments génériques
//!   (`{id, kind, params}`, [`Params`]), opérations ([`Op`],
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
//! - [`PackRef`], [`PackVersion`], [`PackFiles`], [`WidgetFile`] : pack de modèles d'un deck, version adressée par contenu, fichiers de widgets ; [`ThemeRef`] : thème d'un deck, versionné de même.
//! - [`Slide::text_fields`] : les champs texte d'une slide dans l'ordre de
//!   lecture ([`TextField`], [`FieldOwner`]), ordre de Tab de l'éditeur.
//! - [`is_hex_color`] : forme hexadécimale des couleurs des jetons.
//! - [`WidgetRegistry`] : schéma des widgets déclarés en Typst, lu depuis la sonde du rendu.

mod canonical;
mod color;
pub mod error;
mod fields;
pub mod locate;
pub mod model;
pub mod ops;
pub mod pack;
pub mod params;
pub mod replica;
pub mod sources;
#[cfg(test)]
pub(crate) mod test_registry;
pub mod validate;
pub mod widget;

pub use canonical::canonicalize;
pub use color::is_hex_color;
pub use error::{
    ListBound, OpError, OpErrorKind, PackVersionError, ReplicaError, ReplicaErrorKind, SchemaFault, SourcesError,
    SourcesErrorKind, TransactionError, ValueReason, WidgetError, WidgetErrorKind, WidgetFileError,
};
pub use fields::{FieldOwner, TextField};
pub use locate::{Located, MAX_SLOT_DEPTH, Walk, slot_depth};
pub use model::*;
pub use ops::{
    GRID_TRACKS, ITEM_ID_MAX, LIST_ITEMS_MAX, ListOp, ListSpec, Op, PLAIN_MAX, RICH_BYTES_MAX, RICH_PARAGRAPHS_MAX,
    SlideField, SlotRef, Target, apply_all, default_items, ops_schema, param_value, text_fits,
};
pub use pack::{COMMON_WIDGET_FILE, PackFiles, PackRef, PackVersion, ThemeRef, WidgetFile};
pub use params::{ListItem, ParamValue, Params};
pub use replica::{DeckRead, DeckReplica, HistoryPreview, MergeReport, SlideChanges, Version, slide_changes};
pub use sources::{SourceBundle, SourceLimits, asset_id};
pub use validate::{Issue, validate};
pub use widget::{Category, HostType, ParamSchema, ParamType, SlotMode, WidgetRegistry, WidgetSchema, is_text};
