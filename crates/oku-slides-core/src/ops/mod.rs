//! Opérations sur le deck. Toute modification, qu'elle vienne de la souris ou
//! d'une dictée, passe par une `Op`.
//!
//! Les ops adressent une slide ou un élément par [`Target`], un slot par
//! [`SlotRef`], et contrôlent chaque paramètre par le registre des widgets
//! ([`WidgetRegistry`]) : paramètre déclaré, valeur du bon type
//! ([`param_value`]), slot qui accepte l'élément.

mod check;
mod grid;
mod host;
mod list;
mod value;

use std::collections::{BTreeMap, HashSet};

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{IntoError, OptionExt, ResultExt};

use oku_slides_chart::DataOp;

use crate::error::{
    DuplicateIdSnafu, ElementNotFoundSnafu, IndexOutOfRangeSnafu, InvalidValueSnafu, OpError, ParamTypeSnafu,
    ParseValueSnafu, SlideNotFoundSnafu, TransactionError, TransactionSnafu, UnknownParamSnafu, UnknownWidgetSnafu,
    WrongCategorySnafu,
};
use crate::locate::{Located, MAX_SLOT_DEPTH, Walk, slot_depth};
use crate::model::{Cell, Deck, DeckParams, Element, Frame, Slide, Style};
use crate::pack::{PackRef, ThemeRef};
use crate::params::{ParamValue, Params};
use crate::widget::{Category, HostType, ParamSchema, ParamType, SlotMode, WidgetRegistry, WidgetSchema};

use check::{check_accepts, check_params};
pub(crate) use host::{host_option, normalize_hosts};
pub(crate) use list::{DefaultFault, default_fault};
pub use list::{
    ITEM_ID_MAX, LIST_ITEMS_MAX, ListOp, ListSpec, PLAIN_MAX, RICH_BYTES_MAX, RICH_PARAGRAPHS_MAX, default_items,
    text_fits,
};
pub use value::{GRID_TRACKS, param_value};

/// Refus d'une grille ou d'une cellule sur un slot `flow` : dit comment
/// passer ses éléments sur la grille.
const BLOCKS_OFF_GRID: &str = "flow slots are laid out by the slide and a grid hides them; \
    move each element to a free slot first";

/// Refus d'un élément dont un descendant dépasserait [`MAX_SLOT_DEPTH`].
const TOO_DEEP: &str = "elements nest too deeply";

/// Une slide, ou un élément d'une slide. JSON : `{"slide": "s1"}` ou
/// `{"slide": "s1", "element": "e1"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Target {
    pub slide: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<String>,
}

/// Un slot : le paramètre `param` (de type `slot`) de la slide ou de
/// l'élément `owner`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SlotRef {
    pub owner: Target,
    pub param: String,
}

/// Métadonnée d'une slide, modifiable par `set_slide_field`. Hors rendu :
/// le contenu passe par `set_param`, le widget par `set_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SlideField {
    /// Nom d'une slide modèle.
    Name,
    /// Description d'une slide modèle.
    Description,
}

