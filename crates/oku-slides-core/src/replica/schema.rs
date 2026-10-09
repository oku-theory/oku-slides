//! Catégorie de chaque champ du deck dans le replica, selon son contexte.
//!
//! Les champs fixes (deck, slide, élément) suivent une table statique. Les
//! paramètres d'un widget (`params` d'une slide ou d'un élément) prennent le
//! type que déclare le registre ([`param_kind`]), sinon celui qui se lit dans
//! la forme de leur valeur JSON.

use serde_json::Value;

use crate::params::{is_elements, is_items, is_paragraphs};
use crate::widget::{ParamSchema, ParamType};

/// Où l'on se trouve dans le deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ctx {
    Deck,
    /// `params` du deck (titre, sous-titre… et extras).
    DeckParams,
    Slide,
    Element,
    /// `params` d'une slide ou d'un élément : voir [`param_kind`].
    Params,
    /// Chaque variable de `vars` est une feuille LWW, quelle que soit sa forme
    /// JSON (spec : chaque entrée de `vars` est LWW, jamais fusionnée champ à
    /// champ même si elle contient un objet).
    Vars,
    /// `tokens`, `style`, `frame`, `cell`, extras des `params` du deck :
    /// cartes à feuilles LWW.
    Opaque,
    /// Option d'un paramètre `host`, lue comme [`Ctx::Opaque`] ; sa clé
    /// `dataset` peut porter des colonnes et des lignes à identifiant.
    Host,
}

/// Ce que devient un champ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// `LoroText`, fusion au caractère.
    Prose,
    /// `LoroMap { rich: LoroText à marques }` (paragraphes).
    Rich,
    /// `LoroMovableList` de `LoroMap` à `id`.
    Collection(Ctx),
    /// `LoroMap { list: LoroMovableList de LoroMap à id }` : items d'un
    /// paramètre `list`, lisibles sans registre.
    List,
    /// Une valeur JSON entière, LWW.
    Atom,
    /// Objet → `LoroMap` ; autre valeur → LWW.
    Nested(Ctx),
    /// `LoroMap` ; `dataset` en colonnes et lignes à identifiant.
    Host,
}

/// Clé de la carte d'un paramètre `rich`, qui porte son `LoroText`.
pub(super) const RICH: &str = "rich";

/// Clé de la carte d'un paramètre `list`, qui porte sa `LoroMovableList`.
pub(super) const LIST: &str = "list";

/// Clé de l'option d'un hôte qui porte ses données.
pub(super) const DATASET: &str = "dataset";

/// Clé de la carte d'un `dataset` qui porte ses identifiants de colonnes.
pub(super) const COLUMNS: &str = "columns";

/// Clé de la carte d'un `dataset` qui porte ses lignes.
pub(super) const ROWS: &str = "rows";

/// Clé d'une ligne qui porte ses cellules, par identifiant de colonne.
pub(super) const CELLS: &str = "cells";

impl Ctx {
    /// Catégorie d'un champ fixe. `Ctx::Params` ne passe pas par ici : voir
    /// [`param_kind`].
    pub(super) fn kind(self, key: &str) -> Kind {
        use Ctx::*;
        match (self, key) {
            // Une référence de pack ou de thème entière : l'id et la version ne se mêlent jamais.
            (Deck, "templates" | "theme") => Kind::Atom,
            (Deck, "slides") => Kind::Collection(Slide),
            (Deck, "params") => Kind::Nested(DeckParams),
            (Deck, "vars") => Kind::Nested(Vars),
            (Vars, _) => Kind::Atom,
            (DeckParams, "title" | "subtitle" | "author" | "date" | "organization") => Kind::Prose,
            (Slide, "name" | "description" | "notes") => Kind::Prose,
            (Slide | Element, "params") => Kind::Nested(Params),
            (Slide | Element, "id" | "kind") => Kind::Atom,
            _ => Kind::Nested(Opaque),
        }
    }

