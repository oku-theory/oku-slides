//! Ce qu'une face sfnt dit d'elle-même : la famille sous laquelle Typst la
//! range, ses autres noms, son style, sa graisse, ses droits d'incorporation
//! et ses métriques verticales.

use ttf_parser::{Face, PlatformId, Style, Tag, name_id};

use super::LineMetrics;
use crate::source::FaceStyle;

/// Faces lues au plus dans une collection (`.ttc`).
const MAX_FACES: u32 = 256;

/// Une face lue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FaceInfo {
    /// Famille telle que Typst la nomme : le nom ID 1, suffixes de style ôtés.
    pub family: String,
    /// Noms sous lesquels la chercher, en minuscules : famille Typst, ID 1, ID 16.
    pub aliases: Vec<String>,
    /// Graisse et style.
    pub style: FaceStyle,
    /// Graisse (`usWeightClass`).
    pub weight: u16,
    /// Droits d'incorporation (`fsType` de la table `OS/2`, 0 sans elle).
    pub fs_type: u16,
    /// Hauteur des capitales (`sCapHeight` de la table `OS/2`) en unités de la face, si elle la donne.
    pub cap_height: Option<i16>,
    /// Ascendante en unités de la face : celle de `hhea`, ou de `OS/2` si la
    /// face le demande (`USE_TYPO_METRICS`), comme les navigateurs.
    pub ascender: i16,
    /// Descendante en unités de la face, négative sous la ligne de base.
    pub descender: i16,
    /// Unités par em.
    pub units_per_em: u16,
}

impl FaceInfo {
    /// Hauteur des capitales, ascendante et descendante, en em ; `None` si
    /// la face ne donne pas la hauteur des capitales ou des métriques
    /// absurdes.
    pub fn metrics(&self) -> Option<LineMetrics> {
        let upem = f64::from(self.units_per_em);
        let (cap, ascender) = (f64::from(self.cap_height?) / upem, f64::from(self.ascender) / upem);
        let descender = -f64::from(self.descender) / upem;
        let sane = upem > 0.0 && cap > 0.0 && ascender >= cap && descender >= 0.0;
        sane.then_some(LineMetrics { cap_height: cap, ascender, descender })
    }
}

/// Nombre de faces de `data` : n pour une collection (0 si son en-tête est
/// faux ou s'il en annonce plus de [`MAX_FACES`]), 1 sinon (lisible ou non).
pub(crate) fn count(data: &[u8]) -> u32 {
    if !data.starts_with(b"ttcf") {
        return 1;
    }
    // `ttf-parser` lit la table des décalages sans borne (assertion en debug) : l'en-tête est vérifié ici.
    let n = data.get(8..12).and_then(|b| b.try_into().ok()).map_or(0, u32::from_be_bytes);
    let fits = usize::try_from(n).ok().and_then(|n| n.checked_mul(4)?.checked_add(12)).is_some_and(|l| l <= data.len());
    if n <= MAX_FACES && fits { n } else { 0 }
}

/// Lit la face `index` de `data` ; `None` si elle ne se lit pas ou n'a pas de nom de famille.
pub(crate) fn read(data: &[u8], index: u32) -> Option<FaceInfo> {
    if index >= count(data) {
        return None;
    }
    let face = Face::parse(data, index).ok()?;
    let id1 = name(&face, name_id::FAMILY)?;
    let family = typographic_family(&id1).to_owned();
    let mut aliases = vec![family.to_lowercase(), id1.trim().to_lowercase()];
    aliases.extend(name(&face, name_id::TYPOGRAPHIC_FAMILY).map(|n| n.trim().to_lowercase()));
    aliases.retain(|a| !a.is_empty());
    aliases.sort();
    aliases.dedup();
    let full = name(&face, name_id::FULL_NAME).unwrap_or_default().to_lowercase();
    let weight = face.weight().to_number();
    // Comme Typst : le nom complet rattrape les faces italiques sans le bit.
    let italic = face.style() != Style::Normal || full.contains("italic") || full.contains("oblique");
    let bold = face.is_bold() || weight >= 600;
    let style = match (bold, italic) {
        (false, false) => FaceStyle::Regular,
        (true, false) => FaceStyle::Bold,
        (false, true) => FaceStyle::Italic,
        (true, true) => FaceStyle::BoldItalic,
    };
    let fs_type = face
        .raw_face()
        .table(Tag::from_bytes(b"OS/2"))
        .and_then(|os2| os2.get(8..10)?.try_into().ok())
        .map_or(0, u16::from_be_bytes);
    let (cap_height, units_per_em) = (face.capital_height(), face.units_per_em());
    let (ascender, descender) = (face.ascender(), face.descender());
    (!family.is_empty()).then_some(FaceInfo {
        family,
        aliases,
        style,
        weight,
        fs_type,
        cap_height,
        ascender,
        descender,
        units_per_em,
    })
}

