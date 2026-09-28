//! Opérations sur le deck. Toute modification, qu'elle vienne de la souris ou
//! d'une dictée, passe par une `Op`.

use std::collections::{BTreeMap, HashSet};

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{IntoError, OptionExt, ResultExt};

use oku_slides_chart::{ChartDataOp, ChartLabels, apply_option_patch, normalize, sync_series_with_columns};

use crate::error::{
    ChartSnafu, DuplicateIdSnafu, ElementNotFoundSnafu, IndexOutOfRangeSnafu, InvalidValueSnafu, NoTextSnafu,
    NotAChartSnafu, NotAGroupSnafu, OpError, ParseValueSnafu, SlideNotFoundSnafu, TransactionError, TransactionSnafu,
};
use crate::locate::Walk;
use crate::model::{
    Cell, Deck, Element, ElementBody, Frame, Grid, Paragraph, Slide, SlideKind, SlideLayout, Style, Zone,
};

/// Refus de `set_grid` / `set_cell` sur des blocs : dit comment les passer sur la grille.
const BLOCKS_OFF_GRID: &str = "blocks follow the slide layout and a grid hides them; \
    move each block to the elements zone first (remove_element, then add_element with zone \"elements\" and a cell)";

/// Champ simple d'une slide, modifiable par `set_slide_field`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SlideField {
    Kind,
    Title,
    Subtitle,
    Kicker,
    Layout,
    Text,
    /// Nom et description d'une slide modèle.
    Name,
    Description,
}

/// Une opération sur le deck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Remplace le texte d'un élément texte ou d'une carte.
    SetText {
        slide: String,
        element: String,
        text: Vec<Paragraph>,
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
    /// Accroche un élément libre (zone `elements`) à la grille ; `null` le décroche.
    /// Refusé sur un bloc : le passer d'abord en élément libre.
    SetCell {
        slide: String,
        element: String,
        cell: Option<Cell>,
    },
    /// Grille de la slide ; `null` la retire.
    /// Refusé tant que la slide a des blocs : une grille ne place que les éléments libres.
    SetGrid {
        slide: String,
        grid: Option<Grid>,
    },
    /// Remplace un élément entier (même `id`).
    ReplaceElement {
        slide: String,
        element: Element,
    },
    /// Modifie les options d'un graphique : JSON Merge Patch (RFC 7396) sur
    /// tout sauf `dataset` (`null` retire un champ, un tableau remplace le
    /// tableau entier).
    SetChartOption {
        slide: String,
        element: String,
        patch: Value,
    },
    /// Modifie les données d'un graphique (`dataset.source`, ligne 0 = en-têtes).
    SetChartData {
        slide: String,
        element: String,
        change: ChartDataOp,
    },
    /// Ajoute un élément dans une zone de la slide ou dans un groupe.
    AddElement {
        slide: String,
        #[serde(default)]
        zone: Zone,
        /// Groupe parent ; prioritaire sur `zone`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        /// Position ; absent = à la fin.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
        element: Element,
    },
    RemoveElement {
        slide: String,
        element: String,
    },
    /// Modifie un champ de la slide ; `null` l'efface.
    SetSlideField {
        slide: String,
        field: SlideField,
        value: Value,
    },
    AddSlide {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        index: Option<usize>,
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
    /// Thème de la bibliothèque (`papier`, `pro`) ou `null`.
    SetTheme {
        theme: Option<String>,
    },
    /// Ensemble de modèles de slides du deck ; `null` : l'ensemble par défaut.
    SetTemplates {
        templates: Option<String>,
    },
    /// Jeton de design au chemin donné (`["color", "accent"]`) ; `null` le retire.
    SetToken {
        path: Vec<String>,
        value: Value,
    },
    /// Paramètre du deck (`title`, `author`, `numbering`…) ; `null` le retire.
    SetParam {
        key: String,
        value: Value,
    },
    /// Variable du deck ; `null` la retire.
    SetVar {
        key: String,
        value: Value,
    },
}

// ---- Localisation des éléments ------------------------------------------------

fn find_mut<'a>(list: &'a mut [Element], id: &str) -> Option<&'a mut Element> {
    for e in list.iter_mut() {
        if e.id == id {
            return Some(e);
        }
        if let Some(children) = e.children_mut()
            && let Some(found) = find_mut(children, id)
        {
            return Some(found);
        }
    }
    None
}

fn element_mut<'a>(slide: &'a mut Slide, id: &str) -> Option<&'a mut Element> {
    if let Some(e) = find_mut(&mut slide.blocks, id) {
        return Some(e);
    }
    find_mut(&mut slide.elements, id)
}

