//! SmartArt : son modèle de données (disposition, nœuds et leur ordre) et sa
//! version déjà dessinée (`diagramDrawing`), lue comme un groupe de formes.
//!
//! Sans dessin (LibreOffice, Google, fichiers générés), le modèle seul garde le
//! texte des nœuds : un item [`ItemKind::SmartArt`]. Un SmartArt trouvé dans le
//! dessin d'un SmartArt n'est jamais suivi.

use std::collections::BTreeMap;
use std::rc::Rc;

use roxmltree::Node;

use crate::collect::clip;
use crate::pptx::cascade::{TextChain, resolve_textbox};
use crate::pptx::context::{PartCtx, raw_key};
use crate::pptx::package::Rels;
use crate::pptx::shapes::{GroupCtx, MAX_NESTING, opaque, parse_tree};
use crate::pptx::text::{RunProps, parse_txbody};
use crate::pptx::xml::{attr_r, attr_str, attr_u32, child, children, descendant};
use crate::source::{Frame, Item, ItemKind, OpaqueReason, SmartArt, SmartArtNode, WarningKind};

/// Nœuds gardés au plus par SmartArt ; les suivants sont omis (`Approximated`).
pub(crate) const MAX_NODES: usize = 512;
/// Profondeur gardée au plus dans l'arbre des nœuds ; plus bas, coupé (`NestingTooDeep`).
pub(crate) const MAX_NODE_DEPTH: u8 = 16;

/// Cible de la relation `name` (`dm`, `lo`…) du cadre.
fn rel_target(data: Node<'_, '_>, cx: &PartCtx<'_>, name: &str) -> Option<String> {
    child(data, "relIds").and_then(|r| attr_r(r, name)).and_then(|r| cx.rels.target(r)).map(str::to_owned)
}

/// Relations d'une part liée ; illisibles : vides et `UnreadableItem`, `None` après une erreur fatale.
fn linked_rels(cx: &PartCtx<'_>, part: &str, id: &str) -> Option<Rc<Rels>> {
    match cx.soft(cx.pkg().rels(part), Some(id), WarningKind::UnreadableItem(clip(&format!("{part}: rels")))) {
        Some(r) => Some(r),
        None if cx.shared.sink.borrow().has_fatal() => None,
        None => Some(Rc::default()),
    }
}

/// Un point du modèle retenu : le document (racine) ou un nœud.
struct Point<'a, 'i> {
    model: &'a str,
    doc: bool,
    node: Node<'a, 'i>,
}

/// Les points `doc`, `node` et `asst` de `ptLst`, au plus `MAX_NODES` nœuds, `modelId` uniques.
fn points<'a, 'i>(root: Node<'a, 'i>, cx: &PartCtx<'_>, id: &str) -> Vec<Point<'a, 'i>> {
    let mut out: Vec<Point<'a, 'i>> = Vec::new();
    let mut nodes = 0usize;
    for pt in child(root, "ptLst").into_iter().flat_map(|l| children(l, "pt")) {
        let doc = match attr_str(pt, "type") {
            None | Some("node" | "asst") => false,
            Some("doc") => true,
            Some(_) => continue,
        };
        let Some(model) = attr_str(pt, "modelId") else { continue };
        if out.iter().any(|p| p.model == model) {
            continue;
        }
        if !doc {
            if nodes == MAX_NODES {
                cx.warn(Some(id), WarningKind::Approximated(format!("smartart: {MAX_NODES} nodes")));
                break;
            }
            nodes += 1;
        }
        out.push(Point { model, doc, node: pt });
    }
    out
}

