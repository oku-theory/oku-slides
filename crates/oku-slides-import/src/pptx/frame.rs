//! `graphicFrame` : tableau, graphique, SmartArt, objet OLE, sinon opaque.

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::chart::{ChartSkip, parse_chart};
use crate::pptx::context::PartCtx;
use crate::pptx::shapes::{GroupCtx, ParsedItem, blip_asset, identity, opaque};
use crate::pptx::smartart::smartart;
use crate::pptx::table::parse_table;
use crate::pptx::xml::{attr_r, attr_str, child, descendant};
use crate::source::{Frame, Item, ItemKind, OpaqueReason, WarningKind};

/// Lit un `graphicFrame`.
pub(crate) fn parse_graphic_frame(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    let (id, name, alt) = identity(n, cx)?;
    let frame = child(n, "xfrm").map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    let data = descendant(n, "graphicData")?;
    let uri = attr_str(data, "uri").unwrap_or_default();
    let unknown = || opaque(OpaqueReason::Unknown(clip(uri)), None);
    let kind = if uri.ends_with("/table") {
        child(data, "tbl").map_or_else(unknown, |t| ItemKind::Table(parse_table(t, cx)))
    } else if uri.ends_with("/chart") {
        chart_item(data, cx, &id)
    } else if uri.ends_with("/diagram") {
        smartart(data, cx, &id, frame, g)
    } else if uri.ends_with("/ole") {
        let fallback = descendant(data, "pic").and_then(|p| descendant(p, "blip")).and_then(|b| blip_asset(b, cx));
        opaque(OpaqueReason::Ole, fallback)
    } else {
        unknown()
    };
    Some(ParsedItem { item: Item { id, name, alt, frame, z: 0, role: None, placeholder: None, kind }, ph: None })
}

fn chart_item(data: Node<'_, '_>, cx: &PartCtx<'_>, id: &str) -> ItemKind {
    let unknown = |what: &str| opaque(OpaqueReason::Unknown(clip(what)), None);
    let Some(rid) = child(data, "chart").and_then(|c| attr_r(c, "id")) else { return unknown("chart") };
    let Some(part) = cx.rels.target(rid).map(str::to_owned) else {
        cx.warn(Some(id), WarningKind::MissingPart(clip(rid)));
        return unknown("chart");
    };
    let Some(xml) = cx.part_text(&part, id) else { return unknown("chart") };
    let Some(doc) = cx.linked_doc(&xml, &part, Some(id), WarningKind::UnreadableItem(clip(&part))) else {
        return unknown("chart");
    };
    match parse_chart(doc.root_element(), cx, id) {
        Ok(c) => ItemKind::Chart(c),
        Err(ChartSkip::Unsupported(name)) => {
            cx.warn(Some(id), WarningKind::UnsupportedChart(name.clone()));
            unknown(&name)
        }
        Err(ChartSkip::Malformed(what)) => {
            cx.warn(Some(id), WarningKind::UnreadableItem(clip(&format!("{part}: {what}"))));
            unknown("chart")
        }
    }
}
