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
mod tests {
    use super::*;
    use crate::model::{Frame, GroupLayout, SlideKind, Style};

    fn group(id: &str, kind: GroupLayoutKind, children: Vec<Element>) -> Element {
        Element {
            id: id.into(),
            frame: None,
            cell: None,
            style: Style::default(),
            body: ElementBody::Group { layout: GroupLayout { kind, ..Default::default() }, children },
        }
    }

    fn framed(id: &str) -> Element {
        Element { frame: Some(Frame { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }), ..Element::text(id, vec![]) }
    }

    fn slide() -> Slide {
        let mut s = Slide::new("s", SlideKind::Content);
        s.blocks = vec![
            Element::text("b1", vec![]),
            group("g", GroupLayoutKind::Free, vec![framed("g1"), Element::text("g2", vec![])]),
        ];
        s.elements = vec![framed("e1")];
        s
    }

    #[test]
    fn parcours_parents_avant_enfants_blocs_avant_elements() {
        let s = slide();
        let ids: Vec<&str> = s.element_ids().collect();
        assert_eq!(ids, ["b1", "g", "g1", "g2", "e1"]);
    }

    #[test]
    fn localise_un_enfant_de_groupe_avec_sa_place() {
        let s = slide();
        let g2 = s.locate("g2").unwrap();
        assert_eq!((g2.zone, g2.parent, g2.index), (Zone::Blocks, Some("g"), 1));
        assert!(g2.in_free_layout && !g2.is_free(), "groupe libre, mais ni cadre ni cellule");
        assert!(s.locate("g1").unwrap().is_free());
        assert!(!s.locate("b1").unwrap().in_free_layout);
        let e1 = s.locate("e1").unwrap();
        assert_eq!((e1.zone, e1.parent), (Zone::Elements, None));
        assert!(e1.is_free());
        assert!(s.locate("absent").is_none());
    }
}
