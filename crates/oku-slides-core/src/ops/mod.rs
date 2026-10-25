//! Opérations sur le deck. Toute modification, qu'elle vienne de la souris ou
//! d'une dictée, passe par une `Op`.
//!
//! Les ops adressent une slide ou un élément par [`Target`], un slot par
//! [`SlotRef`], et contrôlent chaque paramètre par le registre des widgets
//! ([`WidgetRegistry`]) : paramètre déclaré, valeur du bon type
//! ([`param_value`]), slot qui accepte l'élément.

mod check;
mod edit;
mod grid;
mod host;
mod list;
mod list_path;
mod moving;
mod packs;
mod value;

use std::collections::{BTreeMap, HashSet};

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{IntoError, OptionExt, ResultExt};

use oku_slides_chart::DataOp;

use crate::error::{
    DuplicateIdSnafu, ElementNotFoundSnafu, InvalidValueSnafu, OpError, ParseValueSnafu, SlideNotFoundSnafu,
    TransactionError, TransactionSnafu, UnknownParamSnafu, UnknownWidgetSnafu, ValueReason, WrongCategorySnafu,
};
use crate::locate::{Located, MAX_SLOT_DEPTH, Walk, slot_depth};
use crate::model::{Cell, Deck, Element, Frame, Slide, Style};
use crate::namespace::Namespace;
use crate::pack::PackPin;
use crate::pack_kind::PackKind;
use crate::params::Params;
use crate::theme_chain::{ThemePin, check_closure};
use crate::widget::{Category, HostType, ParamSchema, ParamType, SlotMode, WidgetRegistry, WidgetSchema};

use check::{check_accepts, check_params};
use edit::{
    add_element, add_slide, move_slide, replace_element, set_cell, set_deck_param, set_frame, set_kind, set_param,
    set_tokens, unset_token,
};
pub(crate) use host::{host_option, normalize_hosts};
pub(crate) use list::{DefaultFault, default_fault};
pub use list::{
    ITEM_ID_MAX, LIST_ITEMS_MAX, ListOp, ListSpec, NESTED_ITEMS_MAX, PLAIN_MAX, RICH_BYTES_MAX, RICH_PARAGRAPHS_MAX,
    default_items, text_fits,
};
pub use list_path::ListPath;
pub use moving::Moved;
pub use packs::{pack_users, remove_pack_refusal};
pub use value::{GRID_TRACKS, param_value};

/// A slide, or an element of a slide.
///
/// JSON: `{"slide": "s1"}` or `{"slide": "s1", "element": "e1"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Target {
    /// Id of the target slide, or of the slide that contains the element.
    pub slide: String,
    /// Id of the element, at any slot depth; absent: the slide itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
}

/// A slot: the `slot` parameter `param` of the slide or element `owner`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SlotRef {
    /// Slide or element that declares the slot.
    pub owner: Target,
    /// Name of the parameter of type `slot`.
    pub param: String,
}

