//! Parcours des éléments d'une slide à travers ses slots, parents avant
//! enfants. Une seule traversée sert aux opérations, à la validation et à la
//! session.

use crate::model::{Element, Slide};
use crate::params::Params;

/// Un élément et sa place.
#[derive(Debug, Clone, Copy)]
pub struct Located<'a> {
    /// The element found.
    pub element: &'a Element,
    /// Élément qui porte le slot, ou `None` pour un slot de la slide.
    pub owner: Option<&'a str>,
    /// Nom du slot.
    pub slot: &'a str,
    /// Position of the element in its slot.
    pub index: usize,
    /// 0 pour un enfant direct d'un slot de la slide.
    pub depth: usize,
}

/// Profondeur maximale d'imbrication des slots (voir « Décisions »).
pub const MAX_SLOT_DEPTH: usize = 16;

/// Profondeur des descendants d'un élément : 0 s'il n'a pas d'enfant.
/// Itérative : un deck non fiable ne peut pas épuiser la pile.
#[must_use]
pub fn slot_depth(e: &Element) -> usize {
    let mut max = 0;
    let mut stack = vec![(e, 0usize)];
    while let Some((e, d)) = stack.pop() {
        max = max.max(d);
        for (_, children) in e.params.slots() {
            stack.extend(children.iter().map(|c| (c, d + 1)));
        }
    }
    max
}

impl Located<'_> {
    /// Placé librement : avec un cadre ou une cellule.
    #[must_use]
    pub fn is_free(&self) -> bool {
        self.element.frame.is_some() || self.element.cell.is_some()
    }
}

/// Un slot en cours de parcours : propriétaire, nom, profondeur, éléments restants.
type Level<'a> = (Option<&'a str>, &'a str, usize, std::iter::Enumerate<std::slice::Iter<'a, Element>>);

/// Parcours en profondeur des slots, dans l'ordre des noms de paramètres.
#[derive(Debug)]
pub struct Walk<'a> {
    stack: Vec<Level<'a>>,
}

impl<'a> Walk<'a> {
    /// Parcourt les slots de `params`, portés par `owner`.
    #[must_use]
    pub fn over(params: &'a Params, owner: Option<&'a str>) -> Self {
        let mut stack: Vec<_> = params.slots().map(|(name, list)| (owner, name, 0, list.iter().enumerate())).collect();
        stack.reverse();
        Self { stack }
    }
}

impl<'a> Iterator for Walk<'a> {
    type Item = Located<'a>;

    fn next(&mut self) -> Option<Located<'a>> {
        loop {
            let (owner, slot, depth, list) = self.stack.last_mut()?;
            let (owner, slot, depth) = (*owner, *slot, *depth);
            let Some((index, element)) = list.next() else {
                self.stack.pop();
                continue;
            };
            let mut children: Vec<_> = element
                .params
                .slots()
                .map(|(name, l)| (Some(element.id.as_str()), name, depth + 1, l.iter().enumerate()))
                .collect();
            children.reverse();
            self.stack.extend(children);
            return Some(Located { element, owner, slot, index, depth });
        }
    }
}

impl Slide {
    /// All elements of the slide, parents before children, slots by name.
    #[must_use]
    pub fn walk(&self) -> Walk<'_> {
        Walk::over(&self.params, None)
    }

    /// The element `id` and its place, at any slot depth.
    #[must_use]
    pub fn locate(&self, id: &str) -> Option<Located<'_>> {
        self.walk().find(|l| l.element.id == id)
    }

    /// The ids of all elements, in [`Slide::walk`] order.
    pub fn element_ids(&self) -> impl Iterator<Item = &str> {
        self.walk().map(|l| l.element.id.as_str())
    }
}

#[cfg(test)]
#[path = "locate_tests.rs"]
mod locate_tests;
