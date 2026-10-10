//! Lecture : conteneurs Loro → JSON du deck, puis JSON → `Deck` en lecture
//! totale (seul un deck d'une autre version fait échouer [`deck`] ;
//! l'illisible est écarté et signalé par une [`Issue`]).
//!
//! La lecture se passe du registre des widgets : le type d'un paramètre se
//! lit dans son conteneur. L'historique, la fusion et l'aperçu d'une version
//! passée ne dépendent donc pas du pack installé.

use loro::{Container, LoroMap, LoroMovableList, LoroText, LoroTree, LoroValue, TreeParentId, ValueOrContainer};
use oku_slides_i18n::Message;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::DeckRead;
use super::schema::{Ctx, DATASET, Kind, LIST, RICH};
use super::tree::{Children, ordered_children, slide_order, stray_roots};
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
pub(super) fn loro_to_json_at(v: &LoroValue, depth: u32) -> Value {
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

/// Une carte `{list: LoroMovableList}` : les items d'un paramètre `list`.
fn list_items(m: &LoroMap) -> Option<LoroMovableList> {
    if m.len() != 1 {
        return None;
    }
    match m.get(LIST) {
        Some(ValueOrContainer::Container(Container::MovableList(l))) => Some(l),
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
            if ctx.reserved(&key) {
                continue;
            }
            let Some(entry) = map.get(&key) else { continue };
            // Le marqueur `{columns, rows}` ne vaut que sous `dataset` d'un hôte.
            if ctx == Ctx::Host
                && key == DATASET
                && let ValueOrContainer::Container(Container::Map(m)) = &entry
                && let Some(ds) = super::host::read_dataset(m, depth.deeper().levels, &mut self.issues)
            {
                out.insert(key, ds);
                continue;
            }
            let value = match (ctx.kind(&key), entry) {
                // `slides` est obligatoire, même vide.
                (Kind::Slides, ValueOrContainer::Container(Container::Tree(t))) => {
                    Value::Array(self.read_deck_tree(&t, depth.deeper()))
                }
                // Refusé à l'entrée ; la lecture seule le saute, `materialize` pose `slides: []`.
                (Kind::Slides, _) => continue,
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

    /// Les slots des enfants visibles `kids`, par nom (voir les règles de lecture).
    ///
    /// `depth.slots` est la profondeur des nœuds lus (0 pour les racines).
    /// Un élément illisible est écarté avec son sous-arbre ; un élément au
    /// slot réparé est gardé et signalé ; des enfants visibles au-delà de
    /// [`MAX_SLOT_DEPTH`] ne sont pas lus (une `Issue` par nœud). Les enfants
    /// de chaque nœud sont calculés ici, avec son `TreeID` comme `pid`.
    fn read_tree(&mut self, tree: &LoroTree, kids: &Children, depth: Depth) -> Map<String, Value> {
        let mut out = Map::new();
        for (slot, children) in &kids.slots {
            let mut items = Vec::with_capacity(children.len());
            for child in children {
                let Some(meta) = &child.meta else {
                    self.issues.push(issue(None, None, Message::new("issue-unreadable-element")));
                    continue;
                };
                let mut obj = self.read_map_at(meta, Ctx::Element, Depth { levels: depth.levels + 2, ..depth });
                let id = obj.get("id").and_then(Value::as_str).map(str::to_owned);
                if Element::deserialize(&Value::Object(obj.clone())).is_err() {
                    self.issues.push(issue(None, id, Message::new("issue-unreadable-element")));
                    continue;
                }
                if child.repaired {
                    let message = Message::new("issue-slot-repaired").arg("slot", slot.clone());
                    self.issues.push(issue(None, id.clone(), message));
                }
                let grand = ordered_children(tree, TreeParentId::Node(child.node), &child.node.to_string());
                let count = grand.len();
                if count > 0 && depth.slots + 1 >= MAX_SLOT_DEPTH {
                    for _ in 0..count {
                        self.issues.push(issue(None, None, Message::new("issue-slot-too-deep")));
                    }
                } else if count > 0 {
                    let inner = Depth { levels: depth.levels + 2, slots: depth.slots + 1 };
                    let slots = self.read_tree(tree, &grand, inner);
                    for param in merge_slots(&mut obj, slots) {
                        let message = Message::new("issue-param-type").arg("param", param);
                        self.issues.push(issue(None, id.clone(), message));
                    }
                }
                obj.sort_keys();
                items.push(Value::Object(obj));
            }
            if !items.is_empty() {
                out.insert(slot.clone(), Value::Array(items));
            }
        }
        out
    }

    /// Paramètres d'un widget, sans registre : le type se lit dans le
    /// conteneur. Un texte, un texte riche, une liste vides sont absents.
    /// Les slots ne sont pas ici : ils viennent de l'arbre ([`Self::read_tree`]).
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
                ValueOrContainer::Container(Container::Map(m)) if let Some(l) = list_items(&m) => {
                    let items = self.read_items(&l, depth.deeper());
                    if items.is_empty() {
                        continue;
                    }
                    Value::Array(items)
                }
                ValueOrContainer::Container(Container::Map(m)) => match rich_text(&m) {
                    Some(t) => {
                        let paragraphs = super::rich::read(&t);
                        if paragraphs.is_empty() {
                            continue;
                        }
                        Value::Array(paragraphs)
                    }
                    // Option d'un hôte : son `dataset` peut porter le marqueur.
                    None => Value::Object(self.read_map_at(&m, Ctx::Host, depth.deeper())),
                },
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

    /// Les items d'une liste : chaque carte relue comme des paramètres (son
    /// `id` en valeur). Un item sans `id` chaîne, ou qui porte un `kind`
    /// (il se lirait comme un élément), est écarté et signalé.
    fn read_items(&mut self, list: &LoroMovableList, depth: Depth) -> Vec<Value> {
        let mut out = Vec::with_capacity(list.len());
        for i in 0..list.len() {
            let Some(ValueOrContainer::Container(Container::Map(m))) = list.get(i) else { continue };
            let item = self.read_params(&m, depth.deeper());
            if !item.get("id").is_some_and(Value::is_string) || item.contains_key("kind") {
                self.issues.push(issue(None, None, Message::new("issue-unreadable-item")));
                continue;
            }
            out.push(Value::Object(item));
        }
        out
    }

    /// Les slides de l'arbre du deck, dans l'ordre de `slide_order` (R8).
    ///
    /// Une racine non vide sans `@slide = true` est écartée et signalée. Les
    /// `Issue` des éléments d'une slide reçoivent son id.
    fn read_deck_tree(&mut self, tree: &LoroTree, depth: Depth) -> Vec<Value> {
        for stray in stray_roots(tree) {
            let id = tree.get_meta(stray).ok().and_then(|m| match m.get("id") {
                Some(ValueOrContainer::Value(LoroValue::String(s))) => Some(s.to_string()),
                _ => None,
            });
            self.issues.push(issue(id, None, Message::new("issue-unreadable-slide")));
        }
        let order = slide_order(tree);
        let mut out = Vec::with_capacity(order.len());
        for node in order {
            // Listé par `slide_order`, qui vient d'en lire la méta.
            let Ok(meta) = tree.get_meta(node) else { continue };
            let first = self.issues.len();
            let mut obj = self.read_map_at(&meta, Ctx::Slide, depth);
            let kids = ordered_children(tree, TreeParentId::Node(node), &node.to_string());
            let slots = self.read_tree(tree, &kids, Depth { levels: depth.levels + 1, slots: 0 });
            for param in merge_slots(&mut obj, slots) {
                self.issues.push(issue(None, None, Message::new("issue-param-type").arg("param", param)));
            }
            obj.sort_keys();
            let id = obj.get("id").and_then(Value::as_str).unwrap_or_default();
            for i in &mut self.issues[first..] {
                i.slide.get_or_insert_with(|| id.to_owned());
            }
            out.push(Value::Object(obj));
        }
        out
    }
}

/// Verse `slots` dans `obj["params"]`, créé au besoin, puis trie ses clés.
/// Un slot l'emporte sur un paramètre de même nom, dont le nom est rendu.
fn merge_slots(obj: &mut Map<String, Value>, slots: Map<String, Value>) -> Vec<String> {
    if slots.is_empty() {
        return Vec::new();
    }
    let params = obj.entry("params").or_insert_with(|| Value::Object(Map::new()));
    if !params.is_object() {
        *params = Value::Object(Map::new());
    }
    let mut clashes = Vec::new();
    if let Value::Object(params) = params {
        clashes.extend(slots.keys().filter(|k| params.contains_key(*k)).cloned());
        params.extend(slots);
        params.sort_keys();
    }
    clashes
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
        Some(Value::Array(a)) => {
            let mut slides: Vec<Slide> = a.into_iter().filter_map(|s| read_slide(s, &mut issues)).collect();
            // Même réparation que la forme canonique : la vue égale la relecture (t-652).
            crate::canonical::unique_ids(&mut slides);
            slides
        }
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
