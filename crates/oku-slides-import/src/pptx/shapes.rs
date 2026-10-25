//! Arbre de formes (`spTree`, `grpSp`, `dsp:spTree`) → items.

use roxmltree::Node;

use crate::collect::{clip, clip_to};
use crate::pptx::cascade::resolve_textbox;
use crate::pptx::color::color_spec_in;
use crate::pptx::context::PartCtx;
use crate::pptx::fill::{FillSpec, LineSpec, approximation, parse_fill, parse_line};
use crate::pptx::frame::parse_graphic_frame;
use crate::pptx::geometry::parse_geometry;
use crate::pptx::placeholder::{Inherited, PhKey};
use crate::pptx::text::{BodyProps, LevelProps, RunProps, empty_levels, font_ref, parse_txbody};
use crate::pptx::units::{angle, emu, pct};
use crate::pptx::xml::{attr_bool, attr_i64, attr_r, attr_str, attr_u32, child, descendant, elements};
use crate::source::{
    Connection, Crop, Fill, Frame, Geometry, Item, ItemKind, Opaque, OpaqueReason, Picture, Shape, Site, TextBox,
    WarningKind,
};

/// Profondeur d'imbrication (groupes, `AlternateContent`) au-delà de laquelle on coupe.
pub(crate) const MAX_NESTING: u8 = 32;
const MAX_COORD: f64 = 1e6;
const MIN_SCALE: f64 = 1e-6;
const MAX_SCALE: f64 = 1e6;
/// Espaces de noms d'extension dont on sait lire le `mc:Choice` (a14, p14, p15, a16, asvg).
const KNOWN_NS: [&str; 5] = [
    "http://schemas.microsoft.com/office/drawing/2010/main",
    "http://schemas.microsoft.com/office/powerpoint/2010/main",
    "http://schemas.microsoft.com/office/powerpoint/2012/main",
    "http://schemas.microsoft.com/office/drawing/2014/main",
    "http://schemas.microsoft.com/office/drawing/2016/SVG/main",
];

/// Déclaration de placeholder d'un item : clé et propriétés propres (non résolues).
#[derive(Debug, Clone)]
pub(crate) struct PhDecl {
    pub key: PhKey,
    pub lst: LevelProps,
    pub body: BodyProps,
    pub fill: Option<FillSpec>,
    pub line: LineSpec,
    pub geom: Option<Geometry>,
}

/// Item lu et, s'il en est un, sa déclaration de placeholder.
#[derive(Debug, Clone)]
pub(crate) struct ParsedItem {
    pub item: Item,
    pub ph: Option<PhDecl>,
}

/// Transformation de groupe (pt), remplissage de groupe, profondeur, « dans un dessin de SmartArt ».
#[derive(Debug, Clone)]
pub(crate) struct GroupCtx {
    pub sx: f64,
    pub sy: f64,
    pub tx: f64,
    pub ty: f64,
    pub fill: Option<FillSpec>,
    pub depth: u8,
    pub in_drawing: bool,
}

fn bound(v: f64) -> f64 {
    if v.is_finite() { v.clamp(-MAX_COORD, MAX_COORD) } else { 0.0 }
}

fn pt_pair(n: Option<Node<'_, '_>>, a: &str, b: &str) -> (f64, f64) {
    n.map_or((0.0, 0.0), |n| (emu(attr_i64(n, a).unwrap_or(0)), emu(attr_i64(n, b).unwrap_or(0))))
}

impl GroupCtx {
    /// Pas de groupe.
    pub(crate) const ROOT: GroupCtx =
        GroupCtx { sx: 1.0, sy: 1.0, tx: 0.0, ty: 0.0, fill: None, depth: 0, in_drawing: false };

    /// Cadre de `xfrm` dans l'espace de la page.
    pub(crate) fn frame(&self, xfrm: Node<'_, '_>) -> Frame {
        let (x, y) = pt_pair(child(xfrm, "off"), "x", "y");
        let (w, h) = pt_pair(child(xfrm, "ext"), "cx", "cy");
        Frame {
            x: bound(self.tx + x * self.sx),
            y: bound(self.ty + y * self.sy),
            w: bound(w * self.sx),
            h: bound(h * self.sy),
            rot: attr_i64(xfrm, "rot").map_or(0.0, angle).rem_euclid(360.0),
            flip_h: attr_bool(xfrm, "flipH").unwrap_or(false),
            flip_v: attr_bool(xfrm, "flipV").unwrap_or(false),
        }
    }

