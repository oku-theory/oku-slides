//! XML : analyse bornée (nœuds, profondeur), et accès par nom local (`p:sp`, `dsp:sp`, `sp` se lisent pareil).

use roxmltree::{Document, Node, ParsingOptions};

use crate::error::{ImportError, structure, xml_error};

const NS_REL: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "http://purl.oclc.org/ooxml/officeDocument/relationships",
];

/// Profondeur maximale d'un document : roxmltree analyse récursivement.
pub(crate) const MAX_XML_DEPTH: usize = 256;

/// Profondeur d'éléments d'un texte XML, par un balayage linéaire (commentaires, CDATA,
/// instructions et déclarations ignorés ; `>` entre guillemets ignoré).
pub(crate) fn max_depth(text: &str) -> usize {
    let b = text.as_bytes();
    let (mut i, mut depth, mut max) = (0usize, 0usize, 0usize);
    let skip_to = |from: usize, end: &[u8]| -> usize {
        b.get(from..)
            .and_then(|rest| rest.windows(end.len()).position(|w| w == end))
            .map_or(b.len(), |p| from + p + end.len())
    };
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &b[i..];
        if rest.starts_with(b"<!--") {
            i = skip_to(i + 4, b"-->");
        } else if rest.starts_with(b"<![CDATA[") {
            i = skip_to(i + 9, b"]]>");
        } else if rest.starts_with(b"<?") {
            i = skip_to(i + 2, b"?>");
        } else if rest.starts_with(b"<!") {
            i = skip_to(i + 2, b">");
        } else if rest.starts_with(b"</") {
            depth = depth.saturating_sub(1);
            i = skip_to(i + 2, b">");
        } else {
            let (mut j, mut quote) = (i + 1, None::<u8>);
            while j < b.len() {
                match (quote, b[j]) {
                    (None, q @ (b'"' | b'\'')) => quote = Some(q),
                    (Some(q), c) if c == q => quote = None,
                    (None, b'>') => break,
                    _ => {}
                }
                j += 1;
            }
            if j > i + 1 && b.get(j - 1) != Some(&b'/') {
                depth += 1;
                max = max.max(depth);
            }
            i = j + 1;
        }
    }
    max
}

/// Analyse une part XML : sans DTD, au plus `nodes` nœuds et `MAX_XML_DEPTH` niveaux.
pub(crate) fn parse<'i>(text: &'i str, part: &str, nodes: u32) -> Result<Document<'i>, ImportError> {
    if max_depth(text) > MAX_XML_DEPTH {
        return Err(structure(&format!("{part}: XML nested deeper than {MAX_XML_DEPTH}")));
    }
    let opt = ParsingOptions { allow_dtd: false, nodes_limit: nodes, ..ParsingOptions::default() };
    Document::parse_with_options(text, opt).map_err(|e| xml_error(part, e))
}

/// Enfants éléments.
pub(crate) fn elements<'a, 'i>(n: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(Node::is_element)
}

/// Premier enfant élément de nom local `local`.
pub(crate) fn child<'a, 'i>(n: Node<'a, 'i>, local: &str) -> Option<Node<'a, 'i>> {
    elements(n).find(|c| c.tag_name().name() == local)
}

/// Enfants éléments de nom local `local`.
pub(crate) fn children<'a, 'i>(n: Node<'a, 'i>, local: &str) -> impl Iterator<Item = Node<'a, 'i>> {
    elements(n).filter(move |c| c.tag_name().name() == local)
}

/// Premier descendant (lui-même exclu) de nom local `local`.
pub(crate) fn descendant<'a, 'i>(n: Node<'a, 'i>, local: &str) -> Option<Node<'a, 'i>> {
    n.descendants().skip(1).find(|c| c.is_element() && c.tag_name().name() == local)
}

/// Attribut sans espace de noms.
pub(crate) fn attr_str<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.attributes().find(|a| a.namespace().is_none() && a.name() == name).map(|a| a.value())
}

/// Attribut de l'espace des relations (`r:embed`, `r:id`…), par nom local.
pub(crate) fn attr_r<'a>(n: Node<'a, '_>, local: &str) -> Option<&'a str> {
    n.attributes()
        .find(|a| a.name() == local && a.namespace().is_some_and(|ns| NS_REL.contains(&ns)))
        .map(|a| a.value())
}

/// Entier signé.
pub(crate) fn attr_i64(n: Node<'_, '_>, name: &str) -> Option<i64> {
    attr_str(n, name)?.trim().parse().ok()
}

/// Entier non signé 32 bits.
pub(crate) fn attr_u32(n: Node<'_, '_>, name: &str) -> Option<u32> {
    attr_str(n, name)?.trim().parse().ok()
}

/// Nombre fini (`NaN`, `inf` rejetés).
pub(crate) fn attr_f64(n: Node<'_, '_>, name: &str) -> Option<f64> {
    attr_str(n, name)?.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Booléen OOXML (`1`/`true`/`on`, `0`/`false`/`off`).
pub(crate) fn attr_bool(n: Node<'_, '_>, name: &str) -> Option<bool> {
    match attr_str(n, name)? {
        "1" | "true" | "on" => Some(true),
        "0" | "false" | "off" => Some(false),
        _ => None,
    }
}

/// Texte concaténé des descendants `t` (titres de graphique).
pub(crate) fn text_of(n: Node<'_, '_>) -> String {
    n.descendants().filter(|d| d.is_element() && d.tag_name().name() == "t").filter_map(|d| d.text()).collect()
}

#[cfg(test)]
#[path = "xml_tests.rs"]
mod xml_tests;