/// Vrai si `fs_type` n'autorise pas l'incorporation (« Restricted License »
/// seul : un bit plus permissif l'emporte, comme le veut OpenType pour les
/// tables `OS/2` des versions 0 à 2).
pub(crate) fn restricted(fs_type: u16) -> bool {
    fs_type & 0x000E == 0x0002
}

/// Extension du fichier : `otf` (CFF), `ttc` (collection), `ttf` sinon.
pub(crate) fn extension(data: &[u8]) -> &'static str {
    match data.get(..4) {
        Some(b"OTTO") => "otf",
        Some(b"ttcf") => "ttc",
        _ => "ttf",
    }
}

/// Le premier nom `id` décodable (Unicode, ou ASCII en Mac Roman).
fn name(face: &Face<'_>, id: u16) -> Option<String> {
    face.names().into_iter().filter(|n| n.name_id == id).find_map(|n| {
        n.to_string().or_else(|| {
            (n.platform_id == PlatformId::Macintosh && n.encoding_id == 0 && n.name.is_ascii())
                .then(|| String::from_utf8_lossy(n.name).into_owned())
        })
    })
}

/// La famille sans ses suffixes de style (« Calibri Light » → « Calibri »).
///
/// Reprend `typographic_family` de Typst (`typst-library`, `text/font/info.rs`) :
/// le nom retenu doit être celui sous lequel le rendu trouve la face.
pub(crate) fn typographic_family(family: &str) -> &str {
    const SEPARATORS: [char; 3] = [' ', '-', '_'];
    const MODIFIERS: &[&str] = &["extra", "ext", "ex", "x", "semi", "sem", "sm", "demi", "dem", "ultra"];
    #[rustfmt::skip]
    const SUFFIXES: &[&str] = &[
        "normal", "italic", "oblique", "slanted",
        "thin", "th", "hairline", "light", "lt", "regular", "medium", "med",
        "md", "bold", "bd", "demi", "extb", "black", "blk", "bk", "heavy",
        "narrow", "condensed", "cond", "cn", "cd", "compressed", "expanded", "exp",
        "vf", "var", "variable",
    ];

    let family = family.trim().trim_start_matches('.');
    // Les suffixes sont ASCII : les longueurs en octets de `lower` valent celles de `family`.
    let lower = family.to_ascii_lowercase();
    let mut len = usize::MAX;
    let mut trimmed = lower.as_str();
    while trimmed.len() < len {
        len = trimmed.len();
        let mut t = trimmed;
        let mut shortened = false;
        while let Some(s) = SUFFIXES.iter().find_map(|s| t.strip_suffix(s)) {
            shortened = true;
            t = s;
        }
        if !shortened {
            break;
        }
        if let Some(s) = t.strip_suffix(SEPARATORS) {
            trimmed = s;
            t = s;
        }
        if let Some(t) = MODIFIERS.iter().find_map(|s| t.strip_suffix(s))
            && let Some(stripped) = t.strip_suffix(SEPARATORS)
        {
            trimmed = stripped;
        }
    }
    family.get(..len).unwrap_or(family)
}

#[cfg(test)]
#[path = "face_tests.rs"]
mod face_tests;
