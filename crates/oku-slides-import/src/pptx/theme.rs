//! Thème : emplacements de couleur, polices, styles de format, analysés une fois.

use std::collections::BTreeMap;
use std::rc::Rc;

use roxmltree::{Document, NodeId};

use crate::collect::clip_to;
use crate::error::{ImportError, structure};
use crate::pptx::color::{ColorCtx, ColorSpec, SLOTS, color_spec_in};
use crate::pptx::fill::{FillSpec, LineSpec, parse_fill_element, parse_line};
use crate::pptx::xml::{attr_str, child, descendant, elements, parse};
use crate::source::{Rgba, SourceTheme, ThemeFonts};

/// Longueur maximale d'un nom de police.
pub(crate) const MAX_FONT_NAME: usize = 128;

/// Thème Office (pour un master sans thème lisible).
const OFFICE: &str = r#"<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office"><a:themeElements><a:clrScheme name="Office"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="Office"><a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme><a:fmtScheme name="Office"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#;

/// Thème lu ; ses listes de styles sont indexées une fois pour résoudre `fillRef`, `lnRef`, `bgRef`.
#[derive(Debug)]
pub(crate) struct ThemeData<'x> {
    pub theme: SourceTheme,
    scheme: Rc<BTreeMap<String, Rgba>>,
    doc: Document<'x>,
    fills: Vec<NodeId>,
    bg_fills: Vec<NodeId>,
    lines: Vec<NodeId>,
}

fn list_ids(doc: &Document<'_>, list: &str) -> Vec<NodeId> {
    descendant(doc.root_element(), list).map(|l| elements(l).take(16).map(|n| n.id()).collect()).unwrap_or_default()
}

impl<'x> ThemeData<'x> {
    /// Lit `theme1.xml`.
    pub(crate) fn parse(xml: &'x str, part: &str, nodes: u32) -> Result<Self, ImportError> {
        let doc = parse(xml, part, nodes)?;
        let root = doc.root_element();
        let clr = descendant(root, "clrScheme").ok_or_else(|| structure(part))?;
        // Les emplacements se lisent sans clrMap.
        let bare = ColorCtx::new(BTreeMap::new(), BTreeMap::new());
        let colors: BTreeMap<String, Rgba> = SLOTS
            .iter()
            .filter_map(|s| Some(((*s).to_owned(), color_spec_in(child(clr, s)?)?.resolve(&bare)?.value)))
            .collect();
        let font = |which: &str| {
            let name = descendant(root, which)
                .and_then(|f| child(f, "latin"))
                .and_then(|l| attr_str(l, "typeface"))
                .unwrap_or("Calibri");
            clip_to(name, MAX_FONT_NAME)
        };
        let theme = SourceTheme {
            name: clip_to(attr_str(root, "name").unwrap_or_default(), MAX_FONT_NAME),
            colors: colors.clone(),
            fonts: ThemeFonts { major: font("majorFont"), minor: font("minorFont") },
        };
        let (fills, bg_fills, lines) =
            (list_ids(&doc, "fillStyleLst"), list_ids(&doc, "bgFillStyleLst"), list_ids(&doc, "lnStyleLst"));
        Ok(Self { theme, scheme: Rc::new(colors), doc, fills, bg_fills, lines })
    }

    /// Thème Office intégré.
    pub(crate) fn office() -> Result<ThemeData<'static>, ImportError> {
        ThemeData::parse(OFFICE, "office theme", 10_000)
    }

    /// Contexte de couleur de ce thème avec le `clrMap` donné.
    pub(crate) fn color_ctx(&self, map: BTreeMap<String, String>) -> ColorCtx {
        ColorCtx { scheme: Rc::clone(&self.scheme), map: Rc::new(map) }
    }

    /// Contexte avec le `clrMap` par défaut (tests).
    #[cfg(test)]
    pub(crate) fn default_ctx(&self) -> ColorCtx {
        self.color_ctx(crate::pptx::color::default_map())
    }

    /// Remplissage du style `idx` (1..3 formes, 1001.. fonds ; 0 et 1000 : aucun), `phClr` remplacé par `ph`.
    pub(crate) fn fill_style(
        &self,
        idx: u32,
        ph: Option<&ColorSpec>,
        blip: &mut dyn FnMut(&str) -> Option<String>,
    ) -> Option<FillSpec> {
        if idx == 0 || idx == 1000 {
            return Some(FillSpec::None);
        }
        let (list, i) = if idx > 1000 { (&self.bg_fills, idx - 1001) } else { (&self.fills, idx.checked_sub(1)?) };
        let node = self.doc.get_node(*list.get(usize::try_from(i).ok()?)?)?;
        Some(parse_fill_element(node, blip)?.with_placeholder(ph))
    }

    /// Trait du style `idx` (1..3), `phClr` remplacé par `ph`.
    pub(crate) fn line_style(&self, idx: u32, ph: Option<&ColorSpec>) -> LineSpec {
        if idx == 0 {
            return LineSpec { color: Some(None), ..LineSpec::default() };
        }
        idx.checked_sub(1)
            .and_then(|i| usize::try_from(i).ok())
            .and_then(|i| self.lines.get(i))
            .and_then(|id| self.doc.get_node(*id))
            .map(|ln| parse_line(ln).with_placeholder(ph))
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
