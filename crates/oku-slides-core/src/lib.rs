//! Oku Slides core: deck model, ops, Loro replica,
//! validation and source zip. No dependency on Typst or Tauri.
//!
//! - [`Deck`] and its JSON schema: slides and generic elements
//!   (`{id, kind, params}`, [`Params`]), the [`HeadingLevel`] of a paragraph, ops ([`Op`],
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
//!   [`SourceBundle::merge`] adds the union of the assets, fonts and usage ledger to it.
//! - [`PackPin`], [`PackVersion`], [`PackFiles`], [`WidgetFile`]: widget pack pinned by a deck under a [`Namespace`], content-addressed version, widget files; [`ThemePin`]: theme pinned by a deck, versioned the same way, its `extends` chain in [`theme_chain`].
//! - [`token_spec`]: component tokens declared by packs ([`ComponentToken`]) and editor control ([`Control`]).
//! - [`Slide::text_fields`]: the text fields of a slide in reading
//!   order ([`TextField`], [`FieldOwner`]), the editor's Tab order.
//! - [`is_hex_color`]: hexadecimal form of token colors; [`color_names`]: the color names a deck uses.
//! - [`WidgetRegistry`]: schema of the widgets declared in Typst, read from the render probe (hosts: [`WidgetRegistry::host_params`]; block height: [`Grow`]; label and description: [`LocalizedText`]).
//! - [`Guides`], [`GuideKind`]: the agent's edit and use guides in a pack or theme manifest.
//! - [`Capabilities`]: what a pack or theme version holds, read from its files alone ([`CapabilityKind`]).
//! - [`PackKind`]: the kind of a pack (element `widgets`, slide `layouts`); [`LayoutProfile`]: preset of a layout for import.
//! - [`encode_link_url`]: a link destination written into Markdown.
//! - [`kinds`]: the kinds of the built-in `base` and `essentiel` packs, the only place they are written.
//! - [`media`]: actual type of a media file ([`media::sniff`]) and raster dimensions read from its header.
//! - [`num`]: bounded, named conversions between floats and integers ([`num::round_u32`]…).
//! - [`tokens`]: effective tokens of a theme ([`tokens::effective_tokens`], merge [`tokens::merge`], theme layer [`tokens::theme_layer`]).
//! - [`UsageLedger`]: the deck's agent usage, one [`UsageRecord`] per turn, add-only, checked on read; [`aggregate`] sums it into a [`UsageReport`].

mod canonical;
mod capabilities;
mod color;
mod data_bounds;
mod data_users;
pub mod error;
mod fields;
mod heading_level;
pub mod kinds;
pub mod knowledge;
mod layout_profile;
mod link;
mod localized_text;
pub mod locate;
pub mod media;
pub mod model;
mod namespace;
pub mod num;
pub mod ops;
pub mod pack;
mod pack_data;
mod pack_examples;
mod pack_guides;
mod pack_kind;
mod pack_order;
pub mod params;
mod prepare;
mod prepare_view;
pub mod replica;
mod shape_path;
pub mod sources;
mod sources_bases;
#[cfg(test)]
pub(crate) mod test_registry;
pub mod theme_chain;
pub mod token_spec;
pub mod tokens;
pub mod usage;
pub mod usage_report;
pub mod validate;
mod validate_data;
mod views;
pub mod widget;

pub use canonical::canonicalize;
pub use capabilities::{Capabilities, CapabilityKind};
pub use color::{color_names, is_hex_color};
pub use data_bounds::{DeckLoad, check_deck_bounds};
pub use data_users::ViewRef;
pub use error::{
    DataFaultKind, DeckJsonError, DeckJsonErrorKind, FrontmatterReason, HeadingLevelError, KnowledgeLimit,
    KnowledgeReason, ListBound, NamespaceError, OpError, OpErrorKind, PackManifestError, PackManifestErrorKind,
    PackVersionError, PathReason, ReplicaError, ReplicaErrorKind, SchemaFault, SourcesError, SourcesErrorKind,
    TokenEntryError, TokenEntryErrorKind, TransactionError, ValueReason, WidgetError, WidgetErrorKind, WidgetFileError,
};
pub use fields::{FieldOwner, TextField};
pub use heading_level::HeadingLevel;
pub use knowledge::{
    ASSETS_DIR, Frontmatter, KnowledgeFile, KnowledgePath, Provenance, check_text, cited_images, frontmatter,
    knowledge_file_id, knowledge_over_limits, normalize, stamp,
};
pub use layout_profile::{Arrangement, LayoutProfile, SlideRole};
pub use link::encode_link_url;
pub use localized_text::LocalizedText;
pub use locate::{Located, MAX_SLOT_DEPTH, Walk, slot_depth};
pub use model::*;
pub use namespace::{Namespace, qualify, split_kind};
pub use oku_slides_chart::ChartLabels;
pub use ops::{
    GRID_TRACKS, ITEM_ID_MAX, LIST_ITEMS_MAX, ListOp, ListPath, ListSpec, Moved, NESTED_ITEMS_MAX, Op, PLAIN_MAX,
    RICH_BYTES_MAX, RICH_PARAGRAPHS_MAX, SlotRef, Target, apply_all, apply_all_moving, bind_choices, default_items,
    ops_schema, pack_users, param_value, remove_pack_refusal, text_fits,
};
pub use pack::{
    COMMON_WIDGET_FILE, PackFiles, PackManifest, PackPin, PackScope, PackVersion, ThemeManifest, WidgetFile,
    now_created,
};
pub use pack_data::{MAX_PACK_DATA, check_pack_data};
pub use pack_guides::{GuideKind, Guides};
pub use pack_kind::PackKind;
pub use pack_order::{item_order, normalized_order};
pub use params::{ListItem, ParamValue, Params};
pub use prepare::{Origin, PrepareCtx, Prepared, check_external, fresh_ids, prepare_ops};
pub use prepare_view::{brings_inline, strip_inline};
pub use replica::{
    DeckRead, DeckReplica, HistoryPreview, KnowledgeSync, MergeReport, ReplicaLimits, SlideChanges, Version,
    slide_changes,
};
pub use shape_path::{PathCommand, SHAPE_PATH_COORD_MAX, SHAPE_PATH_MAX, ShapePath};
pub use sources::{SourceBundle, SourceLimits, asset_id};
pub use sources_bases::{BaseFiles, BaseFilesError};
pub use theme_chain::{ThemeChainError, ThemePin, check_closure, linearize};
pub use token_spec::{ComponentToken, Control, check_entries};
pub use usage::{
    Inserted, USAGE_FILE, USAGE_MAX_BYTES, USAGE_MAX_RECORDS, UsageCost, UsageLedger, UsageRecord, UsageTab,
    UsageTokens, shown_char, shown_text,
};
pub use usage_report::{
    By, CostTotal, Range, UsageRangeError, UsageRangeErrorKind, UsageReport, UsageRow, aggregate, parse_day,
};
pub use validate::{Issue, asset_refs, element_asset_refs, knowledge_asset_refs, validate};
pub use views::{MAX_RESOLVED_BYTES, MAX_RESOLVED_CELLS, PlanOutcome, ViewPlan, plan_views};
pub use widget::{
    Category, Grow, HostType, PARAM_NAME_MAX, ParamSchema, ParamType, SCHEMA_LIST_MAX, SlotMode,
    WIDGET_DESCRIPTION_MAX, WIDGET_LABEL_MAX, WidgetRegistry, WidgetSchema, is_text,
};