/// An operation on the deck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Sets a parameter of a slide or element, checked by its widget's schema.
    ///
    /// `null` (or an empty text) removes it. A bound parameter (`bind`) writes
    /// into the deck parameters. The elements of a slot go through `add_element`.
    SetParam {
        /// Slide or element whose parameter changes.
        target: Target,
        /// Name of the parameter, declared by the widget's schema.
        param: String,
        /// New value, of the parameter's type; `null` removes it.
        value: Value,
    },
    /// Sets a deck parameter (`title`, `author`, `numbering`…); `null` removes it.
    SetDeckParam {
        /// Name of the deck parameter.
        key: String,
        /// New value; `null` removes it.
        value: Value,
    },
    /// Changes the widget of a slide or element, within the same category.
    ///
    /// The parameters are kept as they are.
    SetKind {
        /// Slide or element whose widget changes.
        target: Target,
        /// New widget, qualified by its pack, of the same category.
        kind: String,
    },
    /// Merges style properties; absent fields do not change.
    SetStyle {
        /// Slide that contains the element.
        slide: String,
        /// Element whose style is merged.
        element: String,
        /// Properties to merge; absent ones stay unchanged.
        style: Style,
    },
    /// Replaces the whole style of an element.
    ReplaceStyle {
        /// Slide that contains the element.
        slide: String,
        /// Element whose style is replaced.
        element: String,
        /// New complete style.
        style: Style,
    },
    /// Moves or resizes an element; `null` hands it back to the layout.
    SetFrame {
        /// Slide that contains the element.
        slide: String,
        /// Element to move or resize.
        element: String,
        /// Frame of positive width and height; `null` hands the element back to the layout.
        frame: Option<Frame>,
    },
    /// Attaches an element of a free slot to the grid; `null` detaches it.
    ///
    /// Refused in a `flow` slot: remove the element from it, then add it to a free slot.
    SetCell {
        /// Slide that contains the element.
        slide: String,
        /// Element to attach to the grid.
        element: String,
        /// Cell in the slide's grid; `null` detaches the element.
        cell: Option<Cell>,
    },
    /// Changes the grid of a slide and reframes the elements of its free slots.
    SetGrid {
        /// Slide whose grid changes.
        slide: String,
        /// Columns, from 1 to 24; absent: unchanged.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<u32>,
        /// Rows, from 1 to 24; absent: unchanged.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rows: Option<u32>,
    },
    /// Replaces a whole element, with the same `id`.
    ReplaceElement {
        /// Slide that contains the element.
        slide: String,
        /// Complete element that replaces the one with the same `id`.
        element: Element,
    },
    /// Changes the options of a chart (`param`, of type `host(chart)`).
    ///
    /// JSON Merge Patch (RFC 7396) on everything but `dataset`: `null` removes a
    /// field, an array replaces the whole array.
    SetChartOption {
        /// Slide or element that holds the chart.
        target: Target,
        /// Parameter of type `host(chart)`.
        param: String,
        /// JSON Merge Patch of the options, without `dataset`.
        patch: Value,
    },
    /// Changes the display options of a table (`host(table)` parameter).
    ///
    /// JSON Merge Patch on `header-row`, `header-column`, `striped`, `align`; never `dataset`.
    SetTableOption {
        /// Slide or element that holds the table.
        target: Target,
        /// Parameter of type `host(table)`.
        param: String,
        /// JSON Merge Patch of the display options, without `dataset`.
        patch: Value,
    },
    /// Changes the data of a chart or table.
    ///
    /// `dataset.source`, row 0 = column names; `param` is of type `host`.
    SetData {
        /// Slide or element that holds the chart or table.
        target: Target,
        /// Parameter of type `host`.
        param: String,
        /// Change to the data (rows, columns, cells).
        change: DataOp,
    },
    /// Changes a `list` parameter, with items addressed by their id.
    ///
    /// `param`: the parameter, or `param/item/column` for the nested list of an
    /// item. To replace the whole list, use `set_param`.
    EditList {
        /// Slide or element that holds the list.
        target: Target,
        /// `list` parameter, or `param/item/column` for a nested list.
        param: String,
        /// Change to the items, addressed by their id.
        change: ListOp,
    },
    /// Adds an element to a slot of a slide or element.
    AddElement {
        /// Slot that receives the element.
        slot: SlotRef,
        /// Position in the slot; absent = at the end.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        /// Seeds absent lists with the schema's default.
        ///
        /// For a new object, not a paste or a move.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        defaults: bool,
        /// Element to add, with ids free in the slide.
        element: Element,
    },
    /// Removes an element and all its children, at any slot depth.
    RemoveElement {
        /// Slide that contains the element.
        slide: String,
        /// Element to remove, with its children.
        element: String,
    },
    /// Moves an element, with its children, to a slot of a slide or element.
    ///
    /// The same slide is allowed. Entering a `flow` slot removes `frame` and `cell`.
    MoveElement {
        /// Slide that contains the element.
        slide: String,
        /// Element to move.
        element: String,
        /// Target slot, in the same slide or another one.
        to: SlotRef,
        /// Position in the target slot; absent = at the end.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
    },
    /// Adds a slide, with the elements of its slots.
    AddSlide {
        /// Position in the deck; absent = at the end.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        /// Seeds absent lists with the schema's default.
        ///
        /// For a new object, not a paste or a move.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        defaults: bool,
        /// Slide to add, with a free id.
        slide: Slide,
    },
    /// Removes a slide and all its elements.
    RemoveSlide {
        /// Slide to remove.
        slide: String,
    },
    /// Moves a slide to another position in the deck.
    MoveSlide {
        /// Slide to move.
        slide: String,
        /// Final position, within the deck's bounds.
        index: usize,
    },
    /// Replaces the speaker notes of a slide.
    SetNotes {
        /// Slide whose notes are replaced.
        slide: String,
        /// Speaker notes, which replace the previous ones; empty: no notes.
        notes: String,
    },
    /// Theme of the deck and its pins; `null`: the default theme, with no pins.
    ///
    /// `themes` must be exactly the closure of the `extends` of `theme`:
    /// otherwise refused (`ThemeCycle`, `ThemeClosure`). The session materializes the
    /// closure from the library.
    SetTheme {
        /// Theme of the deck; `null`: the default theme.
        theme: Option<Namespace>,
        /// Pins of the closure of the `extends` of `theme`.
        #[serde(default)]
        themes: BTreeMap<Namespace, ThemePin>,
    },
    /// Pins version `pin` of pack `namespace` of kind `kind` (added or replaced).
    ///
    /// Every namespace in `pin.requires` must already be pinned among the
    /// widget packs: the session adds the requirements first, in the same
    /// transaction. A namespace pinned under the other kind is refused.
    AddPack {
        /// Kind of the pack: widgets or layouts.
        kind: PackKind,
        /// Namespace of the pack in the deck.
        namespace: Namespace,
        /// Pinned version, with its requirements.
        pin: PackPin,
    },
    /// Removes pack `namespace` of kind `kind`, if neither required nor used.
    ///
    /// The `base` widget pack is never removed.
    RemovePack {
        /// Kind of the pack: widgets or layouts.
        kind: PackKind,
        /// Namespace of the pack to remove.
        namespace: Namespace,
    },
    /// Sets the design token at the given path (`["color", "accent"]`).
    ///
    /// `null` is set as is: it "empties" a key that accepts it.
    SetToken {
        /// Path of the token, not empty.
        path: Vec<String>,
        /// Value set as is, `null` included.
        value: Value,
    },
    /// Removes the token at the given path, falling back to the layer below.
    ///
    /// An object left empty is removed too.
    UnsetToken {
        /// Path of the token to remove, not empty.
        path: Vec<String>,
    },
    /// Replaces all the deck's tokens: an object, or `null` (none).
    SetTokens {
        /// Object of the deck's tokens, or `null`.
        tokens: Value,
    },
    /// Sets a deck variable; `null` removes it.
    SetVar {
        /// Name of the variable.
        key: String,
        /// New value; `null` removes it.
        value: Value,
    },
}

