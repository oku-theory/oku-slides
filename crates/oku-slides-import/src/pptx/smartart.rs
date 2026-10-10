//! SmartArt : sa version déjà dessinée (`diagramDrawing`), lue comme un groupe de formes.

use std::rc::Rc;

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::context::PartCtx;
use crate::pptx::package::Rels;
use crate::pptx::shapes::{GroupCtx, MAX_NESTING, opaque, parse_tree};
use crate::pptx::xml::{attr_r, attr_str, child, descendant};
use crate::source::{Frame, Item, ItemKind, OpaqueReason, WarningKind};

/// Part du dessin d'un SmartArt, via sa part de données ; `None` si une étape manque.
fn drawing_part(data: Node<'_, '_>, cx: &PartCtx<'_>, id: &str) -> Option<String> {
    let dm = child(data, "relIds").and_then(|r| attr_r(r, "dm")).and_then(|r| cx.rels.target(r))?.to_owned();
    let xml = cx.part_text(&dm, id)?;
    let doc = cx.linked_doc(&xml, &dm, Some(id), WarningKind::UnreadableItem(clip(&dm)))?;
    let rel = descendant(doc.root_element(), "dataModelExt").and_then(|e| attr_str(e, "relId"))?;
    cx.rels.target(rel).map(str::to_owned)
}

/// Le SmartArt du cadre `id` : un groupe de formes, ou un opaque.
pub(crate) fn smartart(data: Node<'_, '_>, cx: &PartCtx<'_>, id: &str, frame: Frame, g: &GroupCtx) -> ItemKind {
    let not_drawn = || opaque(OpaqueReason::SmartArt, None);
    if g.in_drawing || g.depth >= MAX_NESTING {
        cx.warn(Some(id), WarningKind::NestingTooDeep(clip(id)));
        return not_drawn();
    }
    let Some(part) = drawing_part(data, cx, id) else {
        if !cx.shared.sink.borrow().has_fatal() {
            cx.warn(Some(id), WarningKind::MissingPart("diagramDrawing".into()));
        }
        return not_drawn();
    };
    let Some(xml) = cx.part_text(&part, id) else { return not_drawn() };
    let Some(doc) = cx.linked_doc(&xml, &part, Some(id), WarningKind::UnreadableItem(clip(&part))) else {
        return not_drawn();
    };
    let Some(tree) = descendant(doc.root_element(), "spTree") else { return not_drawn() };
    // Relations du dessin illisibles : le dessin reste lu, sans ses images (`UnreadableItem`).
    let rels: Rc<Rels> =
        match cx.soft(cx.pkg().rels(&part), Some(id), WarningKind::UnreadableItem(clip(&format!("{part}: rels")))) {
            Some(r) => r,
            None if cx.shared.sink.borrow().has_fatal() => return not_drawn(),
            None => Rc::default(),
        };
    let sub = cx.sub(rels, id.to_owned());
    let inner = GroupCtx {
        sx: g.sx,
        sy: g.sy,
        tx: frame.x,
        ty: frame.y,
        fill: None,
        depth: g.depth.saturating_add(1),
        in_drawing: true,
    };
    let items: Vec<Item> = parse_tree(tree, &sub, &inner).into_iter().map(|p| p.item).collect();
    if items.is_empty() { not_drawn() } else { ItemKind::Group { items, smartart: true } }
}

#[cfg(test)]
#[path = "smartart_tests.rs"]
mod smartart_tests;