    /// Contexte des enfants d'un groupe de `xfrm` donné.
    pub(crate) fn nested(&self, xfrm: Node<'_, '_>, fill: Option<FillSpec>) -> GroupCtx {
        let (ox, oy) = pt_pair(child(xfrm, "off"), "x", "y");
        let (ew, eh) = pt_pair(child(xfrm, "ext"), "cx", "cy");
        let (cx, cy) = pt_pair(child(xfrm, "chOff"), "x", "y");
        let (cw, ch) = pt_pair(child(xfrm, "chExt"), "cx", "cy");
        let scale = |e: f64, c: f64| if c > 0.0 && e > 0.0 { (e / c).clamp(MIN_SCALE, MAX_SCALE) } else { 1.0 };
        let (sx, sy) = (scale(ew, cw), scale(eh, ch));
        GroupCtx {
            sx: (self.sx * sx).clamp(MIN_SCALE, MAX_SCALE),
            sy: (self.sy * sy).clamp(MIN_SCALE, MAX_SCALE),
            tx: bound(self.tx + self.sx * (ox - cx * sx)),
            ty: bound(self.ty + self.sy * (oy - cy * sy)),
            fill: fill.or_else(|| self.fill.clone()),
            depth: self.depth.saturating_add(1),
            in_drawing: self.in_drawing,
        }
    }

    fn deeper(&self) -> GroupCtx {
        GroupCtx { depth: self.depth.saturating_add(1), ..self.clone() }
    }
}

fn nv<'a, 'i>(n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
    elements(n).find(|c| c.tag_name().name().starts_with("nv"))
}

fn ph_key(n: Node<'_, '_>) -> Option<PhKey> {
    nv(n).and_then(|v| child(v, "nvPr")).and_then(|p| child(p, "ph")).map(PhKey::from_ph)
}

/// (id, nom, texte alternatif) d'un item, ou `None` s'il est caché ; `modelId` (SmartArt) prime sur `cNvPr@id`.
pub(crate) fn identity(n: Node<'_, '_>, cx: &PartCtx<'_>) -> Option<(String, String, Option<String>)> {
    let c = nv(n).and_then(|v| child(v, "cNvPr"))?;
    if attr_bool(c, "hidden") == Some(true) {
        return None;
    }
    let id = match attr_str(n, "modelId") {
        Some(m) => cx.item_id_raw(m),
        None => cx.item_id(attr_str(c, "id").unwrap_or("0")),
    };
    let alt = attr_str(c, "descr").filter(|d| !d.is_empty()).map(|d| clip_to(d, 2048));
    Some((id, clip(attr_str(c, "name").unwrap_or_default()), alt))
}

/// Item opaque.
pub(crate) fn opaque(reason: OpaqueReason, fallback: Option<String>) -> ItemKind {
    ItemKind::Opaque(Opaque { reason, fallback })
}

fn has_text(tb: &TextBox) -> bool {
    tb.paragraphs.iter().any(|p| p.runs.iter().any(|r| !r.text.trim().is_empty()))
}

fn report_sp_pr(sp_pr: Node<'_, '_>, id: &str, cx: &PartCtx<'_>) {
    if child(sp_pr, "effectLst").is_some_and(|e| elements(e).next().is_some()) {
        cx.warn(Some(id), WarningKind::EffectIgnored("effectLst".into()));
    }
    for e in ["scene3d", "sp3d"] {
        if child(sp_pr, e).is_some() {
            cx.warn(Some(id), WarningKind::EffectIgnored(e.into()));
        }
    }
    if let Some(a) = approximation(sp_pr) {
        cx.warn(Some(id), WarningKind::Approximated(a.into()));
    }
}

