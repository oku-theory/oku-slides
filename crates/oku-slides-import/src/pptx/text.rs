//! Texte tel qu'écrit dans une part : propriétés partielles, couleurs non résolues, valeurs bornées.

use roxmltree::Node;

use crate::collect::clip_to;
use crate::pptx::bounds::{
    BULLET_SIZE, CHAR_SPACING, FONT_SCALE, FONT_SIZE, INDENT, LINE_REDUCTION, MARGIN, SPACING_PERCENT, SPACING_POINTS,
    attr_in,
};
use crate::pptx::color::{ColorSpec, color_spec_in};
use crate::pptx::theme::MAX_FONT_NAME;
use crate::pptx::units::{emu, hundredths, pct};
use crate::pptx::xml::{attr_bool, attr_i64, attr_r, attr_str, attr_u32, child, elements};
use crate::source::{Align, Anchor, Autofit, Caps, Script, Spacing};

/// Niveaux de texte d'OOXML (`lvl1pPr`..`lvl9pPr`).
pub(crate) const LEVELS: usize = 9;
const MAX_COLUMNS: u32 = 16;
const MAX_BULLET_CHARS: usize = 8;
const MAX_SCHEME_CHARS: usize = 32;

/// Longueur maximale d'une étiquette de langue (`lang`).
const MAX_LANG: usize = 32;

/// Police d'un run : celle d'une écriture du thème (`+mj-ea`…) ou nommée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FontRef {
    Major(Script),
    Minor(Script),
    Named(String),
}

/// Propriétés de run partielles (`rPr`, `defRPr`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RunProps {
    /// Police latine (`latin`).
    pub font: Option<FontRef>,
    /// Police d'Asie de l'Est (`ea`).
    pub font_ea: Option<FontRef>,
    /// Police des écritures complexes (`cs`).
    pub font_cs: Option<FontRef>,
    /// Langue (`lang`), bornée.
    pub lang: Option<String>,
    pub size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strike: Option<bool>,
    pub color: Option<ColorSpec>,
    pub highlight: Option<ColorSpec>,
    pub baseline: Option<i32>,
    pub caps: Option<Caps>,
    pub char_spacing: Option<f64>,
    pub link: Option<String>,
}

/// `$self.$f` prend `$top.$f` quand il est présent.
macro_rules! overlay_fields {
    ($self:ident, $top:ident, $($f:ident),+) => {
        $( if $top.$f.is_some() { $self.$f.clone_from(&$top.$f); } )+
    };
}

impl RunProps {
    pub(crate) fn overlay(&mut self, top: &RunProps) {
        overlay_fields!(
            self,
            top,
            font,
            font_ea,
            font_cs,
            lang,
            size,
            bold,
            italic,
            underline,
            strike,
            color,
            highlight,
            baseline,
            caps,
            char_spacing,
            link
        );
    }
}

/// Sorte de puce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BulletKind {
    None,
    Char(String),
    AutoNum { scheme: String, start: u32 },
}

/// Propriétés de puce partielles ; chacune s'hérite séparément.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BulletProps {
    pub kind: Option<BulletKind>,
    /// `Some(None)` : la puce suit la couleur du texte (`buClrTx`).
    pub color: Option<Option<ColorSpec>>,
    pub size_pct: Option<f64>,
    /// `Some(None)` : la puce suit la police du texte (`buFontTx`).
    pub font: Option<Option<String>>,
}

impl BulletProps {
    pub(crate) fn overlay(&mut self, top: &BulletProps) {
        overlay_fields!(self, top, kind, color, size_pct, font);
    }
}

/// Propriétés de paragraphe partielles (`pPr`, `lvlNpPr`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ParaProps {
    pub align: Option<Align>,
    pub indent: Option<f64>,
    pub margin_left: Option<f64>,
    pub line_spacing: Option<Spacing>,
    pub space_before: Option<Spacing>,
    pub space_after: Option<Spacing>,
    pub bullet: BulletProps,
    pub run: RunProps,
    /// De droite à gauche (`rtl`).
    pub rtl: Option<bool>,
}

impl ParaProps {
    pub(crate) fn overlay(&mut self, top: &ParaProps) {
        overlay_fields!(self, top, align, indent, margin_left, line_spacing, space_before, space_after, rtl);
        self.bullet.overlay(&top.bullet);
        self.run.overlay(&top.run);
    }
}

/// Propriétés par niveau 0..8.
pub(crate) type LevelProps = Vec<ParaProps>;

/// Neuf niveaux vides.
pub(crate) fn empty_levels() -> LevelProps {
    vec![ParaProps::default(); LEVELS]
}

/// Propriétés de corps partielles (`bodyPr`).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BodyProps {
    /// l, t, r, b en pt.
    pub insets: [Option<f64>; 4],
    pub anchor: Option<Anchor>,
    pub autofit: Option<Autofit>,
    pub wrap: Option<bool>,
    pub columns: Option<u32>,
}

