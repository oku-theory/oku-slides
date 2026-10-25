//! Type réel d'un média, lu dans ses premiers octets et jamais déduit de son extension.
//!
//! L'import d'un `.pptx` et le collage d'une image dans le brouillon de l'agent s'en servent.

/// Type d'un contenu non reconnu : les projecteurs le signalent comme image non prise en charge.
pub const OCTET_STREAM: &str = "application/octet-stream";
/// Type d'un SVG, reconnu à sa racine `<svg` (préfixée ou non) après un prologue XML facultatif.
pub const SVG: &str = "image/svg+xml";

/// Signatures : décalage, octets attendus, type.
const MAGIC: [(usize, &[u8], &str); 13] = [
    (0, b"\x89PNG\r\n\x1a\n", "image/png"),
    (0, b"\xFF\xD8\xFF", "image/jpeg"),
    (0, b"GIF87a", "image/gif"),
    (0, b"GIF89a", "image/gif"),
    (8, b"WEBP", "image/webp"),
    (0, b"BM", "image/bmp"),
    (0, b"II*\0", "image/tiff"),
    (0, b"MM\0*", "image/tiff"),
    (0, b"II\xBC", "image/vnd.ms-photo"),
    (40, b" EMF", "image/emf"),
    (0, b"\xD7\xCD\xC6\x9A", "image/wmf"),
    (0, b"\x01\0\x09\0", "image/wmf"),
    (0, b"\x02\0\x09\0", "image/wmf"),
];

/// Type MIME de `bytes` d'après leur contenu ; seule source du type d'un asset importé.
#[must_use]
pub fn sniff(bytes: &[u8]) -> &'static str {
    let at = |off: usize, sig: &[u8]| bytes.get(off..off + sig.len()) == Some(sig);
    let found = MAGIC.iter().find(|(off, sig, mime)| {
        at(*off, sig)
            && match *mime {
                "image/webp" => at(0, b"RIFF"),
                "image/emf" => at(0, b"\x01\0\0\0"),
                _ => true,
            }
    });
    match found {
        Some((_, _, mime)) => mime,
        None if is_svg(bytes) => SVG,
        None => OCTET_STREAM,
    }
}

/// Largeur et hauteur annoncées par l'en-tête d'un raster que Typst dessine (png, jpeg, gif, webp).
///
/// Lues sans rien décoder ni allouer, pour refuser une image géante avant que le rendu ne la
/// décompresse ; `None` si l'en-tête est tronqué ou incohérent, ou si `mime` n'est pas l'un d'eux.
#[must_use]
pub fn dimensions(bytes: &[u8], mime: &str) -> Option<(u32, u32)> {
    let le16 = |at: usize| bytes.get(at..at + 2).map(|b| u32::from(u16::from_le_bytes([b[0], b[1]])));
    let le24 = |at: usize| bytes.get(at..at + 3).map(|b| u32::from_le_bytes([b[0], b[1], b[2], 0]));
    let be32 = |at: usize| bytes.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    match mime {
        "image/png" => (bytes.get(12..16) == Some(b"IHDR")).then_some((be32(16)?, be32(20)?)),
        "image/gif" => Some((le16(6)?, le16(8)?)),
        "image/webp" => match bytes.get(12..16)? {
            b"VP8 " => (bytes.get(23..26) == Some(b"\x9d\x01\x2a")).then_some((le16(26)? & 0x3FFF, le16(28)? & 0x3FFF)),
            b"VP8L" => {
                let bits = u32::from_le_bytes(bytes.get(21..25)?.try_into().ok()?);
                (bytes.get(20) == Some(&0x2F)).then_some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
            }
            b"VP8X" => Some((le24(24)? + 1, le24(27)? + 1)),
            _ => None,
        },
        "image/jpeg" => jpeg_dimensions(bytes),
        _ => None,
    }
}

/// Dimensions du premier segment `SOFn` d'un JPEG, segments précédents sautés par leur longueur.
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let be16 = |at: usize| bytes.get(at..at + 2).map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])));
    let mut at = 2;
    loop {
        // Octets de remplissage `FF` admis avant un marqueur.
        while bytes.get(at + 1) == Some(&0xFF) && bytes.get(at) == Some(&0xFF) {
            at += 1;
        }
        if bytes.get(at) != Some(&0xFF) {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        match marker {
            // Marqueurs sans segment : TEM, RSTn.
            0x01 | 0xD0..=0xD7 => at += 2,
            // SOF0 à SOF15, sauf DHT (C4), JPG (C8) et DAC (CC).
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                let (h, w) = (be16(at + 5)?, be16(at + 7)?);
                return Some((u32::try_from(w).ok()?, u32::try_from(h).ok()?));
            }
            // Début des données ou fin d'image avant tout SOF.
            0xD9 | 0xDA => return None,
            _ => at = at.checked_add(2)?.checked_add(be16(at + 2)?)?,
        }
    }
}

/// Reste de `b` après la première occurrence de `end`.
fn after<'b>(b: &'b [u8], end: &[u8]) -> Option<&'b [u8]> {
    b.windows(end.len()).position(|w| w == end).and_then(|p| b.get(p + end.len()..))
}

/// Reste après un DOCTYPE (sous-ensemble interne `[…]` compris).
fn after_doctype(b: &[u8]) -> Option<&[u8]> {
    let close = b.iter().position(|c| *c == b'>')?;
    // `[` cherché avant `>` seulement : chaque DOCTYPE ne lit que ce qu'il consomme (lecture linéaire).
    match b.get(..close)?.iter().position(|c| *c == b'[') {
        Some(open) => after(b.get(open..)?, b"]").and_then(|r| after(r, b">")),
        None => b.get(close + 1..),
    }
}

/// Nom XML sans préfixe : lettre ou `_`, puis lettres, chiffres, `_`, `-`, `.`.
fn is_ncname(n: &[u8]) -> bool {
    n.first().is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_')
        && n.iter().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.'))
}

/// Racine `<svg` (ou `<préfixe:svg`) après BOM, déclaration, commentaires, instructions et DOCTYPE facultatifs.
fn is_svg(bytes: &[u8]) -> bool {
    let mut rest = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    loop {
        rest = rest.trim_ascii_start();
        let next = if let Some(r) = rest.strip_prefix(b"<?") {
            after(r, b"?>")
        } else if let Some(r) = rest.strip_prefix(b"<!--") {
            after(r, b"-->")
        } else if let Some(r) = rest.strip_prefix(b"<!DOCTYPE") {
            after_doctype(r)
        } else {
            break;
        };
        match next {
            Some(r) => rest = r,
            None => return false,
        }
    }
    let Some(tag) = rest.strip_prefix(b"<") else { return false };
    let end = tag.iter().position(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/')).unwrap_or(tag.len());
    let (name, delimited) = tag.split_at(end);
    // Un préfixe est admis : `sanitize` vérifie ensuite l'espace de noms SVG par son URI.
    !delimited.is_empty() && (name == b"svg" || name.strip_suffix(b":svg").is_some_and(is_ncname))
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod media_tests;
