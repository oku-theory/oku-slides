//! Forme canonique d'un deck : celle que relit le replica.
//!
//! La relecture du replica est canonique (runs vides retirés, runs voisins de
//! même format fusionnés, prose blanche absente…). [`canonicalize`] met un
//! `Deck` sous cette forme, pour que la vue gardée par
//! [`SourceBundle`](crate::SourceBundle) soit toujours égale à la relecture de
//! son replica. Appelée par [`apply_all`](crate::apply_all) et
//! [`SourceBundle::new`](crate::SourceBundle::new).

use serde_json::{Number, Value};

use crate::model::{Deck, Element, ElementBody, Paragraph, Run};
use crate::replica::PARAGRAPH_SEPARATOR;

/// Met `deck` sous forme canonique :
///
/// - prose facultative faite d'espaces → `None`, autre prose rognée (même
///   règle que les ops `set_slide_field` et `set_param`) ;
/// - notes faites d'espaces → `""` (les autres gardent leurs bords) ;
/// - prose riche : `U+2029` dans un run → `\n` (le contenu est gardé, sans
///   nouveau paragraphe), runs vides retirés, runs voisins de même format fusionnés ;
/// - entier JSON au-delà d'`i64` (`vars`, extras de `params`, `tokens`) →
///   flottant, comme le stocke Loro ; `tokens` objet vide → `null`.
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
        for field in [&mut s.name, &mut s.description, &mut s.title, &mut s.subtitle, &mut s.kicker, &mut s.text] {
            prose(field);
        }
        if s.notes.trim().is_empty() {
            s.notes.clear();
        }
        s.blocks.iter_mut().chain(&mut s.elements).for_each(element);
    }
}

/// Vide = absent ; bords rognés.
fn prose(field: &mut Option<String>) {
    *field = field.take().map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
}

fn element(e: &mut Element) {
    match &mut e.body {
        ElementBody::Text { text } => paragraphs(text),
        ElementBody::Card { title, kicker, text, .. } => {
            prose(title);
            prose(kicker);
            paragraphs(text);
        }
        ElementBody::Group { children, .. } => children.iter_mut().for_each(element),
        ElementBody::Image { .. } | ElementBody::Shape { .. } | ElementBody::Chart { .. } => {}
    }
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

/// Un entier hors d'`i64` devient flottant, récursivement.
fn numbers(v: &mut Value) {
    match v {
        Value::Number(n) if n.as_i64().is_none() && n.is_u64() => {
            if let Some(f) = n.as_f64().and_then(Number::from_f64) {
                *n = f;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(numbers),
        Value::Object(map) => map.values_mut().for_each(numbers),
        _ => {}
    }
}

#[cfg(test)]
#[path = "canonical_tests.rs"]
mod canonical_tests;
