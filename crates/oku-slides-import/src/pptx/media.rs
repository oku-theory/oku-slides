//! Type réel d'un média, lu dans ses premiers octets et jamais déduit de son extension.

/// Type d'un contenu non reconnu : les projecteurs le signalent comme image non prise en charge.
pub(crate) const OCTET_STREAM: &str = "application/octet-stream";
/// Type d'un SVG, reconnu à sa racine `<svg` (préfixée ou non) après un prologue XML facultatif.
pub(crate) const SVG: &str = "image/svg+xml";

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
pub(crate) fn sniff(bytes: &[u8]) -> &'static str {
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
