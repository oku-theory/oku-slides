//! Items de liste d'une page : boîte, rang et bornes de leur liste, pour les
//! poignées de la scène (retirer, déplacer) ; et les listes elles-mêmes, même
//! vides.

use std::borrow::Cow;

use oku_slides_core::{ListPath, ListSpec, ParamSchema, ParamType, Params, Slide, WidgetRegistry};
use oku_slides_render::ItemBox;
use serde::Serialize;

use crate::fields::OwnerKind;

/// An item on the page, and its list.
#[derive(Debug, Serialize)]
pub struct ItemView<'a> {
    /// Id of the slide or element that holds the list.
    pub owner: &'a str,
    /// Slide or element: says what `owner` refers to.
    pub owner_kind: OwnerKind,
    /// Path of the list: the `list` parameter, or `list/id/column` for the nested list of an item.
    pub list: &'a str,
    /// Id of the item.
    pub item: &'a str,
    /// Rank of the item in its list, from 0.
    pub index: usize,
    /// Number of items in the list.
    pub len: usize,
    /// Minimum items: the schema's `min`, below which removal stops.
    pub min: usize,
    /// Maximum items: the schema's `max`, otherwise the core's bound (`LIST_ITEMS_MAX`).
    pub max: usize,
    /// Name of one item, in the singular ("step").
    pub item_label: Option<&'a str>,
    /// Left edge of the rendered item, in pt from the page's left edge.
    pub x: f64,
    /// Top edge of the rendered item, in pt from the top of the page.
    pub y: f64,
    /// Width of the rendered item, in pt.
    pub w: f64,
    /// Height of the rendered item, in pt.
    pub h: f64,
}

/// A list on the page, even an empty one.
#[derive(Debug, Serialize)]
pub struct ListView<'a> {
    /// Id of the slide or element that holds the list.
    pub owner: &'a str,
    /// Slide or element: says what `owner` refers to.
    pub owner_kind: OwnerKind,
    /// Path of the list: the `list` parameter, or `list/id/column` for the nested list of an item.
    pub list: Cow<'a, str>,
    /// Number of items (0 if the list is absent).
    pub len: usize,
    /// Minimum items: the schema's `min`, below which removal stops.
    pub min: usize,
    /// Maximum items: the schema's `max`, otherwise the core's bound (`LIST_ITEMS_MAX`).
    pub max: usize,
    /// Name of one item, in the singular ("step").
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
            let path = ListPath::parse(&b.list)?;
            let p = path.schema(&registry.get(kind)?.params)?;
            let shape = Shape::of(p)?;
            let items = path.items(params)?;
            let index = items.iter().position(|i| i.id == b.item)?;
            Some(ItemView {
                owner: &b.owner,
                owner_kind,
                list: &b.list,
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
            schema.flat_map(move |p| {
                let top = Shape::of(p).map(|shape| ListView {
                    owner,
                    owner_kind,
                    list: Cow::Borrowed(p.name.as_str()),
                    len: params.list(&p.name).map_or(0, <[_]>::len),
                    min: shape.spec.min,
                    max: shape.spec.max_items(),
                    item_label: shape.item_label,
                });
                // Listes imbriquées : une par item et par colonne `list`, même vide.
                let columns = match &p.ty {
                    ParamType::List { item, .. } => item.as_slice(),
                    _ => &[],
                };
                let items = params.list(&p.name).unwrap_or_default();
                let nested = items.iter().flat_map(move |it| {
                    columns.iter().filter_map(move |c| {
                        let shape = Shape::of(c)?;
                        Some(ListView {
                            owner,
                            owner_kind,
                            list: Cow::Owned(format!("{}/{}/{}", p.name, it.id, c.name)),
                            len: it.values.list(&c.name).map_or(0, <[_]>::len),
                            min: shape.spec.min,
                            max: shape.spec.max_items(),
                            item_label: shape.item_label,
                        })
                    })
                });
                top.into_iter().chain(nested)
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "items_tests.rs"]
mod items_tests;
