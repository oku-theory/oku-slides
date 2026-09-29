//! Champs texte d'une slide dans l'ordre de lecture : ceux de la slide, puis
//! ceux de chaque élément, parents avant enfants (ordre de Tab de l'éditeur).

use crate::model::{Element, Slide};
use crate::widget::{ParamSchema, ParamType, WidgetRegistry};

/// Objet qui porte un champ.
#[derive(Debug, Clone, Copy)]
pub enum FieldOwner<'a> {
    Slide,
    Element(&'a Element),
}

/// Un paramètre `plain` ou `rich` d'un objet de la slide.
#[derive(Debug, Clone, Copy)]
pub struct TextField<'a> {
    pub owner: FieldOwner<'a>,
    /// Id de la slide ou de l'élément.
    pub owner_id: &'a str,
    pub param: &'a ParamSchema,
}

fn text_params<'a>(registry: &'a WidgetRegistry, kind: &str) -> impl Iterator<Item = &'a ParamSchema> {
    registry
        .get(kind)
        .into_iter()
        .flat_map(|w| w.params.iter())
        .filter(|p| matches!(p.ty, ParamType::Plain | ParamType::Rich))
}

impl Slide {
    /// Champs texte dans l'ordre de lecture : slide (ordre du schéma), puis
    /// éléments (parcours des slots), chacun dans l'ordre de son schéma.
    pub fn text_fields<'a>(&'a self, registry: &'a WidgetRegistry) -> Vec<TextField<'a>> {
        let slide = text_params(registry, &self.kind).map(|param| TextField {
            owner: FieldOwner::Slide,
            owner_id: &self.id,
            param,
        });
        let elements = self.walk().flat_map(|l| {
            let e = l.element;
            text_params(registry, &e.kind).map(move |param| TextField {
                owner: FieldOwner::Element(e),
                owner_id: &e.id,
                param,
            })
        });
        slide.chain(elements).collect()
    }
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod fields_tests;