// ---- Localisation ------------------------------------------------------------------

/// Élément `id` d'une liste d'éléments, en descendant dans tous les slots.
/// Même ordre que [`Walk`] (parents avant enfants, slots par nom). La
/// profondeur des slots est bornée ([`MAX_SLOT_DEPTH`]) : récursion sûre.
fn find_mut<'a>(list: &'a mut [Element], id: &str) -> Option<&'a mut Element> {
    for e in list.iter_mut() {
        if e.id == id {
            return Some(e);
        }
        for (_, children) in e.params.slots_mut() {
            if let Some(found) = find_mut(children, id) {
                return Some(found);
            }
        }
    }
    None
}

fn element_mut<'a>(slide: &'a mut Slide, id: &str) -> Option<&'a mut Element> {
    for (_, list) in slide.params.slots_mut() {
        if let Some(e) = find_mut(list, id) {
            return Some(e);
        }
    }
    None
}

/// Retire le premier élément `id` (ordre de [`Walk`]) d'une liste ou de ses slots.
fn remove_from(list: &mut Vec<Element>, id: &str) -> Option<Element> {
    let mut found = None;
    for (i, e) in list.iter_mut().enumerate() {
        if e.id == id {
            found = Some(i);
            break;
        }
        for (_, children) in e.params.slots_mut() {
            if let Some(removed) = remove_from(children, id) {
                return Some(removed);
            }
        }
    }
    // `i` vient de l'énumération de `list` : dans les bornes.
    found.map(|i| list.remove(i))
}

fn slide_mut<'a>(deck: &'a mut Deck, id: &str) -> Result<&'a mut Slide, OpError> {
    Ok(deck.slide_mut(id).context(SlideNotFoundSnafu { slide: id })?)
}

fn slide_ref<'a>(deck: &'a Deck, id: &str) -> Result<&'a Slide, OpError> {
    Ok(deck.slide(id).context(SlideNotFoundSnafu { slide: id })?)
}

fn not_found(slide: &str, element: &str) -> OpError {
    ElementNotFoundSnafu { slide, element }.build().into()
}

fn located<'a>(slide: &'a Slide, id: &str) -> Result<Located<'a>, OpError> {
    slide.locate(id).ok_or_else(|| not_found(&slide.id, id))
}

