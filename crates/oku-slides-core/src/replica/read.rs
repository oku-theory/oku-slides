//! Lecture : conteneurs Loro → JSON du deck, puis JSON → `Deck` en lecture
//! totale (seul un deck d'une autre version fait échouer [`deck`] ;
//! l'illisible est écarté et signalé par une [`Issue`]).
//!
//! La lecture se passe du registre des widgets : le type d'un paramètre se
//! lit dans son conteneur. L'historique, la fusion et l'aperçu d'une version
//! passée ne dépendent donc pas du pack installé.

use loro::{Container, LoroMap, LoroMovableList, LoroText, LoroValue, ValueOrContainer};
use oku_slides_i18n::Message;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::DeckRead;
use super::schema::{Ctx, Kind, RICH};
use crate::error::{ReplicaInner, UnsupportedDeckVersionSnafu};
use crate::locate::MAX_SLOT_DEPTH;
use crate::model::{DECK_VERSION, Deck, Element, Slide};
use crate::validate::Issue;

/// Profondeur maximale explorée dans un replica : au-delà, le sous-arbre est
/// ignoré. Un blob non fiable peut imbriquer arbitrairement des cartes ;
/// sans cette borne, la récursion de la lecture pourrait épuiser la pile
/// (un `catch_unwind` ne rattrape pas un débordement de pile).
pub(super) const MAX_DEPTH: u32 = 64;

/// Point d'entrée : profondeur 0 (utilisé pour une valeur autonome, par
/// exemple une marque de texte riche).
pub(super) fn loro_to_json(v: &LoroValue) -> Value {
    loro_to_json_at(v, 0)
}

/// Une `LoroValue` peut elle-même imbriquer listes et cartes sans passer par
/// un conteneur (`LoroValue::List`/`LoroValue::Map`) : un blob non fiable
/// peut donc faire déborder la pile ici aussi si on ne borne pas cette
/// récursion, séparée de celle de la lecture des conteneurs.
fn loro_to_json_at(v: &LoroValue, depth: u32) -> Value {
    if depth >= MAX_DEPTH {
        return Value::Null;
    }
    match v {
        LoroValue::Null | LoroValue::Binary(_) | LoroValue::Container(_) => Value::Null,
        LoroValue::Bool(b) => Value::Bool(*b),
        LoroValue::Double(d) => serde_json::json!(d),
        LoroValue::I64(i) => serde_json::json!(i),
        LoroValue::String(s) => Value::String(s.to_string()),
        LoroValue::List(l) => Value::Array(l.iter().map(|v| loro_to_json_at(v, depth + 1)).collect()),
        LoroValue::Map(m) => {
            let mut out: Map<String, Value> =
                m.iter().map(|(k, v)| (k.to_string(), loro_to_json_at(v, depth + 1))).collect();
            // Clés triées, comme la forme canonique (`canonical::numbers`).
            out.sort_keys();
            Value::Object(out)
        }
    }
}

/// Lit le deck de la carte racine `root` : lecture totale, sauf pour un deck
/// d'une autre version que [`DECK_VERSION`], refusé.
pub(super) fn deck(root: &LoroMap) -> Result<DeckRead, ReplicaInner> {
    let mut reader = Reader::default();
    let json = reader.read_map_at(root, Ctx::Deck, Depth::default());
    materialize(json, reader.issues)
}

/// Lecture des conteneurs ; garde les éléments illisibles écartés.
#[derive(Default)]
struct Reader {
    issues: Vec<Issue>,
}

/// Où en est la descente.
#[derive(Debug, Clone, Copy, Default)]
struct Depth {
    /// Niveaux de conteneurs et de valeurs, bornés par [`MAX_DEPTH`].
    levels: u32,
    /// Profondeur (`Located::depth`) des éléments des slots lus ici : 0
    /// sous une slide. Bornée par [`MAX_SLOT_DEPTH`], comme les ops.
    slots: usize,
}

impl Depth {
    fn deeper(self) -> Self {
        Self { levels: self.levels + 1, ..self }
    }
}

