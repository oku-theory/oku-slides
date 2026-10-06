//! Items de liste d'une page : boîte, rang et bornes de leur liste, pour les
//! poignées de la scène (retirer, déplacer) ; et les listes elles-mêmes, même
//! vides.

use oku_slides_core::{ListSpec, ParamSchema, ParamType, Params, Slide, WidgetRegistry};
use oku_slides_render::ItemBox;
use serde::Serialize;

use crate::fields::OwnerKind;

/// Un item sur la page et sa liste.
#[derive(Debug, Serialize)]
pub struct ItemView<'a> {
    /// Id de la slide ou de l'élément qui porte la liste.
    pub owner: &'a str,
    pub owner_kind: OwnerKind,
    /// Nom du paramètre `list`.
    pub list: &'a str,
    /// Id de l'item.
    pub item: &'a str,
    /// Rang de l'item dans sa liste.
    pub index: usize,
    /// Nombre d'items de la liste.
    pub len: usize,
    pub min: usize,
    /// Items au plus : le `max` du schéma, sinon la borne du cœur (`LIST_ITEMS_MAX`).
    pub max: usize,
    /// Nom d'un item, au singulier (« étape »).
    pub item_label: Option<&'a str>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Une liste de la page, même vide.
#[derive(Debug, Serialize)]
pub struct ListView<'a> {
    pub owner: &'a str,
    pub owner_kind: OwnerKind,
    /// Nom du paramètre `list`.
    pub list: &'a str,
    /// Nombre d'items (0 si la liste est absente).
    pub len: usize,
    pub min: usize,
    /// Items au plus : le `max` du schéma, sinon la borne du cœur (`LIST_ITEMS_MAX`).
    pub max: usize,
    pub item_label: Option<&'a str>,
}

/// Colonnes, bornes et libellé d'un paramètre `list`.
struct Shape<'a> {
    spec: ListSpec<'a>,
    item_label: Option<&'a str>,
}

impl<'a> Shape<'a> {
    fn of(p: &'a ParamSchema) -> Option<Self> {
        let spec = ListSpec::of(&p.name, &p.ty)?;
        let ParamType::List { item_label, .. } = &p.ty else { return None };
        Some(Self { spec, item_label: item_label.as_deref() })
    }
}

/// Items des boîtes `boxes` qui appartiennent à `slide`, dans l'ordre des boîtes.
pub(crate) fn item_views<'a>(
    slide: &'a Slide,
    registry: &'a WidgetRegistry,
    boxes: &'a [ItemBox],
) -> Vec<ItemView<'a>> {
    boxes
        .iter()
        .filter_map(|b| {
            let (owner_kind, kind, params) = if b.owner == slide.id {
                (OwnerKind::Slide, slide.kind.as_str(), &slide.params)
            } else {
                let e = slide.locate(&b.owner)?.element;
                (OwnerKind::Element, e.kind.as_str(), &e.params)
            };
            let p = registry.get(kind)?.param(&b.list)?;
            let shape = Shape::of(p)?;
            let items = params.list(&b.list)?;
            let index = items.iter().position(|i| i.id == b.item)?;
            Some(ItemView {
                owner: &b.owner,
                owner_kind,
                list: &p.name,
                item: &items[index].id,
                index,
                len: items.len(),
                min: shape.spec.min,
                max: shape.spec.max_items(),
                item_label: shape.item_label,
                x: b.x,
                y: b.y,
                w: b.w,
                h: b.h,
            })
        })
        .collect()
}

/// Les listes de `slide` et de ses éléments, dans l'ordre de lecture.
pub(crate) fn list_views<'a>(slide: &'a Slide, registry: &'a WidgetRegistry) -> Vec<ListView<'a>> {
    let slide_owner = std::iter::once((slide.id.as_str(), OwnerKind::Slide, slide.kind.as_str(), &slide.params));
    let elements =
        slide.walk().map(|l| (l.element.id.as_str(), OwnerKind::Element, l.element.kind.as_str(), &l.element.params));
    slide_owner
        .chain(elements)
        .flat_map(|(owner, owner_kind, kind, params): (&'a str, OwnerKind, &'a str, &'a Params)| {
            let schema = registry.get(kind).into_iter().flat_map(|w| w.params.iter());
            schema.filter_map(move |p| {
                let shape = Shape::of(p)?;
                Some(ListView {
                    owner,
                    owner_kind,
                    list: &p.name,
                    len: params.list(&p.name).map_or(0, <[_]>::len),
                    min: shape.spec.min,
                    max: shape.spec.max_items(),
                    item_label: shape.item_label,
                })
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "items_tests.rs"]
mod items_tests;
