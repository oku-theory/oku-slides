//! Polices : familles utilisées par le texte, faces embarquées lisibles et leurs droits.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use roxmltree::Node;

use crate::ImportErrorKind;
use crate::collect::{Sink, clip};
use crate::error::ImportError;
use crate::pptx::eot;
use crate::pptx::package::{Package, Rels};
use crate::pptx::theme::MAX_FONT_NAME;
use crate::pptx::xml::{attr_bool, attr_r, attr_str, child, children};
use crate::source::{
    Bullet, FaceStyle, FontFace, Item, ItemKind, Layout, Master, ParaStyle, Paragraph, Placeholder, SourceFont,
    SourceSlide, TextBox, WarningKind,
};

const FACES: [(&str, FaceStyle); 4] = [
    ("regular", FaceStyle::Regular),
    ("bold", FaceStyle::Bold),
    ("italic", FaceStyle::Italic),
    ("boldItalic", FaceStyle::BoldItalic),
];

fn bullet_font(p: &ParaStyle, out: &mut BTreeSet<String>) {
    if let Bullet::Char { font: Some(f), .. } = &p.bullet {
        out.insert(f.clone());
    }
}

fn paragraphs(ps: &[Paragraph], out: &mut BTreeSet<String>) {
    for p in ps {
        out.extend(p.runs.iter().map(|r| r.style.font.value.clone()));
        bullet_font(&p.style, out);
    }
}

fn textbox(tb: &TextBox, out: &mut BTreeSet<String>) {
    paragraphs(&tb.paragraphs, out);
}

fn items(list: &[Item], out: &mut BTreeSet<String>) {
    for it in list {
        match &it.kind {
            ItemKind::Text(tb) => textbox(tb, out),
            ItemKind::Shape(s) => s.text.iter().for_each(|tb| textbox(tb, out)),
            ItemKind::Table(t) => t.rows.iter().flatten().for_each(|c| textbox(&c.text, out)),
            ItemKind::Group { items: kids } => items(kids, out),
            ItemKind::Picture(_) | ItemKind::Chart(_) | ItemKind::Opaque(_) => {}
        }
    }
}

fn placeholders(phs: &[Placeholder], out: &mut BTreeSet<String>) {
    for p in phs {
        for l in &p.levels {
            out.insert(l.run.font.value.clone());
            bullet_font(&l.para, out);
        }
        textbox(&p.text, out);
    }
}

/// Familles utilisées par tout le texte du deck.
pub(crate) fn used(masters: &[Master], layouts: &[Layout], slides: &[SourceSlide]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for m in masters {
        items(&m.decor, &mut out);
        placeholders(&m.placeholders, &mut out);
    }
    for l in layouts {
        items(&l.decor, &mut out);
        placeholders(&l.placeholders, &mut out);
    }
    for s in slides {
        items(&s.items, &mut out);
    }
    out.retain(|f| !f.is_empty());
    out
}

/// Vrai pour une police sfnt (TrueType, OpenType, collection).
pub(crate) fn is_sfnt(data: &[u8]) -> bool {
    matches!(data.get(..4), Some(b"\x00\x01\x00\x00" | b"OTTO" | b"true" | b"ttcf"))
}

/// Droits d'incorporation (`fsType` de la table `OS/2`) d'une police sfnt.
pub(crate) fn fs_type(sfnt: &[u8]) -> Option<u16> {
    let be16 = |o: usize| sfnt.get(o..o.checked_add(2)?).and_then(|b| b.try_into().ok()).map(u16::from_be_bytes);
    let be32 = |o: usize| sfnt.get(o..o.checked_add(4)?).and_then(|b| b.try_into().ok()).map(u32::from_be_bytes);
    let tables = usize::from(be16(4)?);
    (0..tables.min(256)).find_map(|i| {
        let rec = 12 + 16 * i;
        (sfnt.get(rec..rec + 4)? == b"OS/2")
            .then(|| be16(usize::try_from(be32(rec + 8)?).ok()?.checked_add(8)?))
            .flatten()
    })
}