/// Liste qui reçoit un élément : une zone de la slide, ou un groupe.
/// `slide_id` ne sert qu'au message d'erreur.
fn container_mut<'a>(
    slide: &'a mut Slide,
    zone: Zone,
    parent: Option<&str>,
    slide_id: &str,
) -> Result<&'a mut Vec<Element>, OpError> {
    match parent {
        None => Ok(match zone {
            Zone::Blocks => &mut slide.blocks,
            Zone::Elements => &mut slide.elements,
        }),
        Some(pid) => {
            let parent = element_mut(slide, pid).ok_or_else(|| not_found(slide_id, pid))?;
            Ok(parent.children_mut().context(NotAGroupSnafu { element: pid })?)
        }
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

// ---- Application ----------------------------------------------------------------

/// Refuse un élément (ou un de ses enfants) dont le graphique est incohérent.
fn check_charts(element: &Element) -> Result<(), OpError> {
    for l in Walk::over(std::slice::from_ref(element), Zone::Blocks) {
        if let ElementBody::Chart { option } = &l.element.body {
            normalize(option, ChartLabels::english()).context(ChartSnafu { element: &l.element.id })?;
        }
    }
    Ok(())
}

/// `set_chart_data` sur l'élément `e`.
fn set_chart_data(e: &mut Element, change: ChartDataOp) -> Result<(), OpError> {
    let element = e.id.clone();
    let before = e.clone();
    let ElementBody::Chart { option } = &mut e.body else {
        return Err(NotAChartSnafu { element }.build().into());
    };
    let applied = (|| -> Result<(), OpError> {
        let dataset = option
            .dataset
            .as_mut()
            .context(InvalidValueSnafu { field: "dataset", reason: "the chart has no dataset" })?;
        change.apply(dataset).context(ChartSnafu { element: &element })?;
        sync_series_with_columns(option);
        normalize(option, ChartLabels::english()).context(ChartSnafu { element: &element })?;
        Ok(())
    })();
    if let Err(err) = applied {
        // Le deck reste intact.
        *e = before;
        return Err(err);
    }
    Ok(())
}

fn slide_mut<'a>(deck: &'a mut Deck, id: &str) -> Result<&'a mut Slide, OpError> {
    Ok(deck.slide_mut(id).context(SlideNotFoundSnafu { slide: id })?)
}

fn not_found(slide: &str, element: &str) -> OpError {
    ElementNotFoundSnafu { slide, element }.build().into()
}

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

