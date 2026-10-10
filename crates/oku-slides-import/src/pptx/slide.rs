//! Slides et notes.

use crate::collect::clip;
use crate::error::ImportError;
use crate::pptx::context::{PartCtx, PartSpec, Shared};
use crate::pptx::master::{LayoutInfo, MasterInfo, background, clr_map_override};
use crate::pptx::placeholder::PhInheritance;
use crate::pptx::shapes::{GroupCtx, parse_tree};
use crate::pptx::theme::ThemeData;
use crate::pptx::xml::{attr_bool, attr_str, child, descendant, elements, parse};
use crate::source::{Fill, SourceSlide, WarningKind};

/// Lit la slide `part`, n-ième (1..), sur `layout` (s'il est lisible) de `master`.
pub(crate) fn parse_slide(
    shared: &Shared<'_>,
    part: &str,
    n: u32,
    layout: Option<&LayoutInfo>,
    master: &MasterInfo,
    theme: &ThemeData<'_>,
) -> Result<SourceSlide, ImportError> {
    let xml = shared.pkg.text(part)?;
    let doc = parse(&xml, part, shared.pkg.limits().max_xml_nodes)?;
    let root = doc.root_element();
    let base = layout.map_or(&master.clr_map, |l| &l.clr_map);
    let inherit = PhInheritance { styles: &master.styles, master: Some(&master.phs), layout: layout.map(|l| &l.phs) };
    let spec = PartSpec {
        owner: format!("s{n}"),
        slide: Some(n),
        colors: theme.color_ctx(clr_map_override(root, base)),
        theme,
        inherit: Some(&inherit),
    };
    let cx = PartCtx::new(shared, part, spec)?;
    let c_sld = child(root, "cSld");
    let items = c_sld.and_then(|c| child(c, "spTree")).map(|t| parse_tree(t, &cx, &GroupCtx::ROOT)).unwrap_or_default();
    // Fond propre, sinon hérité ; résolu avec les couleurs de la slide (son clrMapOvr compris).
    let bg: Fill = c_sld
        .and_then(|c| background(c, &cx))
        .or_else(|| layout.and_then(|l| l.bg.clone()))
        .or_else(|| master.bg.clone())
        .map_or(Fill::None, |b| b.resolve(&cx.colors));
    Ok(SourceSlide {
        id: format!("s{n}"),
        number: n,
        layout: layout.map(|l| l.layout.id.clone()),
        background: bg,
        show_layout_decor: attr_bool(root, "showMasterSp").unwrap_or(true),
        hidden: attr_bool(root, "show") == Some(false),
        items: items.into_iter().map(|p| p.item).collect(),
        notes: notes_text(&cx),
    })
}

/// Texte des notes de la slide (paragraphes joints par `\n`, `a:br` → `\n`), vide sans notes.
pub(crate) fn notes_text(cx: &PartCtx<'_>) -> String {
    let Some((_, part)) = cx.rels.by_type("/notesSlide").first().copied() else { return String::new() };
    let Some(xml) = cx.part_text(part, "notes") else { return String::new() };
    let Some(doc) = cx.linked_doc(&xml, part, None, WarningKind::UnreadablePart(clip(part))) else {
        return String::new();
    };
    let body = doc.root_element().descendants().find(|n| {
        n.tag_name().name() == "sp" && descendant(*n, "ph").is_some_and(|ph| attr_str(ph, "type") == Some("body"))
    });
    let Some(tx) = body.and_then(|b| child(b, "txBody")) else { return String::new() };
    elements(tx)
        .filter(|p| p.tag_name().name() == "p")
        .map(|p| {
            elements(p)
                .filter_map(|r| match r.tag_name().name() {
                    "r" | "fld" => child(r, "t").and_then(|t| t.text()).map(str::to_owned),
                    "br" => Some("\n".to_owned()),
                    _ => None,
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