    /// Textes créés dès la création de l'objet (spec : « toujours présents »).
    /// Les paramètres créés d'avance viennent du registre ([`declared`]).
    pub(super) fn texts(self) -> &'static [&'static str] {
        match self {
            Ctx::DeckParams => &["title", "subtitle", "author", "date", "organization"],
            Ctx::Slide => &["name", "description", "notes"],
            Ctx::Deck | Ctx::Element | Ctx::Params | Ctx::Vars | Ctx::Opaque | Ctx::Host => &[],
        }
    }

    /// Collections créées dès la création de l'objet.
    pub(super) fn lists(self) -> &'static [&'static str] {
        match self {
            Ctx::Deck => &["slides"],
            _ => &[],
        }
    }
}

/// Catégorie d'un paramètre de type déclaré `ty`. `choice` et `asset` sont
/// atomiques (dernière écriture gagnante) : fusionner deux valeurs au
/// caractère en donnerait une invalide (voir « Décisions » du plan 2).
fn declared_kind(ty: &ParamType) -> Kind {
    match ty {
        ParamType::Plain => Kind::Prose,
        ParamType::Rich => Kind::Rich,
        ParamType::Slot { .. } => Kind::Collection(Ctx::Element),
        ParamType::List { .. } => Kind::List,
        ParamType::Host { .. } => Kind::Host,
        _ => Kind::Atom,
    }
}

/// Catégorie par la forme de la valeur, comme [`ParamValue`](crate::ParamValue).
fn shape_kind(value: &Value) -> Kind {
    match value {
        Value::String(_) => Kind::Prose,
        Value::Array(a) if is_elements(a) => Kind::Collection(Ctx::Element),
        Value::Array(a) if is_items(a) => Kind::List,
        Value::Array(a) if is_paragraphs(a) => Kind::Rich,
        _ => Kind::Atom,
    }
}

/// La valeur tient-elle dans la catégorie `kind` sans rien perdre ?
fn fits(kind: Kind, value: &Value) -> bool {
    match (kind, value) {
        (Kind::Prose, Value::String(_)) | (Kind::Atom, _) => true,
        (Kind::Rich, Value::Array(a)) => is_paragraphs(a),
        (Kind::Collection(_), Value::Array(a)) => a.is_empty() || is_elements(a),
        (Kind::List, Value::Array(a)) => a.is_empty() || is_items(a),
        (Kind::Host, Value::Object(_)) => true,
        _ => false,
    }
}

/// Catégorie du paramètre `name` de valeur `value` d'un widget aux
/// paramètres `params` : le type déclaré s'il y en a un et que la valeur y
/// tient, sinon la forme de la valeur (paramètre inconnu, widget inconnu,
/// valeur d'un autre type : rien ne se perd, `validate` le signale).
pub(super) fn param_kind(params: Option<&[ParamSchema]>, name: &str, value: &Value) -> Kind {
    params
        .and_then(|ps| ps.iter().find(|p| p.name == name))
        .map(|p| declared_kind(&p.ty))
        .filter(|k| fits(*k, value))
        .unwrap_or_else(|| shape_kind(value))
}

/// Catégorie de la colonne `name` d'un item aux colonnes `columns`, comme
/// un paramètre ; mais une colonne n'est jamais une collection (liste imbriquée : t-495), et l'`id`
/// d'un item est une valeur, clé de sa réconciliation.
pub(super) fn column_kind(columns: Option<&[ParamSchema]>, name: &str, value: &Value) -> Kind {
    match param_kind(columns, name, value) {
        _ if name == "id" => Kind::Atom,
        Kind::Collection(_) | Kind::List | Kind::Host => Kind::Atom,
        kind => kind,
    }
}

/// Colonnes d'un paramètre `list` déclaré parmi `params`.
pub(super) fn columns<'a>(params: Option<&'a [ParamSchema]>, name: &str) -> Option<&'a [ParamSchema]> {
    match &params?.iter().find(|p| p.name == name)?.ty {
        ParamType::List { item, .. } => Some(item.as_slice()),
        _ => None,
    }
}

/// Champs à conteneur de `params` (`plain`, `rich`, `slot`, `list`), créés
/// d'avance : deux premières éditions concurrentes visent alors le même
/// conteneur et fusionnent (parade t-188).
pub(super) fn declared(params: Option<&[ParamSchema]>) -> impl Iterator<Item = (&str, Kind)> {
    params
        .into_iter()
        .flatten()
        .map(|p| (p.name.as_str(), declared_kind(&p.ty)))
        .filter(|(_, k)| !matches!(k, Kind::Atom))
}