/// Une carte `{rich: LoroText}` : le texte d'un paramètre `rich`.
fn rich_text(m: &LoroMap) -> Option<LoroText> {
    if m.len() != 1 {
        return None;
    }
    match m.get(RICH) {
        Some(ValueOrContainer::Container(Container::Text(t))) => Some(t),
        _ => None,
    }
}

/// Texte non blanc ; vide = absent.
fn prose(t: &LoroText) -> Option<Value> {
    let s = t.to_string();
    (!s.trim().is_empty()).then_some(Value::String(s))
}

impl Reader {
    fn read_map_at(&mut self, map: &LoroMap, ctx: Ctx, depth: Depth) -> Map<String, Value> {
        let mut out = Map::new();
        // Bornée : un replica non fiable peut imbriquer des cartes sans limite ;
        // au-delà, on ignore le sous-arbre plutôt que de récurser indéfiniment.
        if depth.levels >= MAX_DEPTH {
            return out;
        }
        for key in map.keys() {
            let key = key.to_string();
            let Some(entry) = map.get(&key) else { continue };
            let value = match (ctx.kind(&key), entry) {
                (Kind::Nested(Ctx::Params), ValueOrContainer::Container(Container::Map(m))) => {
                    let params = self.read_params(&m, depth.deeper());
                    if params.is_empty() {
                        continue;
                    }
                    Value::Object(params)
                }
                (_, ValueOrContainer::Container(Container::Text(t))) => match prose(&t) {
                    Some(v) => v,
                    None => continue,
                },
                (Kind::Collection(child), ValueOrContainer::Container(Container::MovableList(l))) => {
                    let items = self.read_list(&l, child, depth.deeper());
                    // `slides` est obligatoire ; les autres collections vides sont omises.
                    if items.is_empty() && ctx != Ctx::Deck {
                        continue;
                    }
                    Value::Array(items)
                }
                (kind, ValueOrContainer::Container(Container::Map(m))) => {
                    let child = match kind {
                        Kind::Nested(c) => c,
                        _ => Ctx::Opaque,
                    };
                    Value::Object(self.read_map_at(&m, child, depth.deeper()))
                }
                (_, ValueOrContainer::Value(v)) => loro_to_json_at(&v, depth.levels + 1),
                // Conteneur d'un type inattendu : ignoré.
                _ => continue,
            };
            out.insert(key, value);
        }
        // Clés triées, comme la forme canonique (`canonical::numbers`).
        out.sort_keys();
        out
    }

    /// Paramètres d'un widget, sans registre : le type se lit dans le
    /// conteneur. Un texte, un texte riche, un slot vides sont absents.
    ///
    /// La profondeur continue de croître à travers le cycle `params` →
    /// liste → élément → `params` : deux niveaux par slot. Un slot dont les
    /// éléments dépasseraient [`MAX_SLOT_DEPTH`] n'est pas lu (signalé) :
    /// les ops ne le créent pas, seul un replica forgé le porte.
    fn read_params(&mut self, map: &LoroMap, depth: Depth) -> Map<String, Value> {
        let mut out = Map::new();
        if depth.levels >= MAX_DEPTH {
            return out;
        }
        for key in map.keys() {
            let key = key.to_string();
            let Some(entry) = map.get(&key) else { continue };
            let value = match entry {
                ValueOrContainer::Container(Container::Text(t)) => match prose(&t) {
                    Some(v) => v,
                    None => continue,
                },
                ValueOrContainer::Container(Container::Map(m)) => match rich_text(&m) {
                    Some(t) => {
                        let paragraphs = super::rich::read(&t);
                        if paragraphs.is_empty() {
                            continue;
                        }
                        Value::Array(paragraphs)
                    }
                    None => Value::Object(self.read_map_at(&m, Ctx::Opaque, depth.deeper())),
                },
                ValueOrContainer::Container(Container::MovableList(l)) if depth.slots >= MAX_SLOT_DEPTH => {
                    if !l.is_empty() {
                        self.issues.push(issue(None, None, Message::new("issue-slot-too-deep")));
                    }
                    continue;
                }
                ValueOrContainer::Container(Container::MovableList(l)) => {
                    let items = self.read_list(&l, Ctx::Element, depth.deeper());
                    if items.is_empty() {
                        continue;
                    }
                    Value::Array(items)
                }
                ValueOrContainer::Value(v) => match loro_to_json_at(&v, depth.levels + 1) {
                    Value::Null => continue,
                    v => v,
                },
                // Conteneur d'un type inattendu : ignoré.
                _ => continue,
            };
            out.insert(key, value);
        }
        out
    }