/// Widget, paramètres et catégorie attendue d'une cible.
fn target_ref<'a>(deck: &'a Deck, t: &Target) -> Result<(&'a str, &'a Params, Category), OpError> {
    let s = slide_ref(deck, &t.slide)?;
    match &t.element {
        None => Ok((&s.kind, &s.params, Category::Slide)),
        Some(id) => {
            let e = located(s, id)?.element;
            Ok((&e.kind, &e.params, Category::Element))
        }
    }
}

fn target_mut<'a>(deck: &'a mut Deck, t: &Target) -> Result<(&'a mut String, &'a mut Params), OpError> {
    let s = slide_mut(deck, &t.slide)?;
    match &t.element {
        None => Ok((&mut s.kind, &mut s.params)),
        Some(id) => {
            let e = element_mut(s, id).ok_or_else(|| not_found(&t.slide, id))?;
            Ok((&mut e.kind, &mut e.params))
        }
    }
}

// ---- Registre -----------------------------------------------------------------------

fn widget<'r>(registry: &'r WidgetRegistry, kind: &str) -> Result<&'r WidgetSchema, OpError> {
    Ok(registry.get(kind).context(UnknownWidgetSnafu { kind })?)
}

/// Le widget `kind`, s'il est de la catégorie `category`.
fn widget_of<'r>(registry: &'r WidgetRegistry, kind: &str, category: Category) -> Result<&'r WidgetSchema, OpError> {
    let schema = widget(registry, kind)?;
    if schema.category != category {
        return Err(WrongCategorySnafu { kind }.build().into());
    }
    Ok(schema)
}

fn declared<'r>(schema: &'r WidgetSchema, param: &str) -> Result<&'r ParamSchema, OpError> {
    Ok(schema.param(param).context(UnknownParamSnafu { widget: &schema.name, param })?)
}

/// Un slot `flow` de `schema` a-t-il des éléments ? Une grille les cacherait.
fn has_filled_flow_slot(schema: &WidgetSchema, params: &Params) -> bool {
    schema.params.iter().any(|p| {
        matches!(p.ty, ParamType::Slot { mode: SlotMode::Flow, .. })
            && params.slot(&p.name).is_some_and(|list| !list.is_empty())
    })
}

/// Mode et `accepts` du slot où se trouve `l`, si son propriétaire est
/// connu du registre et le déclare comme slot.
fn containing_slot<'r>(
    registry: &'r WidgetRegistry,
    slide: &Slide,
    l: &Located<'_>,
) -> Option<(SlotMode, Option<&'r [String]>)> {
    let owner_kind = match l.owner {
        None => slide.kind.as_str(),
        Some(owner) => slide.locate(owner)?.element.kind.as_str(),
    };
    match &registry.get(owner_kind)?.param(l.slot)?.ty {
        ParamType::Slot { mode, accepts } => Some((*mode, accepts.as_deref())),
        _ => None,
    }
}

/// Refuse `kind` à la place de l'élément `id` si son slot ne l'accepte pas.
fn check_containing_accepts(registry: &WidgetRegistry, slide: &Slide, id: &str, kind: &str) -> Result<(), OpError> {
    let l = located(slide, id)?;
    match containing_slot(registry, slide, &l) {
        Some((_, accepts)) => check_accepts(accepts, l.slot, kind),
        None => Ok(()),
    }
}

// ---- Contrôles des éléments ajoutés -------------------------------------------------

/// Identifiants d'un élément et de ses descendants.
fn subtree_ids(e: &Element) -> impl Iterator<Item = &str> {
    std::iter::once(e.id.as_str()).chain(Walk::over(&e.params, Some(&e.id)).map(|l| l.element.id.as_str()))
}

/// Refuse un identifiant de `new` déjà dans `taken`, ou répété dans `new`.
fn check_ids<'a>(mut taken: HashSet<&'a str>, new: &'a Element) -> Result<(), OpError> {
    match subtree_ids(new).find(|id| !taken.insert(*id)) {
        Some(dup) => Err(DuplicateIdSnafu { id: dup }.build().into()),
        None => Ok(()),
    }
}

/// Refuse `e` si, placé dans un slot à `owner_slots` slots sous la slide
/// (0 : slot de la slide), un de ses descendants dépasse [`MAX_SLOT_DEPTH`] slots.
fn check_depth(owner_slots: usize, e: &Element) -> Result<(), OpError> {
    if owner_slots + 1 + slot_depth(e) > MAX_SLOT_DEPTH {
        return Err(InvalidValueSnafu { field: "element", reason: ValueReason::TooDeep }.build().into());
    }
    Ok(())
}