/// Enfants de chaque point (index), par `srcOrd` puis ordre du fichier, d'après les liens `parOf`.
fn children_of(root: Node<'_, '_>, pts: &[Point<'_, '_>]) -> (BTreeMap<usize, Vec<usize>>, Vec<bool>) {
    let index: BTreeMap<&str, usize> = pts.iter().enumerate().map(|(k, p)| (p.model, k)).collect();
    let mut kids: BTreeMap<usize, Vec<(u32, usize)>> = BTreeMap::new();
    let mut has_parent = vec![false; pts.len()];
    for cxn in child(root, "cxnLst").into_iter().flat_map(|l| children(l, "cxn")) {
        if !matches!(attr_str(cxn, "type"), None | Some("parOf")) {
            continue;
        }
        let at = |name: &str| attr_str(cxn, name).and_then(|m| index.get(m).copied());
        let (Some(src), Some(dest)) = (at("srcId"), at("destId")) else { continue };
        if src == dest || has_parent[dest] {
            continue;
        }
        has_parent[dest] = true;
        kids.entry(src).or_default().push((attr_u32(cxn, "srcOrd").unwrap_or(0), dest));
    }
    let sorted = kids
        .into_iter()
        .map(|(k, mut v)| {
            // Tri stable : à `srcOrd` égal, l'ordre des liens dans le fichier.
            v.sort_by_key(|(ord, _)| *ord);
            (k, v.into_iter().map(|(_, d)| d).collect())
        })
        .collect();
    (sorted, has_parent)
}

/// Ordre de lecture des points : parcours préfixe depuis les documents, puis
/// depuis les nœuds sans parent, puis ceux d'un cycle ; (index, profondeur), le
/// document à -1. Itératif, chaque point visité une fois, coupé à `MAX_NODE_DEPTH`.
fn reading_order(root: Node<'_, '_>, pts: &[Point<'_, '_>], cx: &PartCtx<'_>, id: &str) -> Vec<(usize, i32)> {
    let (kids, has_parent) = children_of(root, pts);
    let starts = (0..pts.len())
        .filter(|&k| pts[k].doc)
        .chain((0..pts.len()).filter(|&k| !pts[k].doc && !has_parent[k]))
        .chain(0..pts.len());
    let mut seen = vec![false; pts.len()];
    let mut out = Vec::new();
    let mut cut = false;
    for start in starts {
        let mut stack = vec![(start, if pts[start].doc { -1 } else { 0 })];
        while let Some((k, depth)) = stack.pop() {
            if seen[k] {
                continue;
            }
            seen[k] = true;
            // Sous la coupe, le sous-arbre est parcouru sans être gardé : il ne revient pas en racine.
            if depth > i32::from(MAX_NODE_DEPTH) {
                cut = true;
            } else {
                out.push((k, depth));
            }
            for &c in kids.get(&k).into_iter().flatten().rev() {
                stack.push((c, depth + 1));
            }
        }
    }
    if cut {
        cx.warn(Some(id), WarningKind::NestingTooDeep(clip(id)));
    }
    out
}

/// Disposition et catégorie : `layoutDef` de la part `lo`, sinon le `prSet` du document.
fn layout_of(pts: &[Point<'_, '_>], lo: Option<&str>, cx: &PartCtx<'_>, id: &str) -> (Option<String>, Option<String>) {
    let def = lo.and_then(|part| {
        let xml = cx.part_text(part, id)?;
        let doc = cx.linked_doc(&xml, part, Some(id), WarningKind::UnreadableItem(clip(part)))?;
        let def = doc.root_element();
        (def.tag_name().name() == "layoutDef").then(|| {
            let cat = child(def, "catLst").and_then(|c| child(c, "cat")).and_then(|c| attr_str(c, "type"));
            (attr_str(def, "uniqueId").map(clip), cat.map(clip))
        })
    });
    let pr_set = pts.iter().find(|p| p.doc).and_then(|p| child(p.node, "prSet"));
    let from_doc = |name: &str| pr_set.and_then(|s| attr_str(s, name)).map(clip);
    let (layout, category) = def.unwrap_or_default();
    (layout.or_else(|| from_doc("loTypeId")), category.or_else(|| from_doc("loCatId")))
}