/// Références de style (`p:style`) : remplissage (si nécessaire), trait, police et couleur du texte.
fn style_refs(n: Node<'_, '_>, cx: &PartCtx<'_>, need_fill: bool) -> (Option<FillSpec>, LineSpec, RunProps) {
    let Some(style) = child(n, "style") else { return (None, LineSpec::default(), RunProps::default()) };
    let idx = |r: Node<'_, '_>| attr_u32(r, "idx").unwrap_or(0);
    let fill = if need_fill {
        child(style, "fillRef")
            .and_then(|r| cx.theme.fill_style(idx(r), color_spec_in(r).as_ref(), &mut |id| cx.asset(id)))
    } else {
        None
    };
    let line =
        child(style, "lnRef").map(|r| cx.theme.line_style(idx(r), color_spec_in(r).as_ref())).unwrap_or_default();
    let font = child(style, "fontRef").map(|r| RunProps { color: color_spec_in(r), ..font_ref(r) }).unwrap_or_default();
    (fill, line, font)
}

/// Ce que `sp` et `pic` lisent de même : identité, placeholder et son héritage, `spPr` (signalé)
/// et cadre (le sien, sinon l'hérité, sinon vide).
struct Head<'a, 'i> {
    id: String,
    name: String,
    alt: Option<String>,
    ph: Option<PhKey>,
    inh: Inherited,
    sp_pr: Option<Node<'a, 'i>>,
    frame: Frame,
}