/// Refuse un style dont un champ est hors de son domaine ([`Style::invalid`]).
fn check_style(style: &Style) -> Result<(), OpError> {
    match style.invalid() {
        Some((field, reason)) => Err(InvalidValueSnafu { field, reason }.build().into()),
        None => Ok(()),
    }
}

// ---- Chemins JSON (jetons) ------------------------------------------------------

/// Pose `value` au chemin (en créant les objets manquants) ; renvoie l'ancienne valeur.
fn set_path(root: &mut Value, path: &[String], value: Value) -> Option<Value> {
    if !root.is_object() {
        *root = Value::Object(Map::new());
    }
    let Value::Object(map) = root else { return None };
    match path {
        [] => None,
        [key] => map.insert(key.clone(), value),
        [key, rest @ ..] => set_path(map.entry(key.as_str()).or_insert(Value::Null), rest, value),
    }
}

/// Un chemin de jeton n'est pas vide.
fn token_path(path: &[String]) -> Result<(), OpError> {
    if path.is_empty() {
        return Err(InvalidValueSnafu { field: "path", reason: ValueReason::EmptyPath }.build().into());
    }
    Ok(())
}

/// Retire la valeur au chemin, élague les objets devenus vides et la renvoie.
fn remove_path(root: &mut Value, path: &[String]) -> Option<Value> {
    let map = root.as_object_mut()?;
    match path {
        [] => None,
        [k] => map.remove(k),
        [k, rest @ ..] => {
            let child = map.get_mut(k)?;
            let removed = remove_path(child, rest);
            if child.as_object().is_some_and(Map::is_empty) {
                map.remove(k);
            }
            removed
        }
    }
}

/// Pose ou retire (`null`) une entrée ; renvoie l'ancienne valeur ou `null`.
fn set_or_remove_map(map: &mut BTreeMap<String, Value>, key: &str, value: Value) -> Value {
    let old = if value.is_null() { map.remove(key) } else { map.insert(key.to_owned(), value) };
    old.unwrap_or(Value::Null)
}

// ---- Valeurs ----------------------------------------------------------------------------

fn opt_string(field: &str, v: Value) -> Result<Option<String>, OpError> {
    match v {
        Value::Null => Ok(None),
        // Vide = absent : une prose vide ne s'affiche pas, et le replica
        // relit un texte vide en `None`. Bords rognés : même règle que l'UI.
        Value::String(s) if s.trim().is_empty() => Ok(None),
        Value::String(s) => Ok(Some(s.trim().to_owned())),
        _ => Err(InvalidValueSnafu { field, reason: ValueReason::NotText }.build().into()),
    }
}

fn typed<T: DeserializeOwned>(field: &str, v: Value) -> Result<T, OpError> {
    Ok(serde_json::from_value(v).context(ParseValueSnafu { field })?)
}

// ---- Application ----------------------------------------------------------------