/// Le modèle du SmartArt `id` lu dans la part de données `dm` ; `lo` : sa part de disposition.
fn model(root: Node<'_, '_>, dm: &str, lo: Option<&str>, cx: &PartCtx<'_>, id: &str) -> Option<SmartArt> {
    let rels = linked_rels(cx, dm, id)?;
    let sub = cx.sub(rels, id.to_owned());
    let pts = points(root, cx, id);
    let inh = cx.lookup(None);
    let chain = TextChain { levels: inh.levels.iter().collect(), bodies: inh.bodies.iter().collect() };
    let nodes = reading_order(root, &pts, cx, id)
        .into_iter()
        .filter_map(|(k, depth)| {
            let p = &pts[k];
            let depth = u8::try_from(depth).ok()?;
            let raw = child(p.node, "t").map(|t| parse_txbody(t, &mut |r| sub.link(r)));
            let text = raw.map(|raw| resolve_textbox(&raw, &chain, &cx.env(RunProps::default()))).unwrap_or_default();
            Some(SmartArtNode { id: raw_key(p.model), depth, text })
        })
        .collect();
    let lo = lo.filter(|l| *l != dm);
    let (layout, category) = layout_of(&pts, lo, cx, id);
    Some(SmartArt { layout, category, nodes })
}

/// La version dessinée du SmartArt `id` (part `part`) : ses formes, placées dans `frame`.
fn drawn(part: &str, cx: &PartCtx<'_>, id: &str, frame: Frame, g: &GroupCtx) -> Vec<Item> {
    let Some(xml) = cx.part_text(part, id) else { return Vec::new() };
    let Some(doc) = cx.linked_doc(&xml, part, Some(id), WarningKind::UnreadableItem(clip(part))) else {
        return Vec::new();
    };
    let Some(tree) = descendant(doc.root_element(), "spTree") else { return Vec::new() };
    // Relations du dessin illisibles : le dessin reste lu, sans ses images (`UnreadableItem`).
    let Some(rels) = linked_rels(cx, part, id) else { return Vec::new() };
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
    parse_tree(tree, &sub, &inner).into_iter().map(|p| p.item).collect()
}

/// Le SmartArt du cadre `id` : son dessin en groupe (avec son modèle), son
/// modèle seul s'il n'a pas de dessin mais des nœuds avec texte, sinon un opaque.
pub(crate) fn smartart(data: Node<'_, '_>, cx: &PartCtx<'_>, id: &str, frame: Frame, g: &GroupCtx) -> ItemKind {
    let not_drawn = || opaque(OpaqueReason::SmartArt, None);
    if g.in_drawing || g.depth >= MAX_NESTING {
        cx.warn(Some(id), WarningKind::NestingTooDeep(clip(id)));
        return not_drawn();
    }
    let missing = || {
        if !cx.shared.sink.borrow().has_fatal() {
            cx.warn(Some(id), WarningKind::MissingPart("diagramDrawing".into()));
        }
        not_drawn()
    };
    let Some(dm) = rel_target(data, cx, "dm") else { return missing() };
    let Some(xml) = cx.part_text(&dm, id) else { return missing() };
    let Some(doc) = cx.linked_doc(&xml, &dm, Some(id), WarningKind::UnreadableItem(clip(&dm))) else {
        return missing();
    };
    let root = doc.root_element();
    let Some(model) = model(root, &dm, rel_target(data, cx, "lo").as_deref(), cx, id) else { return not_drawn() };
    let drawing = descendant(root, "dataModelExt").and_then(|e| attr_str(e, "relId")).and_then(|r| cx.rels.target(r));
    let items = drawing.map(|part| drawn(part, cx, id, frame, g)).unwrap_or_default();
    if cx.shared.sink.borrow().has_fatal() {
        return not_drawn();
    }
    if !items.is_empty() {
        ItemKind::Group { items, smartart: Some(Box::new(model)) }
    } else if model.texted().next().is_some() {
        ItemKind::SmartArt(Box::new(model))
    } else if drawing.is_none() {
        missing()
    } else {
        not_drawn()
    }
}

#[cfg(test)]
#[path = "smartart_tests.rs"]
mod smartart_tests;
