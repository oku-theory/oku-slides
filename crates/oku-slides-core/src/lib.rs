//! Oku Slides core: deck model, ops, Loro replica,
//! validation and source zip. No dependency on Typst or Tauri.
//!
//! - [`Deck`] and its JSON schema: slides and generic elements
//!   (`{id, kind, params}`, [`Params`]), ops ([`Op`],
//!   [`apply_all`]), validation ([`validate`], cited assets: [`asset_refs`],
//!   [`element_asset_refs`]).
//! - [`DeckReplica`]: the deck in a Loro document (CRDT), reconciled by
//!   diff from a `Deck` and read back; undo and redo go through its
//!   `UndoManager`; [`ReplicaError`] for its failures; [`ReplicaLimits`] when opening a
//!   file (size, compaction of dead nodes); [`DeckReplica::merge`]
//!   merges the history of another replica and returns a [`MergeReport`];
//!   [`DeckReplica::history`] returns a [`HistoryPreview`] (its [`Version`]s, the
//!   reading of a version) and [`DeckReplica::restore`] brings the deck back to one
//!   (reconciliation with the version read back, in one commit);
//!   [`slide_changes`] compares a version with the current deck, as [`SlideChanges`].
//! - [`SourceBundle`]: zip of the sources (deck, assets); its view of the deck is
//!   the replica read back, in canonical form ([`canonicalize`]);
//!   [`SourceBundle::merge`] adds the union of the assets and fonts to it.
//! - [`PackPin`], [`PackVersion`], [`PackFiles`], [`WidgetFile`]: widget pack pinned by a deck under a [`Namespace`], content-addressed version, widget files; [`ThemePin`]: theme pinned by a deck, versioned the same way, its `extends` chain in [`theme_chain`].
//! - [`token_spec`]: component tokens declared by packs ([`ComponentToken`]) and editor control ([`Control`]).
//! - [`Slide::text_fields`]: the text fields of a slide in reading
//!   order ([`TextField`], [`FieldOwner`]), the editor's Tab order.
//! - [`is_hex_color`]: hexadecimal form of token colors.
//! - [`WidgetRegistry`]: schema of the widgets declared in Typst, read from the render probe (hosts: [`WidgetRegistry::host_params`]; block height: [`Grow`]).
//! - [`PackKind`]: the kind of a pack (element `widgets`, slide `layouts`); [`LayoutProfile`]: preset of a layout for import.
//! - [`kinds`]: the kinds of the built-in `base` and `essentiel` packs, the only place they are written.
//! - [`media`]: actual type of a media file ([`media::sniff`]) and raster dimensions read from its header.
//! - [`num`]: bounded, named conversions between floats and integers ([`num::round_u32`]…).
//! - [`tokens`]: effective tokens of a theme ([`tokens::effective_tokens`], merge [`tokens::merge`], theme layer [`tokens::theme_layer`]).

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
pub mod num;
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