/// Une opération sur le deck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Paramètre d'une slide ou d'un élément, contrôlé par le schéma de son
    /// widget ; `null` (ou un texte vide) le retire. Un paramètre lié
    /// (`bind`) écrit dans les paramètres du deck. Les éléments d'un slot
    /// passent par `add_element`.
    SetParam {
        target: Target,
        param: String,
        value: Value,
    },
    /// Paramètre du deck (`title`, `author`, `numbering`…) ; `null` le retire.
    SetDeckParam {
        key: String,
        value: Value,
    },
    /// Change le widget d'une slide ou d'un élément (même catégorie) ; les
    /// paramètres sont gardés tels quels.
    SetKind {
        target: Target,
        kind: String,
    },
    /// Fusionne des propriétés de style (les champs absents ne changent pas).
    SetStyle {
        slide: String,
        element: String,
        style: Style,
    },
    /// Remplace tout le style d'un élément.
    ReplaceStyle {
        slide: String,
        element: String,
        style: Style,
    },
    /// Déplace ou redimensionne ; `null` rend l'élément au layout.
    SetFrame {
        slide: String,
        element: String,
        frame: Option<Frame>,
    },
    /// Accroche un élément d'un slot libre à la grille ; `null` le décroche.
    /// Refusé dans un slot `flow` : y retirer l'élément, puis l'ajouter à un slot libre.
    SetCell {
        slide: String,
        element: String,
        cell: Option<Cell>,
    },
    /// Change la grille d'une slide et recadre les éléments de ses slots libres.
    SetGrid {
        slide: String,
        /// Colonnes, de 1 à 24 ; absent : inchangé.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<u32>,
        /// Lignes, de 1 à 24 ; absent : inchangé.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rows: Option<u32>,
    },
    /// Remplace un élément entier (même `id`).
    ReplaceElement {
        slide: String,
        element: Element,
    },
    /// Modifie les options d'un graphique (paramètre `param`, de type
    /// `host(chart)`) : JSON Merge Patch (RFC 7396) sur tout sauf `dataset`
    /// (`null` retire un champ, un tableau remplace le tableau entier).
    SetChartOption {
        target: Target,
        param: String,
        patch: Value,
    },
    /// Modifie les options d'affichage d'un tableau (paramètre `host(table)`).
    ///
    /// JSON Merge Patch sur `header-row`, `header-column`, `striped`, `align` ; jamais `dataset`.
    SetTableOption {
        target: Target,
        param: String,
        patch: Value,
    },
    /// Modifie les données d'un graphique ou d'un tableau.
    ///
    /// `dataset.source`, ligne 0 = noms de colonnes ; `param` est de type `host`.
    SetData {
        target: Target,
        param: String,
        change: DataOp,
    },
    /// Modifie une liste (`list`) : items adressés par leur id. Remplacer
    /// toute la liste : `set_param`.
    EditList {
        target: Target,
        param: String,
        change: ListOp,
    },
    /// Ajoute un élément dans un slot d'une slide ou d'un élément.
    AddElement {
        slot: SlotRef,
        /// Position ; absent = à la fin.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        /// Amorce les listes absentes avec le défaut du schéma : objet nouveau,
        /// pas un collage ni un déplacement.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        defaults: bool,
        element: Element,
    },
    RemoveElement {
        slide: String,
        element: String,
    },
    /// Nom ou description d'une slide ; `null` l'efface.
    SetSlideField {
        slide: String,
        field: SlideField,
        value: Value,
    },
    AddSlide {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        /// Amorce les listes absentes avec le défaut du schéma : objet nouveau,
        /// pas un collage ni un déplacement.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        defaults: bool,
        slide: Slide,
    },
    RemoveSlide {
        slide: String,
    },
    MoveSlide {
        slide: String,
        index: usize,
    },
    SetNotes {
        slide: String,
        notes: String,
    },
    /// Thème du deck (id et version) ; `null` : le thème par défaut.
    SetTheme {
        theme: Option<ThemeRef>,
    },
    /// Pack de modèles du deck (id et version) ; `null` : le pack par défaut.
    SetTemplates {
        templates: Option<PackRef>,
    },
    /// Jeton de design au chemin donné (`["color", "accent"]`). `null` est
    /// posé tel quel : il « vide » une clé qui l'accepte.
    SetToken {
        path: Vec<String>,
        value: Value,
    },
    /// Retire le jeton au chemin donné (retour à la couche inférieure) ; un
    /// objet devenu vide est retiré aussi.
    UnsetToken {
        path: Vec<String>,
    },
    /// Remplace tous les jetons du deck : un objet, ou `null` (aucun).
    SetTokens {
        tokens: Value,
    },
    /// Variable du deck ; `null` la retire.
    SetVar {
        key: String,
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
        return Err(InvalidValueSnafu { field: "element", reason: TOO_DEEP }.build().into());
    }
    Ok(())
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
        return Err(InvalidValueSnafu { field: "path", reason: "empty path" }.build().into());
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
        _ => Err(InvalidValueSnafu { field, reason: "expected a string or null" }.build().into()),
    }
}

