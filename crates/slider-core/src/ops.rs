//! Opérations sur le deck. Toute modification, qu'elle vienne de la souris ou
//! d'une dictée, passe par une `Op`. Chaque op appliquée renvoie son inverse.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

use std::collections::HashSet;

use crate::locate::Walk;
use crate::model::{
    Cell, Deck, Element, ElementBody, Frame, Grid, Paragraph, Slide, SlideKind, SlideLayout, Style, Zone,
};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OpError {
    #[error("slide introuvable : {0}")]
    SlideNotFound(String),
    #[error("élément introuvable : {element} (slide {slide})")]
    ElementNotFound { slide: String, element: String },
    #[error("identifiant déjà utilisé : {0}")]
    DuplicateId(String),
    #[error("index hors limites : {index} (taille {len})")]
    IndexOutOfRange { index: usize, len: usize },
    #[error("l'élément {0} n'est pas un groupe")]
    NotAGroup(String),
    #[error("l'élément {0} ne contient pas de texte")]
    NoText(String),
    #[error("valeur invalide pour {field} : {reason}")]
    InvalidValue { field: String, reason: String },
}

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
    /// Accroche un élément à la grille de la slide ; `null` le décroche.
    SetCell {
        slide: String,
        element: String,
        cell: Option<Cell>,
    },
    /// Grille de la slide ; `null` la retire.
    SetGrid {
        slide: String,
        grid: Option<Grid>,
    },
    /// Remplace un élément entier (même `id`).
    ReplaceElement {
        slide: String,
        element: Element,
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
            parent.children_mut().ok_or_else(|| OpError::NotAGroup(pid.to_owned()))
        }
    }
}

// ---- Chemins JSON (jetons) ------------------------------------------------------

fn get_path<'a>(root: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter().try_fold(root, |v, k| v.as_object()?.get(k))
}

fn set_path(root: &mut Value, path: &[String], value: Value) {
    if !root.is_object() {
        *root = Value::Object(Map::new());
    }
    let (last, init) = path.split_last().expect("chemin non vide");
    let mut cur = root;
    for k in init {
        let map = cur.as_object_mut().expect("objet");
        let next = map.entry(k.clone()).or_insert_with(|| Value::Object(Map::new()));
        if !next.is_object() {
            *next = Value::Object(Map::new());
        }
        cur = next;
    }
    cur.as_object_mut().expect("objet").insert(last.clone(), value);
}

/// Retire la valeur au chemin et élague les objets devenus vides.
fn remove_path(root: &mut Value, path: &[String]) {
    fn rec(v: &mut Value, path: &[String]) {
        let Some(map) = v.as_object_mut() else { return };
        match path {
            [] => {}
            [k] => {
                map.remove(k);
            }
            [k, rest @ ..] => {
                if let Some(child) = map.get_mut(k) {
                    rec(child, rest);
                    if child.as_object().is_some_and(Map::is_empty) {
                        map.remove(k);
                    }
                }
            }
        }
    }
    rec(root, path);
}

fn set_or_remove_map(map: &mut std::collections::BTreeMap<String, Value>, key: &str, value: Value) -> Value {
    let old = if value.is_null() { map.remove(key) } else { map.insert(key.to_owned(), value) };
    old.unwrap_or(Value::Null)
}

// ---- Application ----------------------------------------------------------------

fn slide_mut<'a>(deck: &'a mut Deck, id: &str) -> Result<&'a mut Slide, OpError> {
    deck.slide_mut(id).ok_or_else(|| OpError::SlideNotFound(id.to_owned()))
}

fn not_found(slide: &str, element: &str) -> OpError {
    OpError::ElementNotFound { slide: slide.to_owned(), element: element.to_owned() }
}

fn opt_string(field: &str, v: &Value) -> Result<Option<String>, OpError> {
    match v {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s.clone())),
        _ => Err(OpError::InvalidValue { field: field.to_owned(), reason: "chaîne ou null attendue".into() }),
    }
}

