//! Lecteur PPTX : du paquet OOXML à la [`crate::source::SourceDeck`].

pub(crate) mod bounds;
pub(crate) mod cascade;
pub(crate) mod chart;
pub(crate) mod color;
pub(crate) mod context;
pub(crate) mod eot;
pub(crate) mod fill;
pub(crate) mod fonts;
pub(crate) mod frame;
pub(crate) mod geometry;
pub(crate) mod master;
pub(crate) mod media;
pub(crate) mod package;
pub(crate) mod placeholder;
pub(crate) mod shapes;
pub(crate) mod slide;
pub(crate) mod smartart;
pub(crate) mod svg;
pub(crate) mod table;
pub(crate) mod table_styles;
#[cfg(test)]
pub(crate) mod test_support;
pub(crate) mod text;
pub(crate) mod theme;
pub(crate) mod units;
pub(crate) mod xml;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use roxmltree::Node;

use crate::collect::{Sink, clip};
use crate::error::{ImportError, structure, too_large};
use crate::pptx::context::Shared;
use crate::pptx::master::{LayoutInfo, MasterInfo, parse_layout, parse_master};
use crate::pptx::package::{Package, Rels};
use crate::pptx::slide::parse_slide;
use crate::pptx::text::{empty_levels, parse_lst_style};
use crate::pptx::theme::ThemeData;
use crate::pptx::units::emu;
use crate::pptx::xml::{attr_i64, attr_r, child, children, parse};
use crate::source::{Size, SourceDeck, SourceSlide, WarningKind};
use crate::{ImportErrorKind, ReadLimits};

/// `Ok(Some)` si lu ; `Ok(None)` + `UnreadablePart` pour une erreur non fatale ; `Err` pour une limite dépassée.
fn soft<T>(r: Result<T, ImportError>, sink: &RefCell<Sink>, part: &str) -> Result<Option<T>, ImportError> {
    match r {
        Ok(v) => Ok(Some(v)),
        Err(e) if e.kind() == ImportErrorKind::TooLarge => Err(e),
        Err(_) => {
            sink.borrow_mut().warn(None, None, WarningKind::UnreadablePart(clip(part)));
            Ok(None)
        }
    }
}

/// Rend l'erreur fatale notée pendant la lecture d'une part.
fn check(shared: &Shared<'_>) -> Result<(), ImportError> {
    if !shared.sink.borrow().has_fatal() {
        return Ok(());
    }
    std::mem::take(&mut *shared.sink.borrow_mut()).finish().map(|_| ())
}