fn typed<T: DeserializeOwned>(field: &str, v: Value) -> Result<T, OpError> {
    Ok(serde_json::from_value(v).context(ParseValueSnafu { field })?)
}

// ---- Application ----------------------------------------------------------------

impl Op {
    /// Applique l'op au deck, contrôlée par `registry`. En cas d'erreur, le
    /// deck est intact.
    pub fn apply(self, deck: &mut Deck, registry: &WidgetRegistry) -> Result<(), OpError> {
        match self {
            Op::SetParam { target, param, value } => set_param(deck, registry, &target, &param, value),
            Op::SetDeckParam { key, value } => {
                if DeckParams::is_field(&key) {
                    let text = opt_string(&key, value)?;
                    deck.params.set_field(&key, text);
                } else {
                    set_or_remove_map(&mut deck.params.extra, &key, value);
                }
                Ok(())
            }
            Op::SetKind { target, kind } => {
                let (_, _, category) = target_ref(deck, &target)?;
                widget_of(registry, &kind, category)?;
                if let Some(id) = &target.element {
                    check_containing_accepts(registry, slide_ref(deck, &target.slide)?, id, &kind)?;
                }
                let subject = target.element.as_deref().unwrap_or(&target.slide);
                let mut normalized = target_ref(deck, &target)?.1.clone();
                host::check_host_params(registry, &kind, &mut normalized, subject)?;
                let (k, params) = target_mut(deck, &target)?;
                *params = normalized;
                if *k != kind {
                    list::seed_lists(registry, &kind, params);
                    *k = kind;
                }
                Ok(())
            }
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
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                e.style.merge(style);
                Ok(())
            }
            Op::ReplaceStyle { slide, element, style } => {
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                e.style = style;
                Ok(())
            }
            Op::SetFrame { slide, element, frame } => {
                if let Some(f) = frame
                    && !(f.w > 0.0 && f.h > 0.0)
                {
                    return Err(InvalidValueSnafu { field: "frame", reason: "w and h must be > 0" }.build().into());
                }
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                e.frame = frame;
                Ok(())
            }
            Op::SetCell { slide, element, cell } => {
                if let Some(c) = cell {
                    let s = slide_ref(deck, &slide)?;
                    let l = located(s, &element)?;
                    // Un slot `flow` suit le layout : la grille ne place que les éléments libres.
                    if containing_slot(registry, s, &l).is_some_and(|(mode, _)| mode == SlotMode::Flow) {
                        return Err(InvalidValueSnafu { field: "cell", reason: BLOCKS_OFF_GRID }.build().into());
                    }
                    if !c.fits(&s.resolved_grid(registry)) {
                        return Err(InvalidValueSnafu { field: "cell", reason: "outside the grid" }.build().into());
                    }
                }
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                e.cell = cell;
                Ok(())
            }
            Op::SetGrid { slide, columns, rows } => grid::set_grid(deck, registry, &slide, columns, rows),
            Op::ReplaceElement { slide, mut element } => {
                host::check_hosts(registry, &mut element)?;
                {
                    let s = slide_ref(deck, &slide)?;
                    let previous = located(s, &element.id)?;
                    check_depth(previous.depth, &element)?;
                    check_containing_accepts(registry, s, &element.id, &element.kind)?;
                    check_params(registry, &element.kind, Category::Element, &element.params)?;
                    // Les enfants d'un élément remplacé ne doivent pas entrer en collision.
                    let replaced: HashSet<&str> = subtree_ids(previous.element).collect();
                    check_ids(s.element_ids().filter(|id| !replaced.contains(id)).collect(), &element)?;
                }
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element.id).ok_or_else(|| not_found(&slide, &element.id))?;
                *e = element;
                Ok(())
            }
            Op::AddElement { slot, index, defaults, element } => {
                add_element(deck, registry, slot, index, defaults, element)
            }
            Op::RemoveElement { slide, element } => {
                let s = slide_mut(deck, &slide)?;
                let removed = s.params.slots_mut().find_map(|(_, list)| remove_from(list, &element));
                removed.map(drop).ok_or_else(|| not_found(&slide, &element))
            }
            Op::SetSlideField { slide, field, value } => {
                let s = slide_mut(deck, &slide)?;
                match field {
                    SlideField::Name => s.name = opt_string("name", value)?,
                    SlideField::Description => s.description = opt_string("description", value)?,
                }
                Ok(())
            }
            Op::AddSlide { index, defaults, mut slide } => {
                if deck.slide(&slide.id).is_some() {
                    return Err(DuplicateIdSnafu { id: slide.id }.build().into());
                }
                host::check_host_params(registry, &slide.kind, &mut slide.params, &slide.id)?;
                let mut taken = HashSet::new();
                for (_, list) in slide.params.slots_mut() {
                    for e in list {
                        check_depth(0, e)?;
                        host::check_hosts(registry, e)?;
                        if let Some(dup) = subtree_ids(e).find(|id| !taken.insert(*id)) {
                            return Err(DuplicateIdSnafu { id: dup }.build().into());
                        }
                    }
                }
                check_params(registry, &slide.kind, Category::Slide, &slide.params)?;
                if defaults {
                    list::seed_lists(registry, &slide.kind, &mut slide.params);
                    list::seed_slots(registry, &mut slide.params);
                }
                let at = index.unwrap_or(deck.slides.len());
                if at > deck.slides.len() {
                    return Err(IndexOutOfRangeSnafu { index: at, len: deck.slides.len() }.build().into());
                }
                deck.slides.insert(at, slide);
                Ok(())
            }
            Op::RemoveSlide { slide } => {
                let Some(i) = deck.slide_index(&slide) else {
                    return Err(SlideNotFoundSnafu { slide }.build().into());
                };
                deck.slides.remove(i);
                Ok(())
            }
            Op::MoveSlide { slide, index } => {
                let len = deck.slides.len();
                let i = deck.slide_index(&slide).context(SlideNotFoundSnafu { slide: &slide })?;
                if index >= len {
                    return Err(IndexOutOfRangeSnafu { index, len }.build().into());
                }
                let s = deck.slides.remove(i);
                deck.slides.insert(index, s);
                Ok(())
            }
            Op::SetNotes { slide, notes } => {
                let s = slide_mut(deck, &slide)?;
                s.notes = notes;
                Ok(())
            }
            Op::SetTheme { theme } => {
                deck.theme = theme;
                Ok(())
            }
            Op::SetTemplates { templates } => {
                deck.templates = templates;
                Ok(())
            }
            Op::SetToken { path, value } => {
                token_path(&path)?;
                set_path(&mut deck.tokens, &path, value);
                Ok(())
            }
            Op::UnsetToken { path } => {
                token_path(&path)?;
                remove_path(&mut deck.tokens, &path);
                if deck.tokens.as_object().is_some_and(Map::is_empty) {
                    deck.tokens = Value::Null;
                }
                Ok(())
            }
            Op::SetTokens { tokens } => {
                if !(tokens.is_object() || tokens.is_null()) {
                    return Err(InvalidValueSnafu { field: "tokens", reason: "an object or null" }.build().into());
                }
                deck.tokens = tokens;
                Ok(())
            }
            Op::SetVar { key, value } => {
                set_or_remove_map(&mut deck.vars, &key, value);
                Ok(())
            }
        }
    }
}