/// Obfuscation ODTTF : XOR des 32 premiers octets avec les 16 octets du GUID du nom de part, dans l'ordre inverse.
pub(crate) fn deobfuscate(data: &mut [u8], part: &str) -> bool {
    let name = part.rsplit('/').next().unwrap_or(part);
    let digits: Vec<u8> = name
        .split('.')
        .next()
        .unwrap_or_default()
        .chars()
        .filter_map(|c| c.to_digit(16))
        .filter_map(|d| u8::try_from(d).ok())
        .collect();
    let Ok(digits32) = <[u8; 32]>::try_from(digits.as_slice()) else { return false };
    let key: Vec<u8> = digits32.as_chunks::<2>().0.iter().rev().map(|[hi, lo]| (hi << 4) | lo).collect();
    for (b, k) in data.iter_mut().take(32).zip(key.iter().cycle()) {
        *b ^= k;
    }
    true
}

fn readable(mut data: Vec<u8>, part: &str) -> Option<Vec<u8>> {
    if is_sfnt(&data) {
        return Some(data);
    }
    if let Some(font) = eot::decode(&data).filter(|f| is_sfnt(f)) {
        return Some(font);
    }
    (deobfuscate(&mut data, part) && is_sfnt(&data)).then_some(data)
}

/// Familles utilisées et faces embarquées, triées par famille.
pub(crate) fn collect(
    pres: Node<'_, '_>,
    rels: &Rels,
    pkg: &Package<'_>,
    used: BTreeSet<String>,
    sink: &RefCell<Sink>,
) -> Result<Vec<SourceFont>, ImportError> {
    let subset = attr_bool(pres, "saveSubsetFonts").unwrap_or(false);
    let mut faces: BTreeMap<String, Vec<FontFace>> = used.into_iter().map(|f| (f, Vec::new())).collect();
    let mut read: BTreeMap<String, Option<Vec<u8>>> = BTreeMap::new();
    // Une part partagée par plusieurs faces est copiée dans chacune : les copies comptent dans le budget total.
    let mut copied: u64 = 0;
    for ef in child(pres, "embeddedFontLst").into_iter().flat_map(|l| children(l, "embeddedFont")).take(64) {
        let Some(family) =
            child(ef, "font").and_then(|f| attr_str(f, "typeface")).map(|t| crate::collect::clip_to(t, MAX_FONT_NAME))
        else {
            continue;
        };
        for (tag, style) in FACES {
            let Some(part) = child(ef, tag).and_then(|n| attr_r(n, "id")).and_then(|r| rels.target(r)) else {
                continue;
            };
            if !read.contains_key(part) {
                let decoded = match pkg.bytes(part) {
                    Ok(d) => readable(d, part),
                    Err(e) if e.kind() == ImportErrorKind::TooLarge => return Err(e),
                    Err(_) => {
                        sink.borrow_mut().warn(None, None, WarningKind::MissingPart(clip(part)));
                        read.insert(part.to_owned(), None);
                        continue;
                    }
                };
                if decoded.is_none() {
                    sink.borrow_mut().warn(None, None, WarningKind::FontEmbeddedUnreadable(family.clone()));
                }
                read.insert(part.to_owned(), decoded);
            }
            if let Some(Some(data)) = read.get(part) {
                copied = copied.saturating_add(u64::try_from(data.len()).unwrap_or(u64::MAX));
                if copied > pkg.limits().max_total_bytes {
                    return Err(crate::error::too_large("fonts"));
                }
                faces.entry(family.clone()).or_default().push(FontFace {
                    style,
                    fs_type: fs_type(data),
                    subset,
                    data: data.clone(),
                });
            }
        }
        faces.entry(family).or_default();
    }
    Ok(faces.into_iter().map(|(family, faces)| SourceFont { family, faces }).collect())
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod fonts_tests;