/// Cibles des relations listées par `list/item@r:id`, sans doublon.
fn listed(root: Node<'_, '_>, list: &str, item: &str, rels: &Rels) -> Vec<String> {
    let mut seen = BTreeSet::new();
    child(root, list)
        .into_iter()
        .flat_map(|l| children(l, item))
        .filter_map(|m| rels.target(attr_r(m, "id")?).map(str::to_owned))
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

/// Textes des thèmes, une fois par part, et pour chaque master l'index de son thème
/// (`None` si absent ou illisible, déjà signalé).
type ThemeTexts = (Vec<(String, Option<String>)>, Vec<Option<usize>>);

/// Lit le thème de chaque master ; un thème partagé par plusieurs masters n'est lu qu'une fois.
fn theme_texts(shared: &Shared<'_>, masters: &[String]) -> Result<ThemeTexts, ImportError> {
    let (mut texts, mut of_master) = (Vec::new(), Vec::new());
    let mut by_part: BTreeMap<String, usize> = BTreeMap::new();
    for mp in masters {
        let tp = soft(shared.pkg.rels(mp), &shared.sink, mp)?
            .and_then(|r| r.by_type("/theme").first().map(|(_, t)| (*t).to_owned()));
        let Some(tp) = tp else {
            shared.sink.borrow_mut().warn(None, None, WarningKind::UnreadablePart(clip(&format!("{mp}: theme"))));
            of_master.push(None);
            continue;
        };
        if let Some(k) = by_part.get(&tp) {
            of_master.push(Some(*k));
            continue;
        }
        let text = soft(shared.pkg.text(&tp), &shared.sink, &tp)?;
        by_part.insert(tp.clone(), texts.len());
        of_master.push(Some(texts.len()));
        texts.push((tp, text));
    }
    Ok((texts, of_master))
}

/// Thème de chaque master (même index), chaque part analysée une fois ; thème Office si absent ou illisible.
fn themes<'t>(shared: &Shared<'_>, texts: &'t ThemeTexts) -> Result<Vec<Rc<ThemeData<'t>>>, ImportError> {
    let (parts, of_master) = texts;
    let mut parsed = Vec::new();
    for (part, text) in parts {
        parsed.push(match text {
            Some(x) => {
                soft(ThemeData::parse(x, part, shared.pkg.limits().max_xml_nodes), &shared.sink, part)?.map(Rc::new)
            }
            None => None,
        });
    }
    let mut office: Option<Rc<ThemeData<'t>>> = None;
    let mut out = Vec::new();
    for k in of_master {
        let theme = match k.and_then(|k| parsed.get(k).cloned().flatten()) {
            Some(t) => t,
            None => match &office {
                Some(o) => Rc::clone(o),
                None => Rc::clone(office.insert(Rc::new(ThemeData::office()?))),
            },
        };
        out.push(theme);
    }
    Ok(out)
}

type Layouts = Vec<(String, usize, LayoutInfo)>; // (part, index dans les masters, layout)

/// Masters utilisables, leurs layouts lus, et pour chaque layout cité (lu ou non) l'index du
/// premier master utilisable qui le cite.
struct Masters {
    masters: Vec<MasterInfo>,
    layouts: Layouts,
    owner: BTreeMap<String, usize>,
}

/// Masters utilisables et leurs layouts.
fn masters(shared: &Shared<'_>, parts: &[String], themes: &[Rc<ThemeData<'_>>]) -> Result<Masters, ImportError> {
    let mut masters = Vec::new();
    let mut layouts: Layouts = Vec::new();
    let mut owner = BTreeMap::new();
    for (i, (mp, theme)) in parts.iter().zip(themes).enumerate() {
        let Some(m) = soft(parse_master(shared, mp, i + 1, i, theme), &shared.sink, mp)? else { continue };
        check(shared)?;
        // Un layout cité deux fois par le même master n'est lu qu'une fois ; partagé entre masters, il est lu par chacun.
        let mut seen = BTreeSet::new();
        let layout_parts: Vec<String> = soft(shared.pkg.rels(mp), &shared.sink, mp)?
            .map(|r| {
                r.by_type("/slideLayout")
                    .into_iter()
                    .map(|(_, t)| t.to_owned())
                    .filter(|t| seen.insert(t.clone()))
                    .collect()
            })
            .unwrap_or_default();
        for lp in layout_parts {
            owner.entry(lp.clone()).or_insert(masters.len());
            if let Some(l) = soft(parse_layout(shared, &lp, layouts.len() + 1, &m, theme), &shared.sink, &lp)? {
                layouts.push((lp, masters.len(), l));
            }
            check(shared)?;
        }
        masters.push(m);
    }
    if masters.is_empty() {
        return Err(structure("no usable slide master"));
    }
    Ok(Masters { masters, layouts, owner })
}

