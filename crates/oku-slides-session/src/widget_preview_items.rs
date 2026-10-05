//! Items et listes de la vue d'aperçu d'un widget : les poignées de liste
//! (ajouter, retirer, déplacer) de l'aperçu, comme sur la scène
//! (`items.rs`), pour le seul widget montré.
//!
//! La slide d'aperçu porte les valeurs du cas affiché : l'interface écrit
//! chaque changement dans ce cas (`edit_example_list`), jamais dans le deck.

use oku_slides_core::{Slide, WidgetRegistry};
use oku_slides_render::ItemBox;

use crate::items::{self, ItemView, ListView};

/// Le type du propriétaire `owner` (la slide ou l'un de ses éléments).
fn kind_of<'a>(slide: &'a Slide, owner: &str) -> Option<&'a str> {
    if owner == slide.id { Some(slide.kind.as_str()) } else { slide.locate(owner).map(|l| l.element.kind.as_str()) }
}

/// Les items des boîtes `boxes` et les listes de `slide` dont le propriétaire
/// est le widget `widget` (la slide d'un widget de slide, l'élément d'aperçu
/// sinon).
pub(crate) fn preview_items<'a>(
    slide: &'a Slide,
    registry: &'a WidgetRegistry,
    boxes: &'a [ItemBox],
    widget: &str,
) -> (Vec<ItemView<'a>>, Vec<ListView<'a>>) {
    let shown = |owner: &str| kind_of(slide, owner) == Some(widget);
    let items = items::item_views(slide, registry, boxes).into_iter().filter(|i| shown(i.owner)).collect();
    let lists = items::list_views(slide, registry).into_iter().filter(|l| shown(l.owner)).collect();
    (items, lists)
}

#[cfg(test)]
#[path = "widget_preview_items_tests.rs"]
mod widget_preview_items_tests;