impl BodyProps {
    pub(crate) fn overlay(&mut self, top: &BodyProps) {
        for (mine, theirs) in self.insets.iter_mut().zip(top.insets) {
            if theirs.is_some() {
                *mine = theirs;
            }
        }
        overlay_fields!(self, top, anchor, autofit, wrap, columns);
    }
}

/// Run tel qu'écrit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawRun {
    pub text: String,
    pub field: Option<String>,
    pub props: RunProps,
}

/// Paragraphe tel qu'écrit.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawParagraph {
    pub level: u8,
    pub props: ParaProps,
    pub runs: Vec<RawRun>,
}

/// Corps de texte tel qu'écrit (`txBody`).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawText {
    pub body: BodyProps,
    pub lst: LevelProps,
    pub paragraphs: Vec<RawParagraph>,
}

/// Ancrage d'un attribut `anchor` (`t`, `ctr`, `b`).
pub(crate) fn anchor(v: &str) -> Anchor {
    match v {
        "ctr" => Anchor::Middle,
        "b" => Anchor::Bottom,
        _ => Anchor::Top,
    }
}

/// Polices de thème d'un `fontRef` (`idx="major"|"minor"`) : celles des trois
/// écritures ; rien pour un autre `idx`.
pub(crate) fn font_ref(r: Node<'_, '_>) -> RunProps {
    let scheme: fn(Script) -> FontRef = match attr_str(r, "idx") {
        Some("major") => FontRef::Major,
        Some("minor") => FontRef::Minor,
        _ => return RunProps::default(),
    };
    RunProps {
        font: Some(scheme(Script::Latin)),
        font_ea: Some(scheme(Script::EastAsian)),
        font_cs: Some(scheme(Script::Complex)),
        ..RunProps::default()
    }
}

fn spacing(n: Node<'_, '_>) -> Option<Spacing> {
    if let Some(p) = child(n, "spcPct") {
        return attr_in(p, "val", SPACING_PERCENT).map(|v| Spacing::Percent(pct(v)));
    }
    child(n, "spcPts").and_then(|p| attr_in(p, "val", SPACING_POINTS)).map(|v| Spacing::Points(hundredths(v)))
}

fn typeface(t: &str) -> FontRef {
    match t {
        "+mj-lt" => FontRef::Major(Script::Latin),
        "+mj-ea" => FontRef::Major(Script::EastAsian),
        "+mj-cs" => FontRef::Major(Script::Complex),
        "+mn-lt" => FontRef::Minor(Script::Latin),
        "+mn-ea" => FontRef::Minor(Script::EastAsian),
        "+mn-cs" => FontRef::Minor(Script::Complex),
        t => FontRef::Named(clip_to(t, MAX_FONT_NAME)),
    }
}

/// La police de l'enfant `tag` (`latin`, `ea`, `cs`) d'un `rPr`.
fn script_font(rpr: Node<'_, '_>, tag: &str) -> Option<FontRef> {
    child(rpr, tag).and_then(|l| attr_str(l, "typeface")).map(typeface)
}

/// Lit un `rPr`/`defRPr`/`endParaRPr`.
pub(crate) fn parse_rpr(rpr: Node<'_, '_>, link: &mut dyn FnMut(&str) -> Option<String>) -> RunProps {
    RunProps {
        font: script_font(rpr, "latin"),
        font_ea: script_font(rpr, "ea"),
        font_cs: script_font(rpr, "cs"),
        lang: attr_str(rpr, "lang").map(|l| clip_to(l, MAX_LANG)),
        size: attr_in(rpr, "sz", FONT_SIZE).map(hundredths),
        bold: attr_bool(rpr, "b"),
        italic: attr_bool(rpr, "i"),
        underline: attr_str(rpr, "u").map(|u| u != "none"),
        strike: attr_str(rpr, "strike").map(|s| s != "noStrike"),
        color: child(rpr, "solidFill").and_then(color_spec_in),
        highlight: child(rpr, "highlight").and_then(color_spec_in),
        baseline: attr_i64(rpr, "baseline").map(|b| i32::try_from(b / 1000).unwrap_or(0)),
        caps: attr_str(rpr, "cap").map(|c| match c {
            "all" => Caps::All,
            "small" => Caps::Small,
            _ => Caps::None,
        }),
        char_spacing: attr_in(rpr, "spc", CHAR_SPACING).map(hundredths),
        link: child(rpr, "hlinkClick").and_then(|h| attr_r(h, "id")).and_then(link),
    }
}

