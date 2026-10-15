//! Masters et layouts : décor, placeholders, styles, fond, `clrMap`.

use std::collections::BTreeMap;

use roxmltree::Node;

use crate::error::ImportError;
use crate::pptx::bounds::warn_clamped;
use crate::pptx::cascade::{TextChain, resolve_level, resolve_levels};
use crate::pptx::color::{clean_map, color_spec_in, default_map};
use crate::pptx::context::{PartCtx, PartSpec, Shared};
use crate::pptx::fill::{FillSpec, parse_fill};
use crate::pptx::placeholder::{MAX_PLACEHOLDERS, PhInfo, PhInheritance, PhTable, TextStyles};
use crate::pptx::shapes::{GroupCtx, ParsedItem, parse_tree};
use crate::pptx::text::{LevelProps, RunProps, empty_levels, parse_lst_style};
use crate::pptx::theme::ThemeData;
use crate::pptx::xml::{attr_bool, attr_str, attr_u32, child, parse};
use crate::source::{Fill, Item, ItemKind, Layout, Master, MasterTextStyles, Placeholder, TextBox};

/// Master lu et ce que ses layouts et slides en héritent.
#[derive(Debug)]
pub(crate) struct MasterInfo {
    pub master: Master,
    pub styles: TextStyles,
    pub phs: PhTable,
    pub clr_map: BTreeMap<String, String>,
    /// Fond non résolu (résolu par chaque slide).
    pub bg: Option<FillSpec>,
}

/// Layout lu et ce que ses slides en héritent.
#[derive(Debug)]
pub(crate) struct LayoutInfo {
    pub layout: Layout,
    pub phs: PhTable,
    pub clr_map: BTreeMap<String, String>,
    pub bg: Option<FillSpec>,
}

/// `clrMapOvr/overrideClrMapping` d'un layout ou d'une slide, sinon `base`.
pub(crate) fn clr_map_override(root: Node<'_, '_>, base: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    match child(root, "clrMapOvr").and_then(|o| child(o, "overrideClrMapping")) {
        Some(o) => clean_map(o.attributes().map(|a| (a.name(), a.value())), base),
        None => base.clone(),
    }
}

/// Fond d'un `cSld`, non résolu : `bgPr`, ou `bgRef` via le thème.
pub(crate) fn background(c_sld: Node<'_, '_>, cx: &PartCtx<'_>) -> Option<FillSpec> {
    let bg = child(c_sld, "bg")?;
    if let Some(pr) = child(bg, "bgPr") {
        return parse_fill(pr, &mut |r| cx.asset(r));
    }
    let r = child(bg, "bgRef")?;
    cx.theme.fill_style(attr_u32(r, "idx").unwrap_or(0), color_spec_in(r).as_ref(), &mut |id| cx.asset(id))
}

/// Table des placeholders déclarés parmi `items`.
pub(crate) fn table_of(items: &[ParsedItem]) -> PhTable {
    let entries = items
        .iter()
        .filter_map(|p| {
            let d = p.ph.as_ref()?;
            Some(PhInfo {
                id: p.item.id.clone(),
                key: d.key.clone(),
                frame: p.item.frame,
                lst: d.lst.clone(),
                body: d.body.clone(),
                fill: d.fill.clone(),
                line: d.line.clone(),
                geom: d.geom.clone(),
            })
        })
        .take(MAX_PLACEHOLDERS)
        .collect();
    PhTable { entries }
}

/// Sépare décor et placeholders ; résout le style de chaque niveau des placeholders.
pub(crate) fn placeholders(items: Vec<ParsedItem>, cx: &PartCtx<'_>) -> (Vec<Item>, Vec<Placeholder>) {
    let mut decor = Vec::new();
    let mut phs = Vec::new();
    for p in items {
        let Some(decl) = p.ph else {
            decor.push(p.item);
            continue;
        };
        let inh = cx.lookup(Some(&decl.key));
        let chain = TextChain { levels: inh.levels.iter().collect(), bodies: inh.bodies.iter().collect() };
        let levels = resolve_levels(&chain, &decl.lst, &cx.env(RunProps::default()));
        let text = match p.item.kind {
            ItemKind::Text(tb) => tb,
            ItemKind::Shape(s) => s.text.unwrap_or_default(),
            _ => TextBox::default(),
        };
        phs.push(Placeholder {
            id: p.item.id,
            role: decl.key.role(),
            idx: decl.key.idx,
            frame: p.item.frame,
            levels,
            text,
        });
    }
    (decor, phs)
}

