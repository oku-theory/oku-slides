//! Champs texte d'une slide dans l'ordre de lecture : ceux de la slide, puis
//! ceux de chaque élément, parents avant enfants (ordre de Tab de l'éditeur),
//! et les colonnes texte des items de chaque liste, dans l'ordre des items.

use std::borrow::Cow;

use crate::model::{Element, Slide};
use crate::params::{ListItem, Params};
use crate::widget::{ParamSchema, ParamType, WidgetRegistry, is_text};

/// Objet qui porte un champ.
#[derive(Debug, Clone, Copy)]
pub enum FieldOwner<'a> {
    Slide,
    Element(&'a Element),
}

/// Un paramètre `plain` ou `rich` d'un objet de la slide, ou une colonne
/// `plain` ou `rich` d'un item de liste.
#[derive(Debug, Clone, Copy)]
pub struct TextField<'a> {
    pub owner: FieldOwner<'a>,
    /// Id de la slide ou de l'élément.
    pub owner_id: &'a str,
    /// Paramètre du propriétaire, ou colonne de la liste pour un item.
    pub param: &'a ParamSchema,
    /// Paramètres où lire `param` : ceux du propriétaire, ou ceux de l'item.
    pub values: &'a Params,
    /// Pour un item : le nom de la liste et l'item.
    pub list: Option<(&'a str, &'a ListItem)>,
}

impl<'a> TextField<'a> {
    /// Nom du champ : `liste/id/colonne` pour un item, sinon le paramètre.
    pub fn name(&self) -> Cow<'a, str> {
        match self.list {
            Some((list, item)) => Cow::Owned(format!("{list}/{}/{}", item.id, self.param.name)),
            None => Cow::Borrowed(self.param.name.as_str()),
        }
    }
}

/// Les champs d'un objet de `kind` : un par paramètre texte, dans l'ordre du
/// schéma, et pour une liste un par colonne texte de chaque item.
fn text_params<'a>(
    registry: &'a WidgetRegistry,
    kind: &str,
    owner: FieldOwner<'a>,
    owner_id: &'a str,
    values: &'a Params,
) -> impl Iterator<Item = TextField<'a>> {
    registry.get(kind).into_iter().flat_map(|w| w.params.iter()).flat_map(move |param| {
        let own = is_text(&param.ty).then_some(TextField { owner, owner_id, param, values, list: None });
        let (items, columns) = match &param.ty {
            ParamType::List { item, .. } => (values.list(&param.name).unwrap_or_default(), item.as_slice()),
            _ => (&[][..], &[][..]),
        };
        let name = param.name.as_str();
        let cells = items.iter().flat_map(move |it| {
            columns.iter().filter(|c| is_text(&c.ty)).map(move |param| TextField {
                owner,
                owner_id,
                param,
                values: &it.values,
                list: Some((name, it)),
            })
        });
        own.into_iter().chain(cells)
    })
}

impl Slide {
    /// Champs texte dans l'ordre de lecture : slide (ordre du schéma), puis
    /// éléments (parcours des slots), chacun dans l'ordre de son schéma ; les
    /// colonnes texte des items de chaque liste suivent, dans l'ordre des
    /// items.
    pub fn text_fields<'a>(&'a self, registry: &'a WidgetRegistry) -> Vec<TextField<'a>> {
        let slide = text_params(registry, &self.kind, FieldOwner::Slide, &self.id, &self.params);
        let elements = self.walk().flat_map(|l| {
            let e = l.element;
            text_params(registry, &e.kind, FieldOwner::Element(e), &e.id, &e.params)
        });
        slide.chain(elements).collect()
    }
}

#[cfg(test)]
#[path = "fields_tests.rs"]
mod fields_tests;