fn to_value<T: Serialize>(v: &T) -> Value {
    serde_json::to_value(v).expect("sérialisable")
}

impl Op {
    /// Applique l'op et renvoie son inverse. En cas d'erreur, le deck est intact.
    pub fn apply(&self, deck: &mut Deck) -> Result<Op, OpError> {
        match self {
            Op::SetText { slide, element, text } => {
                let s = slide_mut(deck, slide)?;
                let e = element_mut(s, element).ok_or_else(|| not_found(slide, element))?;
                let target = match &mut e.body {
                    ElementBody::Text { text } | ElementBody::Card { text, .. } => text,
                    _ => return Err(OpError::NoText(element.clone())),
                };
                let old = std::mem::replace(target, text.clone());
                Ok(Op::SetText { slide: slide.clone(), element: element.clone(), text: old })
            }
            Op::SetStyle { slide, element, style } => {
                let s = slide_mut(deck, slide)?;
                let e = element_mut(s, element).ok_or_else(|| not_found(slide, element))?;
                let old = e.style.clone();
                e.style.merge(style);
                Ok(Op::ReplaceStyle { slide: slide.clone(), element: element.clone(), style: old })
            }
            Op::ReplaceStyle { slide, element, style } => {
                let s = slide_mut(deck, slide)?;
                let e = element_mut(s, element).ok_or_else(|| not_found(slide, element))?;
                let old = std::mem::replace(&mut e.style, style.clone());
                Ok(Op::ReplaceStyle { slide: slide.clone(), element: element.clone(), style: old })
            }
            Op::SetFrame { slide, element, frame } => {
                if let Some(f) = frame
                    && !(f.w > 0.0 && f.h > 0.0)
                {
                    return Err(OpError::InvalidValue {
                        field: "frame".into(),
                        reason: "w et h doivent être > 0".into(),
                    });
                }
                let s = slide_mut(deck, slide)?;
                let e = element_mut(s, element).ok_or_else(|| not_found(slide, element))?;
                let old = std::mem::replace(&mut e.frame, *frame);
                Ok(Op::SetFrame { slide: slide.clone(), element: element.clone(), frame: old })
            }
            Op::SetCell { slide, element, cell } => {
                let s = slide_mut(deck, slide)?;
                if let Some(c) = cell {
                    let grid = s.effective_grid();
                    if !c.fits(grid) {
                        return Err(OpError::InvalidValue {
                            field: "cell".into(),
                            reason: format!("hors de la grille {} × {}", grid.columns, grid.rows),
                        });
                    }
                }
                let e = element_mut(s, element).ok_or_else(|| not_found(slide, element))?;
                let old = std::mem::replace(&mut e.cell, *cell);
                Ok(Op::SetCell { slide: slide.clone(), element: element.clone(), cell: old })
            }
            Op::SetGrid { slide, grid } => {
                if let Some(g) = grid
                    && (!(1..=24).contains(&g.columns) || !(1..=24).contains(&g.rows))
                {
                    return Err(OpError::InvalidValue {
                        field: "grid".into(),
                        reason: "1 à 24 colonnes et lignes".into(),
                    });
                }
                let s = slide_mut(deck, slide)?;
                let old = std::mem::replace(&mut s.grid, grid.clone());
                Ok(Op::SetGrid { slide: slide.clone(), grid: old })
            }
            Op::ReplaceElement { slide, element } => {
                let s = slide_mut(deck, slide)?;
                // Les enfants d'un groupe remplacé ne doivent pas entrer en collision.
                let old = s.locate(&element.id).ok_or_else(|| not_found(slide, &element.id))?.element;
                let old_ids: HashSet<&str> =
                    Walk::over(old.children(), Zone::Blocks).map(|l| l.element.id.as_str()).collect();
                let existing: HashSet<&str> = s.element_ids().collect();
                if let Some(dup) = Walk::over(element.children(), Zone::Blocks)
                    .map(|l| l.element.id.as_str())
                    .find(|id| existing.contains(id) && !old_ids.contains(id))
                {
                    return Err(OpError::DuplicateId(dup.to_owned()));
                }
                let e = element_mut(s, &element.id).ok_or_else(|| not_found(slide, &element.id))?;
                let old = std::mem::replace(e, element.clone());
                Ok(Op::ReplaceElement { slide: slide.clone(), element: old })
            }
            Op::AddElement { slide, zone, parent, index, element } => {
                let s = slide_mut(deck, slide)?;
                let existing: HashSet<&str> = s.element_ids().collect();
                if let Some(dup) =
                    Walk::over(std::slice::from_ref(element), *zone).find(|l| existing.contains(l.element.id.as_str()))
                {
                    return Err(OpError::DuplicateId(dup.element.id.clone()));
                }
                let list = container_mut(s, *zone, parent.as_deref(), slide)?;
                let at = index.unwrap_or(list.len());
                if at > list.len() {
                    return Err(OpError::IndexOutOfRange { index: at, len: list.len() });
                }
                list.insert(at, element.clone());
                Ok(Op::RemoveElement { slide: slide.clone(), element: element.id.clone() })
            }
            Op::RemoveElement { slide, element } => {
                let s = slide_mut(deck, slide)?;
                let (zone, parent, index) = s
                    .locate(element)
                    .map(|l| (l.zone, l.parent.map(str::to_owned), l.index))
                    .ok_or_else(|| not_found(slide, element))?;
                let list = container_mut(s, zone, parent.as_deref(), slide)?;
                let removed = list.remove(index);
                Ok(Op::AddElement { slide: slide.clone(), zone, parent, index: Some(index), element: removed })
            }
            Op::SetSlideField { slide, field, value } => {
                let s = slide_mut(deck, slide)?;
                let old = match field {
                    SlideField::Kind => {
                        let kind: SlideKind = serde_json::from_value(value.clone())
                            .map_err(|e| OpError::InvalidValue { field: "kind".into(), reason: e.to_string() })?;
                        to_value(&std::mem::replace(&mut s.kind, kind))
                    }
                    SlideField::Layout => {
                        let layout: Option<SlideLayout> = serde_json::from_value(value.clone())
                            .map_err(|e| OpError::InvalidValue { field: "layout".into(), reason: e.to_string() })?;
                        to_value(&std::mem::replace(&mut s.layout, layout))
                    }
                    SlideField::Title => to_value(&std::mem::replace(&mut s.title, opt_string("title", value)?)),
                    SlideField::Subtitle => {
                        to_value(&std::mem::replace(&mut s.subtitle, opt_string("subtitle", value)?))
                    }
                    SlideField::Kicker => to_value(&std::mem::replace(&mut s.kicker, opt_string("kicker", value)?)),
                    SlideField::Text => to_value(&std::mem::replace(&mut s.text, opt_string("text", value)?)),
                    SlideField::Name => to_value(&std::mem::replace(&mut s.name, opt_string("name", value)?)),
                    SlideField::Description => {
                        to_value(&std::mem::replace(&mut s.description, opt_string("description", value)?))
                    }
                };
                Ok(Op::SetSlideField { slide: slide.clone(), field: *field, value: old })
            }
            Op::AddSlide { index, slide } => {
                if deck.slide(&slide.id).is_some() {
                    return Err(OpError::DuplicateId(slide.id.clone()));
                }
                let at = index.unwrap_or(deck.slides.len());
                if at > deck.slides.len() {
                    return Err(OpError::IndexOutOfRange { index: at, len: deck.slides.len() });
                }
                deck.slides.insert(at, slide.clone());
                Ok(Op::RemoveSlide { slide: slide.id.clone() })
            }
            Op::RemoveSlide { slide } => {
                let i = deck.slide_index(slide).ok_or_else(|| OpError::SlideNotFound(slide.clone()))?;
                let removed = deck.slides.remove(i);
                Ok(Op::AddSlide { index: Some(i), slide: removed })
            }
            Op::MoveSlide { slide, index } => {
                let i = deck.slide_index(slide).ok_or_else(|| OpError::SlideNotFound(slide.clone()))?;
                if *index >= deck.slides.len() {
                    return Err(OpError::IndexOutOfRange { index: *index, len: deck.slides.len() });
                }
                let s = deck.slides.remove(i);
                deck.slides.insert(*index, s);
                Ok(Op::MoveSlide { slide: slide.clone(), index: i })
            }
            Op::SetNotes { slide, notes } => {
                let s = slide_mut(deck, slide)?;
                let old = std::mem::replace(&mut s.notes, notes.clone());
                Ok(Op::SetNotes { slide: slide.clone(), notes: old })
            }
            Op::SetTheme { theme } => {
                let old = std::mem::replace(&mut deck.theme, theme.clone());
                Ok(Op::SetTheme { theme: old })
            }
            Op::SetTemplates { templates } => {
                let old = std::mem::replace(&mut deck.templates, templates.clone());
                Ok(Op::SetTemplates { templates: old })
            }
            Op::SetToken { path, value } => {
                if path.is_empty() {
                    return Err(OpError::InvalidValue { field: "path".into(), reason: "chemin vide".into() });
                }
                let old = get_path(&deck.tokens, path).cloned().unwrap_or(Value::Null);
                if value.is_null() {
                    remove_path(&mut deck.tokens, path);
                    if deck.tokens.as_object().is_some_and(Map::is_empty) {
                        deck.tokens = Value::Null;
                    }
                } else {
                    set_path(&mut deck.tokens, path, value.clone());
                }
                Ok(Op::SetToken { path: path.clone(), value: old })
            }
            Op::SetParam { key, value } => {
                let p = &mut deck.params;
                let old = match key.as_str() {
                    "title" => to_value(&std::mem::replace(&mut p.title, opt_string(key, value)?)),
                    "subtitle" => to_value(&std::mem::replace(&mut p.subtitle, opt_string(key, value)?)),
                    "author" => to_value(&std::mem::replace(&mut p.author, opt_string(key, value)?)),
                    "date" => to_value(&std::mem::replace(&mut p.date, opt_string(key, value)?)),
                    "organization" => to_value(&std::mem::replace(&mut p.organization, opt_string(key, value)?)),
                    "lang" => to_value(&std::mem::replace(&mut p.lang, opt_string(key, value)?)),
                    _ => set_or_remove_map(&mut p.extra, key, value.clone()),
                };
                Ok(Op::SetParam { key: key.clone(), value: old })
            }
            Op::SetVar { key, value } => {
                let old = set_or_remove_map(&mut deck.vars, key, value.clone());
                Ok(Op::SetVar { key: key.clone(), value: old })
            }
        }
    }
}

/// Applique une suite d'ops de façon atomique : en cas d'erreur, les ops
/// déjà appliquées sont annulées. Renvoie les inverses, dans l'ordre où il
/// faut les appliquer pour tout défaire.
pub fn apply_all(deck: &mut Deck, ops: &[Op]) -> Result<Vec<Op>, (usize, OpError)> {
    let mut inverses: Vec<Op> = Vec::with_capacity(ops.len());
    for (i, op) in ops.iter().enumerate() {
        match op.apply(deck) {
            Ok(inv) => inverses.push(inv),
            Err(e) => {
                for inv in inverses.iter().rev() {
                    inv.apply(deck).expect("l'inverse d'une op appliquée s'applique toujours");
                }
                return Err((i, e));
            }
        }
    }
    inverses.reverse();
    Ok(inverses)
}

/// Schéma JSON d'une liste d'ops, pour la sortie structurée des LLM.
pub fn ops_schema() -> Value {
    serde_json::to_value(schemars::schema_for!(Vec<Op>)).expect("schéma sérialisable")
}
