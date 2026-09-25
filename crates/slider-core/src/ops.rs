//! Opérations sur le deck. Toute modification, qu'elle vienne de la souris ou
//! d'une dictée, passe par une `Op`. Chaque op appliquée renvoie son inverse.

use std::collections::{BTreeMap, HashSet};

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

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

/// Pose `value` au chemin (en créant les objets manquants) ; renvoie l'ancienne valeur.
fn set_path(root: &mut Value, path: &[String], value: Value) -> Option<Value> {
    if !root.is_object() {
        *root = Value::Object(Map::new());
    }
    let (last, init) = path.split_last().expect("chemin non vide");
    let mut cur = root;
    for k in init {
        let map = cur.as_object_mut().expect("objet");
        let next = map.entry(k.as_str()).or_insert_with(|| Value::Object(Map::new()));
        if !next.is_object() {
            *next = Value::Object(Map::new());
        }
        cur = next;
    }
    cur.as_object_mut().expect("objet").insert(last.clone(), value)
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

fn slide_mut<'a>(deck: &'a mut Deck, id: &str) -> Result<&'a mut Slide, OpError> {
    deck.slide_mut(id).ok_or_else(|| OpError::SlideNotFound(id.to_owned()))
}

fn not_found(slide: &str, element: &str) -> OpError {
    OpError::ElementNotFound { slide: slide.to_owned(), element: element.to_owned() }
}

fn opt_string(field: &str, v: Value) -> Result<Option<String>, OpError> {
    match v {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s)),
        _ => Err(OpError::InvalidValue { field: field.to_owned(), reason: "chaîne ou null attendue".into() }),
    }
}

fn typed<T: DeserializeOwned>(field: &str, v: Value) -> Result<T, OpError> {
    serde_json::from_value(v).map_err(|e| OpError::InvalidValue { field: field.to_owned(), reason: e.to_string() })
}

fn to_value<T: Serialize>(v: &T) -> Value {
    serde_json::to_value(v).expect("sérialisable")
}