fn head<'a, 'i>(n: Node<'a, 'i>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<Head<'a, 'i>> {
    let (id, name, alt) = identity(n, cx)?;
    let ph = ph_key(n);
    let inh = cx.lookup(ph.as_ref());
    let sp_pr = child(n, "spPr");
    let frame = sp_pr
        .and_then(|s| child(s, "xfrm"))
        .map(|x| g.frame(x))
        .or(inh.frame)
        .unwrap_or(Frame::new(0.0, 0.0, 0.0, 0.0));
    if let Some(s) = sp_pr {
        report_sp_pr(s, &id, cx);
    }
    Some(Head { id, name, alt, ph, inh, sp_pr, frame })
}

fn parse_sp(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx, connector: bool) -> Option<ParsedItem> {
    let Head { id, name, alt, ph, inh, sp_pr, frame } = head(n, cx, g)?;
    if child(n, "txXfrm").is_some() {
        cx.warn(Some(&id), WarningKind::Approximated("txXfrm".into()));
    }
    let own_fill = match sp_pr {
        Some(s) if child(s, "grpFill").is_some() => g.fill.clone(),
        Some(s) => parse_fill(s, &mut |r| cx.asset(r)),
        None => None,
    };
    let own_line = sp_pr.and_then(|s| child(s, "ln")).map(parse_line).unwrap_or_default();
    let own_geom = sp_pr.and_then(parse_geometry);
    let (fill_ref, line_ref, font) = style_refs(n, cx, own_fill.is_none() && inh.fill.is_none());
    let fill: Fill =
        own_fill.clone().or_else(|| inh.fill.clone()).or(fill_ref).map_or(Fill::None, |f| f.resolve(&cx.colors));
    let stroke = own_line.clone().over(&inh.line.clone().over(&line_ref)).finish(&cx.colors);
    let geom = own_geom
        .clone()
        .or_else(|| inh.geom.clone())
        .unwrap_or(Geometry::Preset { name: "rect".into(), adj: std::collections::BTreeMap::default() });
    let raw = if connector { None } else { child(n, "txBody").map(|tx| parse_txbody(tx, &mut |r| cx.link(r))) };
    let tb = raw.as_ref().map(|raw| {
        let chain = inh.chain();
        resolve_textbox(raw, &chain, &cx.env(font))
    });
    let texty = tb.as_ref().is_some_and(has_text);
    let is_rect = matches!(&geom, Geometry::Preset { name, .. } if name == "rect");
    let kind = if !connector && fill.is_none() && stroke.is_none() && is_rect {
        if !texty && ph.is_none() {
            return None;
        }
        ItemKind::Text(tb.unwrap_or_default())
    } else {
        let connection = if connector { connection(n, cx) } else { None };
        ItemKind::Shape(Box::new(Shape { geom, fill, stroke, text: tb.filter(has_text), connection }))
    };
    let decl = ph.clone().map(|key| PhDecl {
        key,
        lst: raw.as_ref().map_or_else(empty_levels, |r| r.lst.clone()),
        body: raw.map(|r| r.body).unwrap_or_default(),
        fill: own_fill,
        line: own_line,
        geom: own_geom,
    });
    let item = Item { id, name, alt, frame, z: 0, role: ph.map(|k| k.role()), placeholder: inh.link, kind };
    Some(ParsedItem { item, ph: decl })
}

/// Formes reliées par le connecteur `n` (`cNvCxnSpPr` : `stCxn`, `endCxn`) ; `None` s'il n'en relie aucune.
fn connection(n: Node<'_, '_>, cx: &PartCtx<'_>) -> Option<Connection> {
    let pr = nv(n).and_then(|v| child(v, "cNvCxnSpPr"))?;
    let site = |name: &str| {
        let s = child(pr, name)?;
        Some(Site { item: cx.item_ref(attr_u32(s, "id")?), idx: attr_u32(s, "idx").unwrap_or(0) })
    };
    let (start, end) = (site("stCxn"), site("endCxn"));
    (start.is_some() || end.is_some()).then_some(Connection { start, end })
}

/// Asset d'un `blip` : sa version SVG, sinon (absente, illisible ou rejetée) son raster.
pub(crate) fn blip_asset(blip: Node<'_, '_>, cx: &PartCtx<'_>) -> Option<String> {
    let svg = descendant(blip, "svgBlip").and_then(|s| attr_r(s, "embed")).and_then(|rid| cx.asset(rid));
    svg.or_else(|| attr_r(blip, "embed").and_then(|rid| cx.asset(rid)))
}

/// Paire de bords de rognage : chacun dans `[-1, 1]`, remise à 0 si leur somme laisse une image vide.
fn crop_pair(a: f64, b: f64) -> (f64, f64) {
    let (a, b) = (a.clamp(-1.0, 1.0), b.clamp(-1.0, 1.0));
    if a + b >= 1.0 { (0.0, 0.0) } else { (a, b) }
}

fn parse_pic(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    let Head { id, name, alt, ph, inh, sp_pr: _, frame } = head(n, cx, g)?;
    let blip_fill = child(n, "blipFill");
    let blip = blip_fill.and_then(|fill| child(fill, "blip"));
    let asset = blip.and_then(|blip| blip_asset(blip, cx));
    let nv_pr = nv(n).and_then(|nv| child(nv, "nvPr"));
    let media = nv_pr.is_some_and(|pr| {
        ["videoFile", "audioFile", "quickTimeFile"].iter().any(|tag| child(pr, tag).is_some())
            || descendant(pr, "media").is_some()
    });
    let kind = if media {
        opaque(OpaqueReason::Media, asset)
    } else {
        let crop = blip_fill.and_then(|fill| child(fill, "srcRect")).map_or_else(Crop::default, |rect| {
            let side = |name: &str| attr_i64(rect, name).map_or(0.0, pct);
            let ((left, right), (top, bottom)) = (crop_pair(side("l"), side("r")), crop_pair(side("t"), side("b")));
            Crop { l: left, t: top, r: right, b: bottom }
        });
        let Some(asset) = asset else {
            // Image seulement liée (`r:link`) : hors du fichier, donc perdue, mais signalée.
            let link = blip.filter(|blip| attr_r(*blip, "embed").is_none()).and_then(|blip| attr_r(blip, "link"));
            if let Some(rid) = link {
                let target = cx.external_target(rid).unwrap_or_else(|| clip(rid));
                cx.warn(Some(&id), WarningKind::ExternalImage(target));
            }
            return None;
        };
        ItemKind::Picture(Picture { asset, crop })
    };
    let decl = ph.clone().map(|key| PhDecl {
        key,
        lst: empty_levels(),
        body: BodyProps::default(),
        fill: None,
        line: LineSpec::default(),
        geom: None,
    });
    let item = Item { id, name, alt, frame, z: 0, role: ph.map(|key| key.role()), placeholder: inh.link, kind };
    Some(ParsedItem { item, ph: decl })
}

fn too_deep(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> ParsedItem {
    let (id, name, alt) = identity(n, cx).unwrap_or_else(|| (cx.item_id("x"), String::new(), None));
    cx.warn(Some(&id), WarningKind::NestingTooDeep(id.clone()));
    let frame =
        child(n, "grpSpPr").and_then(|p| child(p, "xfrm")).map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    let kind = opaque(OpaqueReason::Unknown("nesting".into()), None);
    ParsedItem { item: Item { id, name, alt, frame, z: 0, role: None, placeholder: None, kind }, ph: None }
}

fn parse_group(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Option<ParsedItem> {
    if g.depth >= MAX_NESTING {
        return Some(too_deep(n, cx, g));
    }
    let (id, name, alt) = identity(n, cx)?;
    let gp = child(n, "grpSpPr");
    let xfrm = gp.and_then(|p| child(p, "xfrm"));
    let frame = xfrm.map_or(Frame::new(0.0, 0.0, 0.0, 0.0), |x| g.frame(x));
    if frame.rot != 0.0 {
        cx.warn(Some(&id), WarningKind::EffectIgnored("group rotation".into()));
    }
    if frame.flip_h || frame.flip_v {
        cx.warn(Some(&id), WarningKind::EffectIgnored("group flip".into()));
    }
    let fill = gp.and_then(|p| parse_fill(p, &mut |r| cx.asset(r)));
    let inner = xfrm.map_or_else(|| g.deeper(), |x| g.nested(x, fill));
    let items: Vec<Item> = parse_tree(n, cx, &inner).into_iter().map(|p| p.item).collect();
    if items.is_empty() {
        return None;
    }
    Some(ParsedItem {
        item: Item {
            id,
            name,
            alt,
            frame,
            z: 0,
            role: None,
            placeholder: None,
            kind: ItemKind::Group { items, smartart: None },
        },
        ph: None,
    })
}

/// `mc:Choice` lisible : chaque préfixe de `Requires` désigne un espace de noms connu.
fn understood(choice: Node<'_, '_>) -> bool {
    attr_str(choice, "Requires").is_some_and(|req| {
        req.split_whitespace().all(|p| choice.lookup_namespace_uri(Some(p)).is_some_and(|ns| KNOWN_NS.contains(&ns)))
    })
}

fn parse_alternate(n: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Vec<ParsedItem> {
    if g.depth >= MAX_NESTING {
        cx.warn(None, WarningKind::NestingTooDeep("AlternateContent".into()));
        return Vec::new();
    }
    let inner = g.deeper();
    let choice = child(n, "Choice");
    let fallback = child(n, "Fallback");
    let is_math = choice
        .is_some_and(|c| c.descendants().take(10_000).any(|d| matches!(d.tag_name().name(), "oMath" | "oMathPara")));
    if is_math {
        let Some(fb) = fallback else { return Vec::new() };
        let asset = descendant(fb, "blip").and_then(|b| blip_asset(b, cx));
        let mut items = parse_tree(fb, cx, &inner);
        for p in &mut items {
            p.item.kind = opaque(OpaqueReason::Math, asset.clone());
        }
        return items;
    }
    // La branche est choisie avant d'être lue : l'autre ne consomme ni id, ni asset, ni erreur.
    let branch = match (choice, fallback) {
        (Some(c), _) if understood(c) => Some(c),
        (_, Some(f)) => Some(f),
        (c, None) => c,
    };
    branch.map(|b| parse_tree(b, cx, &inner)).unwrap_or_default()
}

/// Items d'un arbre de formes, dans l'ordre de dessin.
pub(crate) fn parse_tree(tree: Node<'_, '_>, cx: &PartCtx<'_>, g: &GroupCtx) -> Vec<ParsedItem> {
    let mut out: Vec<ParsedItem> = Vec::new();
    for n in elements(tree) {
        match n.tag_name().name() {
            "sp" => out.extend(parse_sp(n, cx, g, false)),
            "cxnSp" => out.extend(parse_sp(n, cx, g, true)),
            "pic" => out.extend(parse_pic(n, cx, g)),
            "grpSp" => out.extend(parse_group(n, cx, g)),
            "graphicFrame" => out.extend(parse_graphic_frame(n, cx, g)),
            "AlternateContent" => out.extend(parse_alternate(n, cx, g)),
            _ => {}
        }
    }
    for (z, p) in out.iter_mut().enumerate() {
        p.item.z = u32::try_from(z).unwrap_or(u32::MAX);
    }
    out
}

#[cfg(test)]
#[path = "shapes_tests.rs"]
mod shapes_tests;
