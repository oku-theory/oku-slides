//! Forme canonique d'un deck : celle que relit le replica.
//!
//! La relecture du replica est canonique (runs vides retirés, runs voisins de
//! même format fusionnés, prose blanche absente…). [`canonicalize`] met un
//! `Deck` sous cette forme, pour que la vue gardée par
//! [`SourceBundle`](crate::SourceBundle) soit toujours égale à la relecture de
//! son replica. Appelée par [`apply_all`](crate::apply_all) et
//! [`SourceBundle::new`](crate::SourceBundle::new).

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use serde_json::{Number, Value};

use crate::model::{Deck, Element, Paragraph, Run, Slide};
use crate::params::{ListItem, ParamValue, Params};
use crate::replica::PARAGRAPH_SEPARATOR;

/// Met `deck` sous forme canonique :
///
/// - prose facultative faite d'espaces → `None`, autre prose rognée (même
///   règle que l'op `set_deck_param`) ;
/// - paramètres des slides et des éléments : texte simple rogné (vide →
///   retiré), texte riche, slot et liste vides retirés, valeurs des items canonisées
///   comme des paramètres, `null` retiré ;
/// - notes faites d'espaces → `""` (les autres gardent leurs bords) ;
/// - prose riche : `U+2029` dans un run → `\n` (le contenu est gardé, sans
///   nouveau paragraphe), runs vides retirés, runs voisins de même format fusionnés ;
/// - entier JSON au-delà d'`i64` (`vars`, extras de `params`, `tokens`,
///   valeurs des paramètres) →
///   flottant, comme le stocke Loro ; `tokens` objet vide → `null` ;
/// - un identifiant par slide, par élément d'une slide et par item d'une
///   liste ([`unique_ids`], la réparation que fait aussi la relecture).
pub fn canonicalize(deck: &mut Deck) {
    let p = &mut deck.params;
    for field in [&mut p.title, &mut p.subtitle, &mut p.author, &mut p.date, &mut p.organization] {
        prose(field);
    }
    p.extra.values_mut().for_each(numbers);
    deck.vars.values_mut().for_each(numbers);
    numbers(&mut deck.tokens);
    if deck.tokens.as_object().is_some_and(serde_json::Map::is_empty) {
        deck.tokens = Value::Null;
    }
    for s in &mut deck.slides {
        if s.notes.trim().is_empty() {
            s.notes.clear();
        }
        params(&mut s.params);
    }
    unique_ids(&mut deck.slides);
}

/// Un identifiant par objet : slides du deck, éléments de chaque slide (tous
/// slots confondus, comme `validate`), items de chaque liste.
///
/// Une copie exacte (à l'identifiant près, même identifiant d'origine) d'un
/// objet déjà vu de la même collection est écartée ; toute autre occurrence
/// d'un identifiant déjà gardé est renommée `{id}-{n}`, au premier `n ≥ 2`
/// libre. Un identifiant vide n'est pas touché (`issue-empty-*-id`).
///
/// Deux objets au même id naissent d'une annulation. Défaire la suppression
/// d'un item recrée l'objet dans un nouveau conteneur Loro, et un `mov`
/// concurrent d'un pair garde l'original (t-652). Une slide ou un élément
/// retiré est marqué, jamais supprimé (t-746, t-747) : un id réajouté, puis
/// l'annulation de son ancien retrait, donnent deux nœuds. Partagée par [`canonicalize`] et
/// la relecture du replica : la vue d'un deck JSON aux ids en double égale la
/// relecture de son replica.
/// Déterministe, linéaire (signature calculée pour les seuls ids en double).
pub(crate) fn unique_ids(slides: &mut Vec<Slide>) {
    let mut deck = Ids::new(slides.iter().map(|s| s.id.clone()));
    *slides = unique(std::mem::take(slides), |s: &mut Slide| &mut s.id, &mut deck);
    for slide in slides.iter_mut() {
        let mut taken = Vec::new();
        element_ids(&slide.params, &mut taken);
        unique_params(&mut slide.params, &mut Ids::new(taken));
    }
}

/// Identifiants d'une portée : slides du deck, éléments d'une slide, items d'une liste.
struct Ids {
    /// Tous les identifiants présents ou attribués : un renommage les évite.
    taken: HashSet<String>,
    /// Identifiants des objets gardés jusqu'ici.
    kept: HashSet<String>,
    /// Prochain `n` à essayer par id de base : tous les plus petits sont pris.
    next: HashMap<String, u64>,
    /// Candidats essayés par `fresh` (tests : coût linéaire, t-693).
    #[cfg(test)]
    tries: usize,
}

impl Ids {
    fn new(taken: impl IntoIterator<Item = String>) -> Self {
        Self {
            taken: taken.into_iter().collect(),
            kept: HashSet::new(),
            next: HashMap::new(),
            #[cfg(test)]
            tries: 0,
        }
    }

    /// `{id}-{n}`, au premier `n ≥ 2` libre ; désormais pris.
    ///
    /// Reprend au dernier `n` essayé pour `id` : `taken` ne fait que grandir,
    /// donc les `n` déjà essayés restent pris. Même résultat qu'une recherche
    /// depuis 2, en coût linéaire sur toute la réparation.
    fn fresh(&mut self, id: &str) -> String {
        let n = self.next.entry(id.to_owned()).or_insert(2);
        loop {
            let candidate = format!("{id}-{n}");
            *n += 1;
            #[cfg(test)]
            {
                self.tries += 1;
            }
            if self.taken.insert(candidate.clone()) {
                return candidate;
            }
        }
    }
}

