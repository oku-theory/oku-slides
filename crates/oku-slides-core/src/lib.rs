//! Cœur d'Oku Slides : modèle du deck, opérations, replica Loro,
//! validation et zip des sources. Aucune dépendance à Typst ni à Tauri.
//!
//! - [`Deck`] et son schéma JSON : slides et éléments génériques
//!   (`{id, kind, params}`, [`Params`]), opérations ([`Op`],
//!   [`apply_all`]), validation ([`validate`], assets cités : [`asset_refs`],
//!   [`element_asset_refs`]).
//! - [`DeckReplica`] : le deck dans un document Loro (CRDT), réconcilié par
//!   diff depuis un `Deck` et relu ; annuler et rétablir passent par son
//!   `UndoManager` ; [`ReplicaError`] pour ses échecs ; [`ReplicaLimits`] à l'ouverture d'un
//!   fichier (taille, compaction des nœuds morts) ; [`DeckReplica::merge`]
//!   fusionne l'historique d'un autre replica et rend un [`MergeReport`] ;
//!   [`DeckReplica::history`] rend un [`HistoryPreview`] (ses [`Version`]s, la
//!   lecture d'une version) et [`DeckReplica::restore`] y ramène le deck
//!   (réconciliation avec la version relue, en un commit) ;
//!   [`slide_changes`] compare une version au deck courant, en [`SlideChanges`].
//! - [`SourceBundle`] : zip des sources (deck, assets) ; sa vue du deck est
//!   la relecture du replica, sous forme canonique ([`canonicalize`]) ;
//!   [`SourceBundle::merge`] y ajoute l'union des assets et des polices.
//! - [`PackPin`], [`PackVersion`], [`PackFiles`], [`WidgetFile`] : pack de widgets épinglé par un deck sous un [`Namespace`], version adressée par contenu, fichiers de widgets ; [`ThemePin`] : thème épinglé par un deck, versionné de même, sa chaîne d'`extends` dans [`theme_chain`].
//! - [`token_spec`] : jetons de composant déclarés par les packs ([`ComponentToken`]) et contrôle d'éditeur ([`Control`]).
//! - [`Slide::text_fields`] : les champs texte d'une slide dans l'ordre de
//!   lecture ([`TextField`], [`FieldOwner`]), ordre de Tab de l'éditeur.
//! - [`is_hex_color`] : forme hexadécimale des couleurs des jetons.
//! - [`WidgetRegistry`] : schéma des widgets déclarés en Typst, lu depuis la sonde du rendu (hôtes : [`WidgetRegistry::host_params`] ; hauteur d'un bloc : [`Grow`]).
//! - [`PackKind`] : la sorte d'un pack (`widgets` d'éléments, `layouts` de slides) ; [`LayoutProfile`] : préréglage d'un layout pour l'import.
//! - [`kinds`] : les kinds des packs intégrés `base` et `essentiel`, seul endroit où ils s'écrivent.
//! - [`media`] : type réel d'un média ([`media::sniff`]) et dimensions d'un raster lues dans son en-tête.
//! - [`tokens`] : jetons effectifs d'un thème ([`tokens::effective_tokens`], fusion [`tokens::merge`], couche de thème [`tokens::theme_layer`]).

mod canonical;
mod color;
pub mod error;
mod fields;
pub mod kinds;
mod layout_profile;
pub mod locate;
pub mod media;
pub mod model;
mod namespace;
pub mod ops;
pub mod pack;
mod pack_examples;
mod pack_kind;
pub mod params;
pub mod replica;
mod shape_path;
pub mod sources;
#[cfg(test)]
pub(crate) mod test_registry;
pub mod theme_chain;
pub mod token_spec;
pub mod tokens;
pub mod validate;
pub mod widget;

pub use canonical::canonicalize;
pub use color::is_hex_color;
pub use error::{
    ListBound, NamespaceError, OpError, OpErrorKind, PackManifestError, PackManifestErrorKind, PackVersionError,
    ReplicaError, ReplicaErrorKind, SchemaFault, SourcesError, SourcesErrorKind, TokenEntryError, TokenEntryErrorKind,
    TransactionError, ValueReason, WidgetError, WidgetErrorKind, WidgetFileError,
};
pub use fields::{FieldOwner, TextField};
pub use layout_profile::{Arrangement, LayoutProfile, SlideRole};
pub use locate::{Located, MAX_SLOT_DEPTH, Walk, slot_depth};
pub use model::*;
pub use namespace::{Namespace, qualify, split_kind};
pub use ops::{
    GRID_TRACKS, ITEM_ID_MAX, LIST_ITEMS_MAX, ListOp, ListPath, ListSpec, Moved, NESTED_ITEMS_MAX, Op, PLAIN_MAX,
    RICH_BYTES_MAX, RICH_PARAGRAPHS_MAX, SlotRef, Target, apply_all, apply_all_moving, default_items, ops_schema,
    pack_users, param_value, remove_pack_refusal, text_fits,
};
pub use pack::{
    COMMON_WIDGET_FILE, PackFiles, PackManifest, PackPin, PackScope, PackVersion, ThemeManifest, WidgetFile,
    now_created,
};
pub use pack_kind::PackKind;
pub use params::{ListItem, ParamValue, Params};
pub use replica::{
    DeckRead, DeckReplica, HistoryPreview, MergeReport, ReplicaLimits, SlideChanges, Version, dataset_row_budget,
    slide_changes,
};
pub use shape_path::{PathCommand, SHAPE_PATH_COORD_MAX, SHAPE_PATH_MAX, ShapePath};
pub use sources::{SourceBundle, SourceLimits, asset_id};
pub use theme_chain::{ThemeChainError, ThemePin, check_closure, linearize};
pub use token_spec::{ComponentToken, Control, check_entries};
pub use validate::{Issue, asset_refs, element_asset_refs, validate};
pub use widget::{Category, Grow, HostType, ParamSchema, ParamType, SlotMode, WidgetRegistry, WidgetSchema, is_text};
