//! Placeholders : clés, tables par part, styles du master, héritage slide → layout → master.

use roxmltree::Node;

use crate::pptx::fill::{FillSpec, LineSpec};
use crate::pptx::text::{BodyProps, LevelProps};
use crate::pptx::xml::{attr_str, attr_u32};
use crate::source::{Frame, Geometry, PlaceholderLink, Role};

/// Placeholders gardés par part (au-delà, ignorés).
pub(crate) const MAX_PLACEHOLDERS: usize = 256;

/// Clé d'un placeholder : type brut (`obj` par défaut) et index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PhKey {
    pub ty: String,
    pub idx: Option<u32>,
}

impl PhKey {
    /// Lit un `p:ph`.
    pub(crate) fn from_ph(ph: Node<'_, '_>) -> PhKey {
        PhKey { ty: crate::collect::clip_to(attr_str(ph, "type").unwrap_or("obj"), 16), idx: attr_u32(ph, "idx") }
    }

    /// Rôle du placeholder.
    pub(crate) fn role(&self) -> Role {
        match self.ty.as_str() {
            "title" | "ctrTitle" => Role::Title,
            "subTitle" => Role::Subtitle,
            "body" => Role::Body,
            "pic" | "clipArt" => Role::Picture,
            "chart" => Role::Chart,
            "tbl" => Role::Table,
            "media" => Role::Media,
            "dt" => Role::Date,
            "ftr" => Role::Footer,
            "sldNum" => Role::SlideNum,
            _ => Role::Object,
        }
    }

    /// Type du placeholder de master correspondant : `title`, `dt`, `ftr`, `sldNum` ou `body`.
    pub(crate) fn master_type(&self) -> &str {
        match self.ty.as_str() {
            "title" | "ctrTitle" => "title",
            "dt" | "ftr" | "sldNum" => self.ty.as_str(),
            _ => "body",
        }
    }
}

/// Un placeholder déclaré dans un master ou un layout, avec ses propriétés propres (non résolues).
#[derive(Debug, Clone)]
pub(crate) struct PhInfo {
    pub id: String,
    pub key: PhKey,
    pub frame: Frame,
    pub lst: LevelProps,
    pub body: BodyProps,
    pub fill: Option<FillSpec>,
    pub line: LineSpec,
    pub geom: Option<Geometry>,
}

/// Placeholders d'une part.
#[derive(Debug, Clone, Default)]
pub(crate) struct PhTable {
    pub entries: Vec<PhInfo>,
}

impl PhTable {
    /// Placeholder de layout correspondant à une forme de slide : index, puis type exact, puis type de master.
    pub(crate) fn find(&self, key: &PhKey) -> Option<&PhInfo> {
        key.idx
            .and_then(|i| self.entries.iter().find(|e| e.key.idx == Some(i)))
            .or_else(|| self.entries.iter().find(|e| e.key.ty == key.ty))
            .or_else(|| self.entries.iter().find(|e| e.key.master_type() == key.master_type()))
    }

    /// Placeholder de master correspondant (par type de master).
    pub(crate) fn find_master(&self, key: &PhKey) -> Option<&PhInfo> {
        self.entries.iter().find(|e| e.key.master_type() == key.master_type())
    }
}

/// Styles texte du master (`txStyles`) et défauts de la présentation (`defaultTextStyle`).
#[derive(Debug, Clone)]
pub(crate) struct TextStyles {
    pub pres: LevelProps,
    pub title: LevelProps,
    pub body: LevelProps,
    pub other: LevelProps,
}

impl TextStyles {
    /// Style de `txStyles` qui s'applique à une forme.
    pub(crate) fn for_key(&self, key: Option<&PhKey>) -> &LevelProps {
        match key.map(|k| k.ty.as_str()) {
            Some("title" | "ctrTitle") => &self.title,
            Some("dt" | "ftr" | "sldNum") | None => &self.other,
            Some(_) => &self.body,
        }
    }
}

/// Ce qu'une forme hérite de ses placeholders parents (non résolu).
#[derive(Debug, Clone, Default)]
pub(crate) struct Inherited {
    /// Couches de styles texte, de la plus faible à la plus forte.
    pub levels: Vec<LevelProps>,
    pub bodies: Vec<BodyProps>,
    pub frame: Option<Frame>,
    pub fill: Option<FillSpec>,
    pub line: LineSpec,
    pub geom: Option<Geometry>,
    pub link: Option<PlaceholderLink>,
}

/// Héritage dans une part : styles du master, placeholders du master et du layout.
#[derive(Debug)]
pub(crate) struct PhInheritance<'a> {
    pub styles: &'a TextStyles,
    pub master: Option<&'a PhTable>,
    pub layout: Option<&'a PhTable>,
}

impl PhInheritance<'_> {
    /// Héritage d'une forme ; `None` = forme ordinaire.
    pub(crate) fn lookup(&self, ph: Option<&PhKey>) -> Inherited {
        let mut levels = vec![self.styles.pres.clone(), self.styles.for_key(ph).clone()];
        let Some(key) = ph else { return Inherited { levels, ..Inherited::default() } };
        let m = self.master.and_then(|t| t.find_master(key));
        let l = self.layout.and_then(|t| t.find(key));
        let mut bodies = Vec::new();
        for info in [m, l].into_iter().flatten() {
            levels.push(info.lst.clone());
            bodies.push(info.body.clone());
        }
        let base_line = m.map(|i| i.line.clone()).unwrap_or_default();
        Inherited {
            levels,
            bodies,
            frame: l.or(m).map(|i| i.frame),
            fill: l.and_then(|i| i.fill.clone()).or_else(|| m.and_then(|i| i.fill.clone())),
            line: l.map(|i| i.line.clone()).unwrap_or_default().over(&base_line),
            geom: l.and_then(|i| i.geom.clone()).or_else(|| m.and_then(|i| i.geom.clone())),
            link: Some(PlaceholderLink {
                idx: key.idx,
                layout: l.map(|i| i.id.clone()),
                master: m.map(|i| i.id.clone()),
            }),
        }
    }
}

#[cfg(test)]
#[path = "placeholder_tests.rs"]
mod placeholder_tests;
