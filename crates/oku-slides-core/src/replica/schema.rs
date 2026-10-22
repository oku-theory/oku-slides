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
    /// Chaque entrée de `widgets` et de `layouts` est une feuille LWW : un
    /// épinglage entier, jamais champ à champ.
    PackPins,
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
    /// Un `LoroTree` : slides en racine, éléments en descendants (ADR-0006).
    Slides,
    /// Éléments d'un slot : nœuds de l'arbre du deck (`tree.rs`).
    Slot,
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

/// Clé de la carte `deck` qui porte l'arbre du deck.
pub(super) const SLIDES: &str = "slides";

/// Clé de la méta d'une racine : `true`, la marque d'une slide.
pub(super) const SLIDE: &str = "@slide";

/// Clé de la méta d'un nœud : slot par `TreeID` de parent (`counter@peer`).
pub(super) const SLOT: &str = "@slot";

/// Préfixe réservé des marques de suppression : `@deleted/<pair>`.
pub(super) const DELETED: &str = "@deleted";

/// La marque de suppression du pair `peer`.
pub(super) fn mark_key(peer: u64) -> String {
    format!("{DELETED}/{peer}")
}

/// Le pair d'une marque bien formée, `None` sinon (forme canonique exigée).
pub(super) fn mark_peer(key: &str) -> Option<u64> {
    let digits = key.strip_prefix(DELETED)?.strip_prefix('/')?;
    let peer: u64 = digits.parse().ok()?;
    (peer.to_string() == digits).then_some(peer)
}

/// Slot de repli d'un élément sans slot lisible.
pub(super) const ORPHANS: &str = "orphans";

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
            // Une référence de thème entière : l'id et la version ne se mêlent jamais.
            (Deck, "theme") => Kind::Atom,
            (Deck, "widgets" | "layouts") => Kind::Nested(PackPins),
            (PackPins, _) => Kind::Atom,
            (Deck, SLIDES) => Kind::Slides,
            (Deck, "params") => Kind::Nested(DeckParams),
            (Deck, "vars") => Kind::Nested(Vars),
            (Vars, _) => Kind::Atom,
            (DeckParams, "title" | "subtitle" | "author" | "date" | "organization") => Kind::Prose,
            (Slide, "notes") => Kind::Prose,
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
            Ctx::Slide => &["notes"],
            Ctx::Deck | Ctx::Element | Ctx::Params | Ctx::Vars | Ctx::PackPins | Ctx::Opaque | Ctx::Host => &[],
        }
    }

    /// Cartes créées dès la création de l'objet et jamais retirées : vides
    /// quand le deck n'en a pas. Deux pairs qui épinglent chacun un premier
    /// pack écrivent ainsi dans la même carte `widgets` (ou `layouts`).
    pub(super) fn maps(self) -> &'static [&'static str] {
        match self {
            Ctx::Deck => &["widgets", "layouts"],
            _ => &[],
        }
    }

    /// Arbres créés dès la création de l'objet : l'arbre du deck.
    pub(super) fn trees(self) -> &'static [&'static str] {
        match self {
            Ctx::Deck => &[SLIDES],
            _ => &[],
        }
    }

    /// Vrai pour une clé du replica, jamais lue ni écrite comme un champ :
    /// dans la méta d'une slide ou d'un élément, `SLOT`, `SLIDE` et le préfixe `DELETED`.
    pub(super) fn reserved(self, key: &str) -> bool {
        match self {
            Ctx::Slide | Ctx::Element => key == SLOT || key == SLIDE || key.starts_with(DELETED),
            _ => false,
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
        ParamType::Slot { .. } => Kind::Slot,
        ParamType::List { .. } => Kind::List,
        ParamType::Host { .. } => Kind::Host,
        _ => Kind::Atom,
    }
}

/// Catégorie par la forme de la valeur, comme [`ParamValue`](crate::ParamValue).
fn shape_kind(value: &Value) -> Kind {
    match value {
        Value::String(_) => Kind::Prose,
        Value::Array(a) if is_elements(a) => Kind::Slot,
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
        (Kind::Slot, Value::Array(a)) => a.is_empty() || is_elements(a),
        (Kind::List, Value::Array(a)) => a.is_empty() || is_items(a),
        (Kind::Host, Value::Object(_)) => true,
        _ => false,
    }
}

/// Catégorie du paramètre `name` de valeur `value` d'un widget aux
/// paramètres `params` : le type déclaré s'il y en a un et que la valeur y
/// tient, sinon la forme de la valeur (paramètre inconnu, widget inconnu,
/// valeur d'un autre type : rien ne se perd, `validate` le signale).
///
/// Un nom vide n'est jamais un slot : sa clé `@slot` ne se relirait pas.
pub(super) fn param_kind(params: Option<&[ParamSchema]>, name: &str, value: &Value) -> Kind {
    let kind = params
        .and_then(|ps| ps.iter().find(|p| p.name == name))
        .map(|p| declared_kind(&p.ty))
        .filter(|k| fits(*k, value))
        .unwrap_or_else(|| shape_kind(value));
    match kind {
        Kind::Slot if name.is_empty() => Kind::Atom,
        kind => kind,
    }
}

/// Catégorie de la colonne `name` d'un item aux colonnes `columns`, comme
/// un paramètre ; une colonne n'est jamais un slot ni un hôte, et n'est une liste
/// (conteneur, liste imbriquée t-495) que si `columns` la déclare `list` : une valeur en
/// forme d'items d'une colonne non déclarée reste une valeur LWW (un seul niveau, et une
/// valeur déjà écrite n'est jamais remplacée par un conteneur, parade t-188). L'`id` d'un
/// item est une valeur, clé de sa réconciliation.
pub(super) fn column_kind(columns: Option<&[ParamSchema]>, name: &str, value: &Value) -> Kind {
    let declared_list = columns
        .and_then(|cs| cs.iter().find(|c| c.name == name))
        .is_some_and(|c| matches!(c.ty, ParamType::List { .. }));
    match param_kind(columns, name, value) {
        _ if name == "id" => Kind::Atom,
        Kind::List if declared_list => Kind::List,
        Kind::Slides | Kind::Slot | Kind::List | Kind::Host => Kind::Atom,
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

/// Champs à conteneur de `params` (`plain`, `rich`, `list`), créés d'avance :
/// deux premières éditions concurrentes visent alors le même conteneur et
/// fusionnent (parade t-188). Un slot vit dans l'arbre du deck, sous le nœud
/// de son propriétaire.
pub(super) fn declared(params: Option<&[ParamSchema]>) -> impl Iterator<Item = (&str, Kind)> {
    params
        .into_iter()
        .flatten()
        .map(|p| (p.name.as_str(), declared_kind(&p.ty)))
        .filter(|(_, k)| !matches!(k, Kind::Atom | Kind::Slot))
}