/// `set_param` : paramètre déclaré, valeur convertie par [`param_value`],
/// lien vers le deck, grille refusée sur une slide à slot `flow` rempli.
fn set_param(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    target: &Target,
    param: &str,
    value: Value,
) -> Result<(), OpError> {
    let (kind, params, _) = target_ref(deck, target)?;
    let schema = widget(registry, kind)?;
    let p = declared(schema, param)?;
    let converted = param_value(&p.ty, param, value)?;
    if p.ty == ParamType::Grid
        && converted.is_some()
        && target.element.is_none()
        && has_filled_flow_slot(schema, params)
    {
        return Err(InvalidValueSnafu { field: param, reason: BLOCKS_OFF_GRID }.build().into());
    }
    if let Some(key) = &p.bind {
        let text = match converted {
            None => None,
            Some(ParamValue::Text(s)) => Some(s),
            Some(_) => return Err(ParamTypeSnafu { param, expected: "a string or null" }.build().into()),
        };
        deck.params.set_field(key, text);
        return Ok(());
    }
    let (_, params) = target_mut(deck, target)?;
    match converted {
        Some(v) => params.set(param, v),
        None => params.remove(param),
    };
    Ok(())
}

/// `add_element` : slot déclaré qui accepte l'élément, élément connu et
/// cohérent, identifiants libres, profondeur bornée.
fn add_element(
    deck: &mut Deck,
    registry: &WidgetRegistry,
    slot: SlotRef,
    index: Option<usize>,
    defaults: bool,
    mut element: Element,
) -> Result<(), OpError> {
    let SlotRef { owner, param } = slot;
    let at = {
        let s = slide_ref(deck, &owner.slide)?;
        // Profondeur du propriétaire en slots sous la slide (0 : la slide).
        let (kind, params, owner_slots) = match &owner.element {
            None => (s.kind.as_str(), &s.params, 0),
            Some(id) => {
                let l = located(s, id)?;
                (l.element.kind.as_str(), &l.element.params, l.depth + 1)
            }
        };
        let p = declared(widget(registry, kind)?, &param)?;
        let ParamType::Slot { accepts, .. } = &p.ty else {
            return Err(ParamTypeSnafu { param, expected: "a value, through set_param" }.build().into());
        };
        // Une valeur qui n'est pas une liste d'éléments serait écrasée par `slot_mut`.
        let len = match (params.get(&param), params.slot(&param)) {
            (None, _) => 0,
            (Some(_), Some(list)) => list.len(),
            (Some(_), None) => {
                return Err(ParamTypeSnafu { param, expected: "a list of elements" }.build().into());
            }
        };
        check_accepts(accepts.as_deref(), &param, &element.kind)?;
        widget_of(registry, &element.kind, Category::Element)?;
        host::check_hosts(registry, &mut element)?;
        check_ids(s.element_ids().collect(), &element)?;
        check_depth(owner_slots, &element)?;
        check_params(registry, &element.kind, Category::Element, &element.params)?;
        let at = index.unwrap_or(len);
        if at > len {
            return Err(IndexOutOfRangeSnafu { index: at, len }.build().into());
        }
        at
    };
    if defaults {
        list::seed_element(registry, &mut element);
    }
    let s = slide_mut(deck, &owner.slide)?;
    let params = match &owner.element {
        None => &mut s.params,
        Some(id) => &mut element_mut(s, id).ok_or_else(|| not_found(&owner.slide, id))?.params,
    };
    let list = params.slot_mut(&param).context(InvalidValueSnafu { field: &param, reason: "not a slot" })?;
    if at > list.len() {
        return Err(IndexOutOfRangeSnafu { index: at, len: list.len() }.build().into());
    }
    list.insert(at, element);
    Ok(())
}

/// Applique une suite d'ops de façon atomique : en cas d'erreur, le deck
/// n'a pas changé. Le deck obtenu est sous forme canonique
/// ([`canonicalize`](crate::canonicalize)) : celle que relit le replica.
pub fn apply_all(deck: &mut Deck, ops: Vec<Op>, registry: &WidgetRegistry) -> Result<(), TransactionError> {
    let mut next = deck.clone();
    for (index, op) in ops.into_iter().enumerate() {
        op.apply(&mut next, registry).map_err(|source| TransactionSnafu { index }.into_error(source))?;
    }
    crate::canonicalize(&mut next);
    *deck = next;
    Ok(())
}

/// Schéma JSON d'une liste d'ops, pour la sortie structurée des LLM.
pub fn ops_schema() -> serde_json::Result<Value> {
    serde_json::to_value(schemars::schema_for!(Vec<Op>))
}
