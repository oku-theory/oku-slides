//! Parcours des éléments d'une slide : blocs, éléments libres et groupes,
//! avec la place de chacun. Une seule traversée sert aux opérations, à la
//! validation et à la session.

use crate::model::{Element, ElementBody, GroupLayoutKind, Slide, Zone};

/// Un élément et sa place dans la slide.
#[derive(Debug, Clone, Copy)]
pub struct Located<'a> {
    pub element: &'a Element,
    /// Zone de premier niveau qui le contient.
    pub zone: Zone,
    /// Groupe parent, s'il y en a un.
    pub parent: Option<&'a str>,
    /// Position dans sa liste.
    pub index: usize,
    /// Son conteneur place ses éléments librement (zone des éléments libres,
    /// ou groupe de layout `free`).
    pub in_free_layout: bool,
}

impl Located<'_> {
    /// Placé librement : dans un conteneur libre, avec un cadre ou une cellule.
    pub fn is_free(&self) -> bool {
        self.in_free_layout && (self.element.frame.is_some() || self.element.cell.is_some())
    }
}

/// Parcours en profondeur, parents avant enfants, blocs avant éléments libres.
#[derive(Debug)]
pub struct Walk<'a> {
    stack: Vec<Level<'a>>,
}

#[derive(Debug)]
struct Level<'a> {
    list: std::iter::Enumerate<std::slice::Iter<'a, Element>>,
    zone: Zone,
    parent: Option<&'a str>,
    free: bool,
}

impl<'a> Walk<'a> {
    /// Parcourt `list` (et ses groupes) comme le contenu de `zone`.
    pub(crate) fn over(list: &'a [Element], zone: Zone) -> Self {
        Self { stack: vec![Level { list: list.iter().enumerate(), zone, parent: None, free: zone == Zone::Elements }] }
    }
}

impl<'a> Iterator for Walk<'a> {
    type Item = Located<'a>;

    fn next(&mut self) -> Option<Located<'a>> {
        loop {
            let level = self.stack.last_mut()?;
            let Some((index, element)) = level.list.next() else {
                self.stack.pop();
                continue;
            };
            let located =
                Located { element, zone: level.zone, parent: level.parent, index, in_free_layout: level.free };
            if let ElementBody::Group { layout, children } = &element.body {
                self.stack.push(Level {
                    list: children.iter().enumerate(),
                    zone: located.zone,
                    parent: Some(&element.id),
                    free: layout.kind == GroupLayoutKind::Free,
                });
            }
            return Some(located);
        }
    }
}

impl Slide {
    /// Tous les éléments de la slide, blocs d'abord, groupes compris.
    pub fn walk(&self) -> Walk<'_> {
        let mut walk = Walk::over(&self.elements, Zone::Elements);
        walk.stack.extend(Walk::over(&self.blocks, Zone::Blocks).stack);
        walk
    }

    /// Un élément par son identifiant, avec sa place.
    pub fn locate(&self, id: &str) -> Option<Located<'_>> {
        self.walk().find(|l| l.element.id == id)
    }

    /// Identifiants de tous les éléments de la slide.
    pub fn element_ids(&self) -> impl Iterator<Item = &str> {
        self.walk().map(|l| l.element.id.as_str())
    }
}

#[cfg(test)]
#[path = "locate_tests.rs"]
mod locate_tests;
