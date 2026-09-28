//! Catégorie de chaque champ du deck dans le replica, selon son contexte.
//!
//! Les champs fixes (deck, slide, élément) suivent une table statique. Les
//! paramètres d'un widget (`params` d'une slide ou d'un élément) prennent le
//! type que déclare le registre ([`param_kind`]), sinon celui qui se lit dans
//! la forme de leur valeur JSON.

use serde_json::Value;

use crate::params::{is_elements, is_paragraphs};
use crate::widget::{ParamType, WidgetSchema};

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
    /// Une valeur JSON entière, LWW.
    Atom,
    /// Objet → `LoroMap` ; autre valeur → LWW.
    Nested(Ctx),
}

/// Clé de la carte d'un paramètre `rich`, qui porte son `LoroText`.
pub(super) const RICH: &str = "rich";

impl Ctx {
    /// Catégorie d'un champ fixe. `Ctx::Params` ne passe pas par ici : voir
    /// [`param_kind`].
    pub(super) fn kind(self, key: &str) -> Kind {
        use Ctx::*;
        match (self, key) {
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
            Ctx::Deck | Ctx::Element | Ctx::Params | Ctx::Vars | Ctx::Opaque => &[],
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
        _ => Kind::Atom,
    }
}

/// Catégorie par la forme de la valeur, comme [`ParamValue`](crate::ParamValue).
fn shape_kind(value: &Value) -> Kind {
    match value {
        Value::String(_) => Kind::Prose,
        Value::Array(a) if is_elements(a) => Kind::Collection(Ctx::Element),
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
        _ => false,
    }
}

/// Catégorie du paramètre `name` de valeur `value` : le type déclaré par
/// `schema` s'il y en a un et que la valeur y tient, sinon la forme de la
/// valeur (paramètre inconnu, widget inconnu, valeur d'un autre type : rien
/// ne se perd, `validate` le signale).
pub(super) fn param_kind(schema: Option<&WidgetSchema>, name: &str, value: &Value) -> Kind {
    schema
        .and_then(|w| w.param(name))
        .map(|p| declared_kind(&p.ty))
        .filter(|k| fits(*k, value))
        .unwrap_or_else(|| shape_kind(value))
}

/// Paramètres de `schema` à conteneur (`plain`, `rich`, `slot`), créés
/// d'avance : deux premières éditions concurrentes visent alors le même
/// conteneur et fusionnent (parade t-188).
pub(super) fn declared(schema: Option<&WidgetSchema>) -> impl Iterator<Item = (&str, Kind)> {
    schema
        .into_iter()
        .flat_map(|w| w.params.iter())
        .map(|p| (p.name.as_str(), declared_kind(&p.ty)))
        .filter(|(_, k)| !matches!(k, Kind::Atom))
}