/// Lit un `pPr`/`lvlNpPr`/`defPPr`.
pub(crate) fn parse_ppr(ppr: Node<'_, '_>) -> ParaProps {
    let mut bullet = BulletProps::default();
    for b in elements(ppr) {
        match b.tag_name().name() {
            "buNone" => bullet.kind = Some(BulletKind::None),
            "buChar" => bullet.kind = attr_str(b, "char").map(|c| BulletKind::Char(clip_to(c, MAX_BULLET_CHARS))),
            "buAutoNum" => {
                bullet.kind = Some(BulletKind::AutoNum {
                    scheme: clip_to(attr_str(b, "type").unwrap_or("arabicPeriod"), MAX_SCHEME_CHARS),
                    start: attr_u32(b, "startAt").unwrap_or(1).max(1),
                });
            }
            "buClrTx" => bullet.color = Some(None),
            "buClr" => bullet.color = color_spec_in(b).map(Some),
            "buSzPct" => bullet.size_pct = attr_in(b, "val", BULLET_SIZE).map(pct),
            "buFontTx" => bullet.font = Some(None),
            "buFont" => bullet.font = attr_str(b, "typeface").map(|t| Some(clip_to(t, MAX_FONT_NAME))),
            _ => {}
        }
    }
    ParaProps {
        align: attr_str(ppr, "algn").map(|a| match a {
            "ctr" => Align::Center,
            "r" => Align::Right,
            "just" | "dist" => Align::Justify,
            _ => Align::Left,
        }),
        indent: attr_in(ppr, "indent", INDENT).map(emu),
        margin_left: attr_in(ppr, "marL", MARGIN).map(emu),
        line_spacing: child(ppr, "lnSpc").and_then(spacing),
        space_before: child(ppr, "spcBef").and_then(spacing),
        space_after: child(ppr, "spcAft").and_then(spacing),
        bullet,
        run: child(ppr, "defRPr").map(|r| parse_rpr(r, &mut |_| None)).unwrap_or_default(),
        rtl: attr_bool(ppr, "rtl"),
    }
}

/// Lit un `lstStyle` (ou `titleStyle`/`bodyStyle`/`otherStyle`, `defaultTextStyle`).
pub(crate) fn parse_lst_style(lst: Node<'_, '_>) -> LevelProps {
    let def = child(lst, "defPPr").map(parse_ppr).unwrap_or_default();
    (1..=LEVELS)
        .map(|i| {
            let mut p = def.clone();
            if let Some(l) = child(lst, &format!("lvl{i}pPr")) {
                p.overlay(&parse_ppr(l));
            }
            p
        })
        .collect()
}

/// Lit un `bodyPr`.
pub(crate) fn parse_body_pr(bp: Node<'_, '_>) -> BodyProps {
    let autofit = elements(bp).find_map(|c| match c.tag_name().name() {
        "noAutofit" => Some(Autofit::None),
        "spAutoFit" => Some(Autofit::Resize),
        "normAutofit" => Some(Autofit::Shrink {
            font_scale: attr_in(c, "fontScale", FONT_SCALE).map_or(1.0, pct),
            line_reduction: attr_in(c, "lnSpcReduction", LINE_REDUCTION).map_or(0.0, pct),
        }),
        _ => None,
    });
    let columns = attr_str(bp, "numCol").map(|_| match attr_i64(bp, "numCol") {
        Some(n) if n >= 1 => u32::try_from(n).unwrap_or(MAX_COLUMNS).min(MAX_COLUMNS),
        _ => 1,
    });
    BodyProps {
        insets: ["lIns", "tIns", "rIns", "bIns"].map(|a| attr_i64(bp, a).map(emu)),
        anchor: attr_str(bp, "anchor").map(anchor),
        autofit,
        wrap: attr_str(bp, "wrap").map(|w| w != "none"),
        columns,
    }
}

/// Lit un `txBody` (`p:txBody`, `a:txBody` de cellule, `dsp:txBody`).
pub(crate) fn parse_txbody(tx: Node<'_, '_>, link: &mut dyn FnMut(&str) -> Option<String>) -> RawText {
    let mut paragraphs = Vec::new();
    for p in elements(tx).filter(|n| n.tag_name().name() == "p") {
        let ppr = child(p, "pPr");
        let mut runs = Vec::new();
        for r in elements(p) {
            let cached = || child(r, "t").and_then(|t| t.text()).unwrap_or_default().to_owned();
            let (text, field) = match r.tag_name().name() {
                "r" => (cached(), None),
                "fld" => (cached(), attr_str(r, "type").map(|t| clip_to(t, MAX_SCHEME_CHARS))),
                "br" => ("\n".to_owned(), None),
                _ => continue,
            };
            let props = child(r, "rPr").map(|rp| parse_rpr(rp, link)).unwrap_or_default();
            runs.push(RawRun { text, field, props });
        }
        paragraphs.push(RawParagraph {
            level: ppr.and_then(|n| attr_u32(n, "lvl")).map_or(0, |l| u8::try_from(l.min(8)).unwrap_or(8)),
            props: ppr.map(parse_ppr).unwrap_or_default(),
            runs,
        });
    }
    RawText {
        body: child(tx, "bodyPr").map(parse_body_pr).unwrap_or_default(),
        lst: child(tx, "lstStyle").map(parse_lst_style).unwrap_or_else(empty_levels),
        paragraphs,
    }
}

#[cfg(test)]
#[path = "text_tests.rs"]
mod text_tests;