impl Op {
    /// Applique l'op au deck. En cas d'erreur, le deck est intact.
    pub fn apply(self, deck: &mut Deck) -> Result<(), OpError> {
        match self {
            Op::SetText { slide, element, text } => {
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                let target = match &mut e.body {
                    ElementBody::Text { text } | ElementBody::Card { text, .. } => text,
                    _ => return Err(NoTextSnafu { element }.build().into()),
                };
                *target = text;
                Ok(())
            }
            Op::SetChartOption { slide, element, patch } => {
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                let ElementBody::Chart { option } = &mut e.body else {
                    return Err(NotAChartSnafu { element }.build().into());
                };
                let (next, _) = apply_option_patch(option, &patch).context(ChartSnafu { element: &element })?;
                **option = next;
                Ok(())
            }
            Op::SetChartData { slide, element, change } => {
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                set_chart_data(e, change)
            }
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
                let s = slide_mut(deck, &slide)?;
                if let Some(c) = cell {
                    // Un bloc suit le layout : la grille ne place que les éléments libres.
                    if s.locate(&element).is_some_and(|l| l.zone == Zone::Blocks) {
                        return Err(InvalidValueSnafu { field: "cell", reason: BLOCKS_OFF_GRID }.build().into());
                    }
                    let grid = s.effective_grid();
                    if !c.fits(grid) {
                        return Err(InvalidValueSnafu { field: "cell", reason: "outside the grid" }.build().into());
                    }
                }
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                e.cell = cell;
                Ok(())
            }
            Op::SetGrid { slide, grid } => {
                if let Some(g) = &grid
                    && (!(1..=24).contains(&g.columns) || !(1..=24).contains(&g.rows))
                {
                    return Err(InvalidValueSnafu { field: "grid", reason: "1 to 24 columns and rows" }.build().into());
                }
                let s = slide_mut(deck, &slide)?;
                // Avec une grille, la slide ne dessine plus ses blocs.
                if grid.is_some() && !s.blocks.is_empty() {
                    return Err(InvalidValueSnafu { field: "grid", reason: BLOCKS_OFF_GRID }.build().into());
                }
                s.grid = grid;
                Ok(())
            }
            Op::ReplaceElement { slide, element } => {
                check_charts(&element)?;
                let s = slide_mut(deck, &slide)?;
                // Les enfants d'un groupe remplacé ne doivent pas entrer en collision.
                let previous = s.locate(&element.id).ok_or_else(|| not_found(&slide, &element.id))?.element;
                let previous_ids: HashSet<&str> =
                    Walk::over(previous.children(), Zone::Blocks).map(|l| l.element.id.as_str()).collect();
                let existing: HashSet<&str> = s.element_ids().collect();
                if let Some(dup) = Walk::over(element.children(), Zone::Blocks)
                    .map(|l| l.element.id.as_str())
                    .find(|id| existing.contains(id) && !previous_ids.contains(id))
                {
                    return Err(DuplicateIdSnafu { id: dup }.build().into());
                }
                let e = element_mut(s, &element.id).ok_or_else(|| not_found(&slide, &element.id))?;
                *e = element;
                Ok(())
            }
            Op::AddElement { slide, zone, parent, index, element } => {
                check_charts(&element)?;
                let s = slide_mut(deck, &slide)?;
                let existing: HashSet<&str> = s.element_ids().collect();
                if let Some(dup) =
                    Walk::over(std::slice::from_ref(&element), zone).find(|l| existing.contains(l.element.id.as_str()))
                {
                    return Err(DuplicateIdSnafu { id: &dup.element.id }.build().into());
                }
                let list = container_mut(s, zone, parent.as_deref(), &slide)?;
                let at = index.unwrap_or(list.len());
                if at > list.len() {
                    return Err(IndexOutOfRangeSnafu { index: at, len: list.len() }.build().into());
                }
                list.insert(at, element);
                Ok(())
            }
            Op::RemoveElement { slide, element } => {
                let s = slide_mut(deck, &slide)?;
                let (zone, parent, index) = s
                    .locate(&element)
                    .map(|l| (l.zone, l.parent.map(str::to_owned), l.index))
                    .ok_or_else(|| not_found(&slide, &element))?;
                let list = container_mut(s, zone, parent.as_deref(), &slide)?;
                list.remove(index);
                Ok(())
            }
            Op::SetSlideField { slide, field, value } => {
                let s = slide_mut(deck, &slide)?;
                match field {
                    SlideField::Kind => s.kind = typed::<SlideKind>("kind", value)?,
                    SlideField::Layout => s.layout = typed::<Option<SlideLayout>>("layout", value)?,
                    SlideField::Title => s.title = opt_string("title", value)?,
                    SlideField::Subtitle => s.subtitle = opt_string("subtitle", value)?,
                    SlideField::Kicker => s.kicker = opt_string("kicker", value)?,
                    SlideField::Text => s.text = opt_string("text", value)?,
                    SlideField::Name => s.name = opt_string("name", value)?,
                    SlideField::Description => s.description = opt_string("description", value)?,
                }
                Ok(())
            }
            Op::AddSlide { index, slide } => {
                if deck.slide(&slide.id).is_some() {
                    return Err(DuplicateIdSnafu { id: slide.id }.build().into());
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
                if path.is_empty() {
                    return Err(InvalidValueSnafu { field: "path", reason: "empty path" }.build().into());
                }
                if value.is_null() {
                    remove_path(&mut deck.tokens, &path);
                    if deck.tokens.as_object().is_some_and(Map::is_empty) {
                        deck.tokens = Value::Null;
                    }
                } else {
                    set_path(&mut deck.tokens, &path, value);
                }
                Ok(())
            }
            Op::SetParam { key, value } => {
                let p = &mut deck.params;
                match key.as_str() {
                    "title" => p.title = opt_string(&key, value)?,
                    "subtitle" => p.subtitle = opt_string(&key, value)?,
                    "author" => p.author = opt_string(&key, value)?,
                    "date" => p.date = opt_string(&key, value)?,
                    "organization" => p.organization = opt_string(&key, value)?,
                    "lang" => p.lang = opt_string(&key, value)?,
                    _ => {
                        set_or_remove_map(&mut p.extra, &key, value);
                    }
                }
                Ok(())
            }
            Op::SetVar { key, value } => {
                set_or_remove_map(&mut deck.vars, &key, value);
                Ok(())
            }
        }
    }
}

/// Applique une suite d'ops de façon atomique : en cas d'erreur, le deck
/// n'a pas changé. Le deck obtenu est sous forme canonique
/// ([`canonicalize`](crate::canonicalize)) : celle que relit le replica.
pub fn apply_all(deck: &mut Deck, ops: Vec<Op>) -> Result<(), TransactionError> {
    let mut next = deck.clone();
    for (index, op) in ops.into_iter().enumerate() {
        op.apply(&mut next).map_err(|source| TransactionSnafu { index }.into_error(source))?;
    }
    crate::canonicalize(&mut next);
    *deck = next;
    Ok(())
}

/// Schéma JSON d'une liste d'ops, pour la sortie structurée des LLM.
pub fn ops_schema() -> serde_json::Result<Value> {
    serde_json::to_value(schemars::schema_for!(Vec<Op>))
}