fn style(root: Node<'_, '_>, name: &str) -> LevelProps {
    child(root, "txStyles").and_then(|t| child(t, name)).map_or_else(empty_levels, parse_lst_style)
}

/// Lit le master `part` (n-ième, 1..), dont le thème est `themes[theme_index]`.
pub(crate) fn parse_master(
    shared: &Shared<'_>,
    part: &str,
    n: usize,
    theme_index: usize,
    theme: &ThemeData<'_>,
) -> Result<MasterInfo, ImportError> {
    let xml = shared.pkg.text(part)?;
    let doc = parse(&xml, part, shared.pkg.limits().max_xml_nodes)?;
    let root = doc.root_element();
    let map = match child(root, "clrMap") {
        Some(m) => clean_map(m.attributes().map(|a| (a.name(), a.value())), &default_map()),
        None => default_map(),
    };
    let styles = TextStyles {
        pres: shared.pres.clone(),
        title: style(root, "titleStyle"),
        body: style(root, "bodyStyle"),
        other: style(root, "otherStyle"),
    };
    let inherit = PhInheritance { styles: &styles, master: None, layout: None };
    let id = format!("m{n}");
    let spec = PartSpec {
        owner: id.clone(),
        slide: None,
        colors: theme.color_ctx(map.clone()),
        theme,
        inherit: Some(&inherit),
    };
    let cx = PartCtx::new(shared, part, spec)?;
    warn_clamped(&cx, root);
    let c_sld = child(root, "cSld");
    let items = c_sld.and_then(|c| child(c, "spTree")).map(|t| parse_tree(t, &cx, &GroupCtx::ROOT)).unwrap_or_default();
    let phs = table_of(&items);
    let bg = c_sld.and_then(|c| background(c, &cx));
    let background: Fill = bg.as_ref().map_or(Fill::None, |b| b.resolve(&cx.colors));
    let name = crate::collect::clip(c_sld.and_then(|c| attr_str(c, "name")).unwrap_or_default());
    let (decor, placeholders) = placeholders(items, &cx);
    let env = cx.env(RunProps::default());
    let first_level = |props: &LevelProps| {
        let chain = TextChain { levels: vec![&styles.pres, props], bodies: Vec::new() };
        resolve_level(&chain, &empty_levels(), &env, 0)
    };
    let text_styles = MasterTextStyles {
        title: first_level(&styles.title),
        body: first_level(&styles.body),
        other: first_level(&styles.other),
    };
    let master = Master { id, name, theme: theme_index, background, decor, placeholders, text_styles };
    Ok(MasterInfo { master, styles, phs, clr_map: map, bg })
}

/// Lit le layout `part` (n-ième, 1..) de `master`.
pub(crate) fn parse_layout(
    shared: &Shared<'_>,
    part: &str,
    n: usize,
    master: &MasterInfo,
    theme: &ThemeData<'_>,
) -> Result<LayoutInfo, ImportError> {
    let xml = shared.pkg.text(part)?;
    let doc = parse(&xml, part, shared.pkg.limits().max_xml_nodes)?;
    let root = doc.root_element();
    let map = clr_map_override(root, &master.clr_map);
    let inherit = PhInheritance { styles: &master.styles, master: Some(&master.phs), layout: None };
    let id = format!("l{n}");
    let spec = PartSpec {
        owner: id.clone(),
        slide: None,
        colors: theme.color_ctx(map.clone()),
        theme,
        inherit: Some(&inherit),
    };
    let cx = PartCtx::new(shared, part, spec)?;
    warn_clamped(&cx, root);
    let c_sld = child(root, "cSld");
    let items = c_sld.and_then(|c| child(c, "spTree")).map(|t| parse_tree(t, &cx, &GroupCtx::ROOT)).unwrap_or_default();
    let phs = table_of(&items);
    let bg = c_sld.and_then(|c| background(c, &cx));
    let name = crate::collect::clip(c_sld.and_then(|c| attr_str(c, "name")).unwrap_or_default());
    let (decor, placeholders) = placeholders(items, &cx);
    let layout = Layout {
        id,
        name,
        kind: crate::collect::clip_to(attr_str(root, "type").unwrap_or("cust"), 32),
        master: master.master.id.clone(),
        background: bg.as_ref().map(|b| b.resolve(&cx.colors)),
        show_master_decor: attr_bool(root, "showMasterSp").unwrap_or(true),
        decor,
        placeholders,
    };
    Ok(LayoutInfo { layout, phs, clr_map: map, bg })
}

#[cfg(test)]
#[path = "master_tests.rs"]
mod master_tests;
