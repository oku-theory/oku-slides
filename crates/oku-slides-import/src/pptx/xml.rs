//! XML : analyse bornée (nœuds, profondeur), et accès par nom local (`p:sp`, `dsp:sp`, `sp` se lisent pareil).

use roxmltree::{Document, Node, ParsingOptions};

use crate::error::{ImportError, structure, xml_error};

const NS_REL: [&str; 2] = [
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "http://purl.oclc.org/ooxml/officeDocument/relationships",
];

/// Profondeur maximale d'un document : roxmltree analyse récursivement.
pub(crate) const MAX_XML_DEPTH: usize = 256;

/// Nombre maximal d'attributs d'un élément : roxmltree cherche les doublons en O(n²).
///
/// Un élément OOXML ou SVG réel en porte quelques dizaines ; à 50 000, l'analyse prenait
/// des secondes (t-754).
pub(crate) const MAX_XML_ATTRIBUTES: usize = 1024;

/// Forme d'un texte XML mesurée avant analyse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct XmlShape {
    /// Profondeur d'éléments maximale.
    pub depth: usize,
    /// Plus grand nombre d'attributs (et de déclarations d'espace de noms) d'un élément.
    pub attributes: usize,
}

/// Forme d'un texte XML, par un balayage linéaire (commentaires, CDATA, instructions et
/// déclarations ignorés ; `>` et `=` entre guillemets ignorés). Un attribut compte pour un `=`
/// hors guillemets dans une balise ouvrante.
pub(crate) fn shape(text: &str) -> XmlShape {
    let b = text.as_bytes();
    let (mut i, mut depth, mut max) = (0usize, 0usize, 0usize);
    let mut attributes = 0usize;
    let skip_to = |from: usize, end: &[u8]| -> usize {
        b.get(from..)
            .and_then(|rest| rest.windows(end.len()).position(|window| window == end))
            .map_or(b.len(), |pos| from + pos + end.len())
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
            let (mut j, mut quote, mut equals) = (i + 1, None::<u8>, 0usize);
            while j < b.len() {
                match (quote, b[j]) {
                    (None, open @ (b'"' | b'\'')) => quote = Some(open),
                    (Some(open), byte) if byte == open => quote = None,
                    (None, b'>') => break,
                    (None, b'=') => equals += 1,
                    _ => {}
                }
                j += 1;
            }
            attributes = attributes.max(equals);
            if j > i + 1 && b.get(j - 1) != Some(&b'/') {
                depth += 1;
                max = max.max(depth);
            }
            i = j + 1;
        }
    }
    XmlShape { depth: max, attributes }
}

/// Analyse une part XML : sans DTD, au plus `nodes` nœuds, `MAX_XML_DEPTH` niveaux et
/// `MAX_XML_ATTRIBUTES` attributs par élément.
pub(crate) fn parse<'i>(text: &'i str, part: &str, nodes: u32) -> Result<Document<'i>, ImportError> {
    parse_opts(text, part, nodes, false)
}

/// Comme `parse`, en tolérant un DOCTYPE : réservé aux SVG, dont toute déclaration est refusée avant.
pub(crate) fn parse_doctype<'i>(text: &'i str, part: &str, nodes: u32) -> Result<Document<'i>, ImportError> {
    parse_opts(text, part, nodes, true)
}

fn parse_opts<'i>(text: &'i str, part: &str, nodes: u32, allow_dtd: bool) -> Result<Document<'i>, ImportError> {
    let shape = shape(text);
    if shape.depth > MAX_XML_DEPTH {
        return Err(structure(&format!("{part}: XML nested deeper than {MAX_XML_DEPTH}")));
    }
    if shape.attributes > MAX_XML_ATTRIBUTES {
        return Err(structure(&format!("{part}: XML element with more than {MAX_XML_ATTRIBUTES} attributes")));
    }
    let opt = ParsingOptions { allow_dtd, nodes_limit: nodes, ..ParsingOptions::default() };
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

fn is(n: Node<'_, '_>, local: &str) -> bool {
    n.is_element() && n.tag_name().name() == local
}

/// Texte des descendants `t` de `n`, au plus `max_chars` caractères.
fn runs_of(n: Node<'_, '_>, max_chars: usize) -> String {
    n.descendants().filter(|d| is(*d, "t")).filter_map(|d| d.text()).flat_map(str::chars).take(max_chars).collect()
}

/// Paragraphes `p` de `n` les plus hauts : un `p` imbriqué dans un autre n'est pas revisité.
fn outer_paragraphs<'a, 'i>(n: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    // Parcours en profondeur, dans l'ordre du document, qui ne descend pas sous un `p`.
    let mut stack: Vec<Node<'a, 'i>> = n.children().rev().filter(Node::is_element).collect();
    std::iter::from_fn(move || {
        while let Some(c) = stack.pop() {
            if is(c, "p") {
                return Some(c);
            }
            stack.extend(c.children().rev().filter(Node::is_element));
        }
        None
    })
}

/// Texte des descendants `t`, un paragraphe `p` par ligne (titres de graphique) : au plus
/// `max_lines` paragraphes de `max_chars` caractères ; chaque `t` n'est lu qu'une fois.
pub(crate) fn text_of(n: Node<'_, '_>, max_lines: usize, max_chars: usize) -> String {
    let paragraphs: Vec<String> = outer_paragraphs(n).take(max_lines).map(|p| runs_of(p, max_chars)).collect();
    if paragraphs.is_empty() { runs_of(n, max_chars) } else { paragraphs.join("\n") }
}

#[cfg(test)]
#[path = "xml_tests.rs"]
mod xml_tests;