    /// Les objets d'une collection. Un élément illisible (enfants déjà
    /// triés) est écarté et signalé ; la slide qui le porte est renseignée
    /// à la lecture de la slide.
    fn read_list(&mut self, list: &LoroMovableList, ctx: Ctx, depth: Depth) -> Vec<Value> {
        let mut out = Vec::with_capacity(list.len());
        for i in 0..list.len() {
            let Some(ValueOrContainer::Container(Container::Map(m))) = list.get(i) else { continue };
            let first = self.issues.len();
            // Les slots d'un élément portent des éléments un cran plus bas ;
            // ceux d'une slide, des éléments à la profondeur 0.
            let inner = match ctx {
                Ctx::Element => Depth { slots: depth.slots + 1, ..depth },
                _ => depth,
            };
            let item = Value::Object(self.read_map_at(&m, ctx, inner));
            let id = item.get("id").and_then(Value::as_str).map(str::to_owned);
            match ctx {
                Ctx::Slide => {
                    for i in &mut self.issues[first..] {
                        i.slide.get_or_insert_with(|| id.clone().unwrap_or_default());
                    }
                }
                Ctx::Element if Element::deserialize(&item).is_err() => {
                    self.issues.push(issue(None, id, Message::new("issue-unreadable-element")));
                    continue;
                }
                _ => {}
            }
            out.push(item);
        }
        out
    }
}

fn issue(slide: Option<String>, element: Option<String>, message: Message) -> Issue {
    Issue { slide, element, message }
}

fn read_slide(s: Value, issues: &mut Vec<Issue>) -> Option<Slide> {
    let id = s.get("id").and_then(Value::as_str).unwrap_or_default().to_owned();
    match serde_json::from_value::<Slide>(s) {
        Ok(slide) => Some(slide),
        Err(_) => {
            issues.push(issue(Some(id), None, Message::new("issue-unreadable-slide")));
            None
        }
    }
}

/// Refuse un deck d'une autre version que [`DECK_VERSION`] ; sans version,
/// le deck est de la version courante (défaut de [`Deck`]).
fn check_version(root: &Map<String, Value>) -> Result<(), ReplicaInner> {
    match root.get("version") {
        None => Ok(()),
        Some(v) if v.as_u64() == Some(u64::from(DECK_VERSION)) => Ok(()),
        Some(v) => UnsupportedDeckVersionSnafu { found: v.to_string() }.fail(),
    }
}

/// Lecture totale d'un deck de la version courante : tout état du replica
/// donne un `Deck`. Ce qui ne se lit pas est écarté (élément, slide) ou
/// remis par défaut (champ du deck), et signalé. Un deck d'une autre
/// version est refusé : relu comme un v2, il perdrait des champs en silence.
fn materialize(mut root: Map<String, Value>, mut issues: Vec<Issue>) -> Result<DeckRead, ReplicaInner> {
    check_version(&root)?;
    let slides: Vec<Slide> = match root.remove("slides") {
        Some(Value::Array(a)) => a.into_iter().filter_map(|s| read_slide(s, &mut issues)).collect(),
        _ => Vec::new(),
    };
    let keys: Vec<String> = root.keys().cloned().collect();
    for key in keys {
        let mut probe = Map::new();
        probe.insert("slides".into(), Value::Array(Vec::new()));
        if let Some(v) = root.get(&key) {
            probe.insert(key.clone(), v.clone());
        }
        if serde_json::from_value::<Deck>(Value::Object(probe)).is_err() {
            root.remove(&key);
            issues.push(issue(None, None, Message::new("issue-unreadable-field").arg("field", key)));
        }
    }
    root.insert("slides".into(), Value::Array(Vec::new()));
    let mut deck = serde_json::from_value::<Deck>(Value::Object(root)).unwrap_or_else(|_| Deck::empty());
    deck.slides = slides;
    Ok((deck, issues))
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod read_tests;