/// Slides lisibles, dans l'ordre de la présentation.
///
/// Une slide dont le layout est illisible garde le master qui cite ce layout ; sans master connu
/// (layout absent ou cité par aucun master), elle prend le premier. Les deux cas sont signalés.
fn slides(
    shared: &Shared<'_>,
    parts: &[String],
    m: &Masters,
    themes: &[Rc<ThemeData<'_>>],
) -> Result<Vec<SourceSlide>, ImportError> {
    let by_part: BTreeMap<&str, usize> = m.layouts.iter().enumerate().map(|(k, (p, _, _))| (p.as_str(), k)).collect();
    let mut out = Vec::new();
    for (i, sp) in parts.iter().enumerate() {
        let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let lp = soft(shared.pkg.rels(sp), &shared.sink, sp)?
            .and_then(|r| r.by_type("/slideLayout").first().map(|(_, t)| (*t).to_owned()));
        let entry = lp.as_deref().and_then(|p| by_part.get(p)).and_then(|k| m.layouts.get(*k));
        let (layout, mi) = match entry {
            Some((_, mi, l)) => (Some(l), *mi),
            None => {
                shared.sink.borrow_mut().warn(
                    Some(n),
                    None,
                    WarningKind::UnreadablePart(clip(&format!("{sp}: layout"))),
                );
                (None, lp.as_deref().and_then(|p| m.owner.get(p)).copied().unwrap_or(0))
            }
        };
        let master = m.masters.get(mi);
        let (Some(master), Some(theme)) = (master, master.and_then(|m| themes.get(m.master.theme))) else {
            continue;
        };
        if let Some(s) = soft(parse_slide(shared, sp, n, layout, master, theme), &shared.sink, sp)? {
            out.push(s);
        }
        check(shared)?;
    }
    Ok(out)
}

/// Lit un `.pptx` et rend sa représentation intermédiaire.
///
/// Échoue seulement si le paquet est illisible, n'est pas une présentation,
/// est protégé, n'a aucun master utilisable, ou dépasse `limits`.
pub fn read_pptx(bytes: &[u8], limits: &ReadLimits) -> Result<SourceDeck, ImportError> {
    let pkg = Package::open(bytes, limits)?;
    let nodes = limits.max_xml_nodes;
    let main = pkg.main_part().to_owned();
    let pres_xml = pkg.text(&main)?;
    let pres = parse(&pres_xml, &main, nodes)?;
    let root = pres.root_element();
    let rels = pkg.rels(&main)?;

    // `tableStyles.xml` doit être analysé avant `Shared` (qui le garde) : ses avertissements attendent dans `early`.
    let early = RefCell::new(Sink::new());
    let ts_part = rels.by_type("/tableStyles").first().map(|(_, t)| (*t).to_owned());
    let ts_xml = match &ts_part {
        Some(p) => soft(pkg.text(p), &early, p)?,
        None => None,
    };
    let table_styles = match (&ts_xml, &ts_part) {
        (Some(x), Some(p)) => soft(parse(x, p, nodes), &early, p)?,
        _ => None,
    };
    let shared = Shared {
        sink: early,
        ..Shared::new(&pkg, table_styles, child(root, "defaultTextStyle").map_or_else(empty_levels, parse_lst_style))
    };

    let size = child(root, "sldSz").map_or(Size { w: 960.0, h: 540.0 }, |s| Size {
        w: emu(attr_i64(s, "cx").unwrap_or(12_192_000)).clamp(1.0, 1e6),
        h: emu(attr_i64(s, "cy").unwrap_or(6_858_000)).clamp(1.0, 1e6),
    });
    let master_parts = listed(root, "sldMasterIdLst", "sldMasterId", &rels);
    let theme_xml = theme_texts(&shared, &master_parts)?;
    let themes = themes(&shared, &theme_xml)?;
    let read_masters = masters(&shared, &master_parts, &themes)?;
    let slide_parts = listed(root, "sldIdLst", "sldId", &rels);
    if slide_parts.len() > limits.max_slides {
        return Err(too_large("slides"));
    }
    let slides = slides(&shared, &slide_parts, &read_masters, &themes)?;
    let Masters { masters, layouts, .. } = read_masters;

    let themes_out = themes.iter().map(|t| t.theme.clone()).collect();
    let masters_out: Vec<_> = masters.into_iter().map(|m| m.master).collect();
    let layouts_out: Vec<_> = layouts.into_iter().map(|(_, _, l)| l.layout).collect();
    let used = fonts::used(&masters_out, &layouts_out, &slides);
    let fonts = fonts::collect(root, &rels, &pkg, used, &shared.sink)?;
    let (assets, warnings) = shared.sink.into_inner().finish()?;
    Ok(SourceDeck {
        size,
        themes: themes_out,
        masters: masters_out,
        layouts: layouts_out,
        slides,
        assets,
        fonts,
        warnings,
    })
}