/// Les objets d'une collection, réparés dans la portée `ids` (voir [`unique_ids`]).
fn unique<T: Serialize>(mut items: Vec<T>, id: fn(&mut T) -> &mut String, ids: &mut Ids) -> Vec<T> {
    let mut count: HashMap<String, usize> = HashMap::with_capacity(items.len());
    for item in &mut items {
        *count.entry(id(item).clone()).or_default() += 1;
    }
    // (identifiant d'origine, signature) des objets vus aux ids en double.
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut out: Vec<T> = Vec::with_capacity(items.len());
    for mut item in items {
        let key = id(&mut item).clone();
        if key.is_empty() {
            out.push(item);
            continue;
        }
        if count.get(&key).is_some_and(|&n| n > 1)
            && let Some(sig) = signature(&mut item, id)
            && !seen.insert((key.clone(), sig))
        {
            continue;
        }
        if !ids.kept.insert(key.clone()) {
            let fresh = ids.fresh(&key);
            ids.kept.insert(fresh.clone());
            *id(&mut item) = fresh;
        }
        out.push(item);
    }
    out
}

/// Le JSON de `item` sans son identifiant ; `None` s'il ne se sérialise pas.
fn signature<T: Serialize>(item: &mut T, id: fn(&mut T) -> &mut String) -> Option<String> {
    let key = std::mem::take(id(item));
    let sig = serde_json::to_string(item).ok();
    *id(item) = key;
    sig
}

/// Les identifiants des éléments de `params`, à toute profondeur.
fn element_ids(params: &Params, out: &mut Vec<String>) {
    for v in params.0.values() {
        if let ParamValue::Slot(list) = v {
            for e in list {
                out.push(e.id.clone());
                element_ids(&e.params, out);
            }
        }
    }
}

/// Répare les slots (portée `elements`, celle de la slide) et les listes
/// (portée propre à chacune, listes imbriquées comprises) de `params`, à toute profondeur.
fn unique_params(params: &mut Params, elements: &mut Ids) {
    for v in params.0.values_mut() {
        match v {
            ParamValue::Slot(list) => {
                *list = unique(std::mem::take(list), |e: &mut Element| &mut e.id, elements);
                for e in list.iter_mut() {
                    unique_params(&mut e.params, elements);
                }
            }
            ParamValue::List(items) => {
                let mut scope = Ids::new(items.iter().map(|i| i.id.clone()));
                *items = unique(std::mem::take(items), |i: &mut ListItem| &mut i.id, &mut scope);
                // Listes imbriquées : chacune sa portée ; un item n'a pas de slot.
                for it in items.iter_mut() {
                    unique_params(&mut it.values, elements);
                }
            }
            _ => {}
        }
    }
}

/// Vide = absent ; bords rognés.
fn prose(field: &mut Option<String>) {
    *field = field.take().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
}

/// Forme canonique des paramètres : textes rognés (vides retirés), textes
/// riches canoniques (vides retirés), slots parcourus (vides retirés),
/// nombres canoniques (`null` retiré).
///
/// Un slot vide est retiré comme un texte riche vide : en JSON, les deux
/// s'écrivent `[]` et se relisent en `Rich` vide, et le replica relit une
/// liste vide comme absente. [`Params::slot_mut`] recrée un slot absent.
fn params(p: &mut Params) {
    p.0.retain(|_, v| match v {
        ParamValue::Text(s) => {
            let t = s.trim();
            if t.len() != s.len() {
                *s = t.to_owned();
            }
            !s.is_empty()
        }
        ParamValue::Rich(ps) => {
            paragraphs(ps);
            !ps.is_empty()
        }
        ParamValue::Slot(list) => {
            list.iter_mut().for_each(|e| params(&mut e.params));
            !list.is_empty()
        }
        ParamValue::List(items) => {
            items.iter_mut().for_each(|i| params(&mut i.values));
            !items.is_empty()
        }
        ParamValue::Value(v) => {
            numbers(v);
            !v.is_null()
        }
    });
}

fn paragraphs(text: &mut [Paragraph]) {
    for p in text {
        let mut runs: Vec<Run> = Vec::with_capacity(p.runs.len());
        for mut run in std::mem::take(&mut p.runs) {
            if run.t.is_empty() {
                continue;
            }
            if run.t.contains(PARAGRAPH_SEPARATOR) {
                run.t = run.t.replace(PARAGRAPH_SEPARATOR, "\n");
            }
            match runs.last_mut() {
                Some(last) if same_format(last, &run) => last.t.push_str(&run.t),
                _ => runs.push(run),
            }
        }
        p.runs = runs;
    }
}

fn same_format(a: &Run, b: &Run) -> bool {
    a.bold == b.bold
        && a.italic == b.italic
        && a.color == b.color
        && a.size == b.size
        && a.link == b.link
        && a.highlight == b.highlight
}

/// Un entier hors d'`i64` devient flottant, et les clés des objets sont
/// triées, récursivement.
fn numbers(v: &mut Value) {
    match v {
        Value::Number(n) if n.as_i64().is_none() && n.is_u64() => {
            if let Some(f) = n.as_f64().and_then(Number::from_f64) {
                *n = f;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(numbers),
        Value::Object(map) => {
            // Clés triées : la relecture du replica les rend dans cet ordre,
            // et le JSON d'un paramètre (haché au rendu) ne change pas.
            map.sort_keys();
            map.values_mut().for_each(numbers);
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "canonical_tests.rs"]
mod canonical_tests;