impl Op {
    /// Applies the op to the deck, checked against `registry`. On error, the
    /// deck is intact.
    ///
    /// # Errors
    ///
    /// Missing target, unknown widget or parameter, mistyped value, slot that
    /// refuses the element, id taken, position out of bounds or slot depth
    /// exceeded.
    pub fn apply(self, deck: &mut Deck, registry: &WidgetRegistry) -> Result<(), OpError> {
        match self {
            Op::SetParam { target, param, value } => set_param(deck, registry, &target, &param, value),
            Op::SetDeckParam { key, value } => set_deck_param(deck, &key, value),
            Op::SetKind { target, kind } => set_kind(deck, registry, &target, kind),
            Op::SetChartOption { target, param, patch } => {
                let subject = target.element.as_deref().unwrap_or(&target.slide);
                let (kind, params) = target_mut(deck, &target)?;
                host::set_option(widget(registry, kind)?, params, &param, HostType::Chart, &patch, subject)
            }
            Op::SetTableOption { target, param, patch } => {
                let subject = target.element.as_deref().unwrap_or(&target.slide);
                let (kind, params) = target_mut(deck, &target)?;
                host::set_option(widget(registry, kind)?, params, &param, HostType::Table, &patch, subject)
            }
            Op::SetData { target, param, change } => {
                let subject = target.element.as_deref().unwrap_or(&target.slide);
                let (kind, params) = target_mut(deck, &target)?;
                host::set_data(registry, kind, params, &param, &change, subject)
            }
            Op::EditList { target, param, change } => list::edit_list(deck, registry, &target, &param, change),
            Op::SetStyle { slide, element, style } => {
                // Seul le patch se contrôle : un champ déjà invalide du deck ne bloque pas un autre réglage.
                // Il se contrôle avant la cible, comme `SetFrame` : refusé pour lui-même, même sur un élément absent.
                check_style(&style)?;
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                e.style.merge(style);
                Ok(())
            }
            Op::ReplaceStyle { slide, element, style } => {
                check_style(&style)?;
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                *e.style = style;
                Ok(())
            }
            Op::SetFrame { slide, element, frame } => set_frame(deck, &slide, &element, frame),
            Op::SetCell { slide, element, cell } => set_cell(deck, registry, &slide, &element, cell),
            Op::SetGrid { slide, columns, rows } => grid::set_grid(deck, registry, &slide, columns, rows),
            Op::ReplaceElement { slide, element } => replace_element(deck, registry, &slide, element),
            Op::AddElement { slot, index, defaults, element } => {
                add_element(deck, registry, &slot, index, defaults, element)
            }
            Op::RemoveElement { slide, element } => {
                let s = slide_mut(deck, &slide)?;
                let removed = s.params.slots_mut().find_map(|(_, list)| remove_from(list, &element));
                removed.map(drop).ok_or_else(|| not_found(&slide, &element))
            }
            Op::MoveElement { slide, element, to, index } => {
                moving::move_element(deck, registry, &slide, &element, &to, index)
            }
            Op::AddSlide { index, defaults, slide } => add_slide(deck, registry, index, defaults, slide),
            Op::RemoveSlide { slide } => {
                let Some(i) = deck.slide_index(&slide) else {
                    return Err(SlideNotFoundSnafu { slide }.build().into());
                };
                deck.slides.remove(i);
                Ok(())
            }
            Op::MoveSlide { slide, index } => move_slide(deck, &slide, index),
            Op::SetNotes { slide, notes } => {
                let s = slide_mut(deck, &slide)?;
                s.notes = notes;
                Ok(())
            }
            Op::SetTheme { theme, themes } => {
                check_closure(theme.as_ref(), &themes)?;
                deck.theme = theme;
                deck.themes = themes;
                Ok(())
            }
            Op::AddPack { kind, namespace, pin } => packs::add(deck, kind, namespace, pin),
            Op::RemovePack { kind, namespace } => packs::remove(deck, kind, &namespace),
            Op::SetToken { path, value } => {
                token_path(&path)?;
                set_path(&mut deck.tokens, &path, value);
                Ok(())
            }
            Op::UnsetToken { path } => unset_token(deck, &path),
            Op::SetTokens { tokens } => set_tokens(deck, tokens),
            Op::SetVar { key, value } => {
                set_or_remove_map(&mut deck.vars, &key, value);
                Ok(())
            }
        }
    }
}

/// Applies a sequence of ops atomically: on error, the deck
/// has not changed. The resulting deck is in canonical form
/// ([`canonicalize`](crate::canonicalize)): the one the replica reads back.
///
/// # Errors
///
/// The first refused op (see [`Op::apply`]), with its rank in the sequence.
pub fn apply_all(deck: &mut Deck, ops: Vec<Op>, registry: &WidgetRegistry) -> Result<(), TransactionError> {
    apply_all_moving(deck, ops, registry).map(drop)
}

/// Like `apply_all`, and returns the cross-slide moves of the transaction.
///
/// An element moved several times gives one entry, from its first
/// slide to the last (see [`Moved`]).
///
/// # Errors
///
/// The first refused op (see [`Op::apply`]), with its rank in the sequence.
pub fn apply_all_moving(
    deck: &mut Deck,
    ops: Vec<Op>,
    registry: &WidgetRegistry,
) -> Result<Vec<Moved>, TransactionError> {
    let mut next = deck.clone();
    let mut moves = Vec::new();
    for (index, op) in ops.into_iter().enumerate() {
        let effect = moving::Effect::of(&op);
        op.apply(&mut next, registry).map_err(|source| TransactionSnafu { index }.into_error(source))?;
        effect.record(&mut moves);
    }
    crate::canonicalize(&mut next);
    *deck = next;
    Ok(moves)
}

/// JSON schema of a list of ops, for structured LLM output.
///
/// # Errors
///
/// The generated schema does not convert into a JSON value.
pub fn ops_schema() -> serde_json::Result<Value> {
    serde_json::to_value(schemars::schema_for!(Vec<Op>))
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod schema_tests;