impl Op {
    /// Applique l'op et renvoie son inverse. L'op se consomme : son contenu
    /// entre dans le deck, ce qu'il remplace part dans l'inverse. En cas
    /// d'erreur, le deck est intact.
    pub fn apply(self, deck: &mut Deck) -> Result<Op, OpError> {
        match self {
            Op::SetText { slide, element, text } => {
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                let target = match &mut e.body {
                    ElementBody::Text { text } | ElementBody::Card { text, .. } => text,
                    _ => return Err(OpError::NoText(element)),
                };
                let old = std::mem::replace(target, text);
                Ok(Op::SetText { slide, element, text: old })
            }
            Op::SetStyle { slide, element, style } => {
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                // L'inverse remplace tout le style : il faut l'état d'avant la fusion.
                let old = e.style.clone();
                e.style.merge(style);
                Ok(Op::ReplaceStyle { slide, element, style: old })
            }
            Op::ReplaceStyle { slide, element, style } => {
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                let old = std::mem::replace(&mut e.style, style);
                Ok(Op::ReplaceStyle { slide, element, style: old })
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
                let s = slide_mut(deck, &slide)?;
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                let old = std::mem::replace(&mut e.frame, frame);
                Ok(Op::SetFrame { slide, element, frame: old })
            }
            Op::SetCell { slide, element, cell } => {
                let s = slide_mut(deck, &slide)?;
                if let Some(c) = cell {
                    let grid = s.effective_grid();
                    if !c.fits(grid) {
                        return Err(OpError::InvalidValue {
                            field: "cell".into(),
                            reason: format!("hors de la grille {} × {}", grid.columns, grid.rows),
                        });
                    }
                }
                let e = element_mut(s, &element).ok_or_else(|| not_found(&slide, &element))?;
                let old = std::mem::replace(&mut e.cell, cell);
                Ok(Op::SetCell { slide, element, cell: old })
            }
            Op::SetGrid { slide, grid } => {
                if let Some(g) = &grid
                    && (!(1..=24).contains(&g.columns) || !(1..=24).contains(&g.rows))
                {
                    return Err(OpError::InvalidValue {
                        field: "grid".into(),
                        reason: "1 à 24 colonnes et lignes".into(),
                    });
                }
                let s = slide_mut(deck, &slide)?;
                let old = std::mem::replace(&mut s.grid, grid);
                Ok(Op::SetGrid { slide, grid: old })
            }
            Op::ReplaceElement { slide, element } => {
                let s = slide_mut(deck, &slide)?;
                // Les enfants d'un groupe remplacé ne doivent pas entrer en collision.
                let old = s.locate(&element.id).ok_or_else(|| not_found(&slide, &element.id))?.element;
                let old_ids: HashSet<&str> =
                    Walk::over(old.children(), Zone::Blocks).map(|l| l.element.id.as_str()).collect();
                let existing: HashSet<&str> = s.element_ids().collect();
                if let Some(dup) = Walk::over(element.children(), Zone::Blocks)
                    .map(|l| l.element.id.as_str())
                    .find(|id| existing.contains(id) && !old_ids.contains(id))
                {
                    return Err(OpError::DuplicateId(dup.to_owned()));
                }
                let e = element_mut(s, &element.id).ok_or_else(|| not_found(&slide, &element.id))?;
                let old = std::mem::replace(e, element);
                Ok(Op::ReplaceElement { slide, element: old })
            }
            Op::AddElement { slide, zone, parent, index, element } => {
                let s = slide_mut(deck, &slide)?;
                let existing: HashSet<&str> = s.element_ids().collect();
                if let Some(dup) =
                    Walk::over(std::slice::from_ref(&element), zone).find(|l| existing.contains(l.element.id.as_str()))
                {
                    return Err(OpError::DuplicateId(dup.element.id.clone()));
                }
                let list = container_mut(s, zone, parent.as_deref(), &slide)?;
                let at = index.unwrap_or(list.len());
                if at > list.len() {
                    return Err(OpError::IndexOutOfRange { index: at, len: list.len() });
                }
                // L'élément entre dans le deck ; l'inverse n'en garde que l'identifiant.
                let id = element.id.clone();
                list.insert(at, element);
                Ok(Op::RemoveElement { slide, element: id })
            }
            Op::RemoveElement { slide, element } => {
                let s = slide_mut(deck, &slide)?;
                let (zone, parent, index) = s
                    .locate(&element)
                    .map(|l| (l.zone, l.parent.map(str::to_owned), l.index))
                    .ok_or_else(|| not_found(&slide, &element))?;
                let list = container_mut(s, zone, parent.as_deref(), &slide)?;
                let removed = list.remove(index);
                Ok(Op::AddElement { slide, zone, parent, index: Some(index), element: removed })
            }
            Op::SetSlideField { slide, field, value } => {
                let s = slide_mut(deck, &slide)?;
                let old = match field {
                    SlideField::Kind => to_value(&std::mem::replace(&mut s.kind, typed::<SlideKind>("kind", value)?)),
                    SlideField::Layout => {
                        to_value(&std::mem::replace(&mut s.layout, typed::<Option<SlideLayout>>("layout", value)?))
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
                Ok(Op::SetSlideField { slide, field, value: old })
            }
            Op::AddSlide { index, slide } => {
                if deck.slide(&slide.id).is_some() {
                    return Err(OpError::DuplicateId(slide.id));
                }
                let at = index.unwrap_or(deck.slides.len());
                if at > deck.slides.len() {
                    return Err(OpError::IndexOutOfRange { index: at, len: deck.slides.len() });
                }
                // La slide entre dans le deck ; l'inverse n'en garde que l'identifiant.
                let id = slide.id.clone();
                deck.slides.insert(at, slide);
                Ok(Op::RemoveSlide { slide: id })
            }
            Op::RemoveSlide { slide } => {
                let i = deck.slide_index(&slide).ok_or(OpError::SlideNotFound(slide))?;
                let removed = deck.slides.remove(i);
                Ok(Op::AddSlide { index: Some(i), slide: removed })
            }
            Op::MoveSlide { slide, index } => {
                let len = deck.slides.len();
                let i = deck.slide_index(&slide).ok_or(OpError::SlideNotFound(slide.clone()))?;
                if index >= len {
                    return Err(OpError::IndexOutOfRange { index, len });
                }
                let s = deck.slides.remove(i);
                deck.slides.insert(index, s);
                Ok(Op::MoveSlide { slide, index: i })
            }
            Op::SetNotes { slide, notes } => {
                let s = slide_mut(deck, &slide)?;
                let old = std::mem::replace(&mut s.notes, notes);
                Ok(Op::SetNotes { slide, notes: old })
            }
            Op::SetTheme { theme } => Ok(Op::SetTheme { theme: std::mem::replace(&mut deck.theme, theme) }),
            Op::SetTemplates { templates } => {
                Ok(Op::SetTemplates { templates: std::mem::replace(&mut deck.templates, templates) })
            }
            Op::SetToken { path, value } => {
                if path.is_empty() {
                    return Err(OpError::InvalidValue { field: "path".into(), reason: "chemin vide".into() });
                }
                let old = if value.is_null() {
                    let removed = remove_path(&mut deck.tokens, &path);
                    if deck.tokens.as_object().is_some_and(Map::is_empty) {
                        deck.tokens = Value::Null;
                    }
                    removed
                } else {
                    set_path(&mut deck.tokens, &path, value)
                };
                Ok(Op::SetToken { path, value: old.unwrap_or(Value::Null) })
            }
            Op::SetParam { key, value } => {
                let p = &mut deck.params;
                let old = match key.as_str() {
                    "title" => to_value(&std::mem::replace(&mut p.title, opt_string(&key, value)?)),
                    "subtitle" => to_value(&std::mem::replace(&mut p.subtitle, opt_string(&key, value)?)),
                    "author" => to_value(&std::mem::replace(&mut p.author, opt_string(&key, value)?)),
                    "date" => to_value(&std::mem::replace(&mut p.date, opt_string(&key, value)?)),
                    "organization" => to_value(&std::mem::replace(&mut p.organization, opt_string(&key, value)?)),
                    "lang" => to_value(&std::mem::replace(&mut p.lang, opt_string(&key, value)?)),
                    _ => set_or_remove_map(&mut p.extra, &key, value),
                };
                Ok(Op::SetParam { key, value: old })
            }
            Op::SetVar { key, value } => {
                let old = set_or_remove_map(&mut deck.vars, &key, value);
                Ok(Op::SetVar { key, value: old })
            }
        }
    }
}

/// Applique une suite d'ops de façon atomique : en cas d'erreur, les ops
/// déjà appliquées sont annulées. Renvoie les inverses, dans l'ordre où il
/// faut les appliquer pour tout défaire.
pub fn apply_all(deck: &mut Deck, ops: Vec<Op>) -> Result<Vec<Op>, (usize, OpError)> {
    let mut inverses: Vec<Op> = Vec::with_capacity(ops.len());
    for (i, op) in ops.into_iter().enumerate() {
        match op.apply(deck) {
            Ok(inv) => inverses.push(inv),
            Err(e) => {
                for inv in inverses.into_iter().rev() {
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
