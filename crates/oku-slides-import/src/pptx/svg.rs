//! Assainissement d'un SVG venu du fichier : liste blanche d'éléments et d'attributs, re-sérialisé.
//!
//! Le DOCTYPE n'admet que des entités internes de texte littéral (celles d'Illustrator) : ni
//! entité paramètre ou externe, ni référence ou balise dans leur valeur, au plus
//! [`MAX_ENTITIES`], et leur expansion totale bornée par la sortie admise. Une entité ne se
//! déplie donc qu'une fois, linéairement (pas de « billion laughs »), sans ressource externe ;
//! toute autre déclaration (`<!ATTLIST`, `<!ELEMENT`…) fait refuser le SVG. Le DOCTYPE est
//! omis à la sortie (entités dépliées), comme commentaires et instructions. Les éléments et attributs d'autres
//! espaces de noms (métadonnées d'éditeur) sont omis sans signalement ; tout autre retrait marque
//! le SVG comme modifié.
//!
//! Les `use` ne s'enchaînent pas : un `use` dont la cible contient un `use` (lui-même compris)
//! fait refuser le SVG, et la somme des nœuds de leurs cibles est bornée par la limite de nœuds,
//! si bien que le rendu ne déplie jamais plus qu'un document de taille admise (bombe de `use`).

use std::collections::HashMap;

use roxmltree::{Attribute, Document, Node};

use crate::error::{ImportError, structure, too_large};
use crate::pptx::xml::parse_doctype;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XLINK_NS: &str = "http://www.w3.org/1999/xlink";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// Déclarations de DTD : hors entités littérales du DOCTYPE, leur présence fait rejeter le SVG.
const DECLARATIONS: [&str; 4] = ["<!ENTITY", "<!ATTLIST", "<!ELEMENT", "<!NOTATION"];

/// Entités internes admises dans un DOCTYPE.
const MAX_ENTITIES: usize = 64;

/// Éléments SVG gardés : formes, texte, peinture, filtres ; ni script, ni animation, ni contenu étranger.
const ALLOWED: [&str; 59] = [
    "a",
    "circle",
    "clipPath",
    "defs",
    "desc",
    "ellipse",
    "feBlend",
    "feColorMatrix",
    "feComponentTransfer",
    "feComposite",
    "feConvolveMatrix",
    "feDiffuseLighting",
    "feDisplacementMap",
    "feDistantLight",
    "feDropShadow",
    "feFlood",
    "feFuncA",
    "feFuncB",
    "feFuncG",
    "feFuncR",
    "feGaussianBlur",
    "feImage",
    "feMerge",
    "feMergeNode",
    "feMorphology",
    "feOffset",
    "fePointLight",
    "feSpecularLighting",
    "feSpotLight",
    "feTile",
    "feTurbulence",
    "filter",
    "g",
    "hatch",
    "hatchpath",
    "image",
    "line",
    "linearGradient",
    "marker",
    "mask",
    "path",
    "pattern",
    "polygon",
    "polyline",
    "radialGradient",
    "rect",
    "solidColor",
    "stop",
    "style",
    "svg",
    "switch",
    "symbol",
    "text",
    "textPath",
    "title",
    "tref",
    "tspan",
    "use",
    "view",
];

/// Images raster admises dans une URI `data:` (jamais un SVG, qui échapperait à l'assainissement).
const DATA_IMAGES: [&str; 5] =
    ["data:image/png", "data:image/jpeg", "data:image/jpg", "data:image/gif", "data:image/webp"];

/// SVG assaini : ses octets re-sérialisés, et si quelque chose de risqué en a été retiré.
#[derive(Debug)]
pub(crate) struct Clean {
    /// Le SVG re-sérialisé, UTF-8, sans prologue ni DOCTYPE.
    pub bytes: Vec<u8>,
    /// Vrai si un élément ou un attribut risqué a été retiré (à signaler).
    pub altered: bool,
}

/// Assainit le SVG `bytes` lu à `part`, au plus `nodes` nœuds et `max_bytes` octets en sortie.
///
/// `Unreadable` s'il est illisible, déclare autre chose que des entités littérales (ou les déplie
/// au-delà de `max_bytes`), n'a pas pour racine `svg` dans l'espace SVG
/// ou enchaîne ses `use` ; `TooLarge` au-delà de `nodes` ou si la sortie (échappée, donc parfois
/// plus longue que l'entrée) dépasse `max_bytes`.
pub(crate) fn sanitize(bytes: &[u8], part: &str, nodes: u32, max_bytes: usize) -> Result<Clean, ImportError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| structure(&format!("{part}: SVG is not UTF-8")))?;
    check_dtd(text, part, max_bytes)?;
    let doc = parse_doctype(text, part, nodes)?;
    let root = doc.root_element();
    if !is_svg(root, "svg") {
        return Err(structure(&format!("{part}: root is not an SVG element")));
    }
    check_uses(&doc, part, nodes)?;
    let mut w = Writer { out: String::with_capacity(text.len().min(max_bytes)), altered: false, max_bytes };
    w.element(root, true);
    if w.out.len() > max_bytes {
        return Err(too_large(&format!("{part}: sanitized SVG over {max_bytes} bytes")));
    }
    Ok(Clean { bytes: w.out.into_bytes(), altered: w.altered })
}

/// Refuse toute déclaration de DTD sauf, dans le DOCTYPE, au plus [`MAX_ENTITIES`] entités
/// internes de texte littéral dont l'expansion totale tient dans `max_bytes`.
fn check_dtd(text: &str, part: &str, max_bytes: usize) -> Result<(), ImportError> {
    if !DECLARATIONS.iter().any(|d| text.contains(d)) {
        return Ok(());
    }
    let refuse = |why: &str| structure(&format!("{part}: SVG DTD {why}"));
    let Doctype { before, after, entities } =
        literal_entities(text).ok_or_else(|| refuse("beyond literal entities"))?;
    if DECLARATIONS.iter().any(|d| before.contains(d) || after.contains(d)) {
        return Err(refuse("declared outside the DOCTYPE"));
    }
    let mut left = max_bytes;
    for (name, value) in entities {
        let uses = after.matches(&format!("&{name};")).count();
        left = uses
            .checked_mul(value.len())
            .and_then(|n| left.checked_sub(n))
            .ok_or_else(|| refuse(&format!("entities expand beyond {max_bytes} bytes")))?;
    }
    Ok(())
}

/// Un texte coupé autour de son DOCTYPE à sous-ensemble interne.
struct Doctype<'t> {
    /// Ce qui précède le DOCTYPE.
    before: &'t str,
    /// Ce qui le suit.
    after: &'t str,
    /// Ses entités : nom et texte littéral.
    entities: Vec<(&'t str, &'t str)>,
}

/// Coupe `text` autour de son DOCTYPE à sous-ensemble interne ; `None` si ce sous-ensemble
/// déclare autre chose que des entités internes de texte littéral.
fn literal_entities(text: &str) -> Option<Doctype<'_>> {
    let start = text.find("<!DOCTYPE")?;
    let header = &text[start..];
    let open = header.find('[').filter(|o| header.find('>').is_some_and(|c| c > *o))?;
    let mut rest = &header[open + 1..];
    let mut entities = Vec::new();
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix(']') {
            let after = r.trim_start().strip_prefix('>')?;
            return Some(Doctype { before: &text[..start], after, entities });
        }
        if let Some(r) = rest.strip_prefix("<!--") {
            rest = &r[r.find("-->")? + 3..];
            continue;
        }
        let r = rest.strip_prefix("<!ENTITY")?;
        let r = r.strip_prefix(|c: char| c.is_ascii_whitespace())?.trim_start();
        let name_len = r.find(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')))?;
        let (name, r) = r.split_at(name_len);
        let r = r.trim_start();
        let quote = r.chars().next().filter(|q| matches!(q, '"' | '\''))?;
        let r = &r[1..];
        let end = r.find(quote)?;
        let value = &r[..end];
        if name.is_empty() || value.contains(['&', '%', '<']) || entities.len() == MAX_ENTITIES {
            return None;
        }
        entities.push((name, value));
        rest = r[end + 1..].trim_start().strip_prefix('>')?;
    }
}

/// Cible d'un lien vers un fragment du document (`#id`), quel que soit son attribut `href`.
fn fragment<'a>(n: Node<'a, '_>) -> Option<&'a str> {
    n.attributes()
        .find(|a| a.name() == "href" && matches!(a.namespace(), None | Some(XLINK_NS)))
        .and_then(|a| a.value().trim().strip_prefix('#'))
}

/// Refuse les `use` enchaînés et borne à `nodes` la somme des nœuds de leurs cibles.
fn check_uses(doc: &Document<'_>, part: &str, nodes: u32) -> Result<(), ImportError> {
    let elements = || doc.descendants().filter(Node::is_element);
    let is_use = |n: &Node<'_, '_>| is_svg(*n, "use");
    if !elements().any(|n| is_use(&n)) {
        return Ok(());
    }
    // Un id en double compte pour chacun de ses porteurs : le rendu peut choisir l'un ou l'autre.
    let mut by_id: HashMap<&str, Vec<Node<'_, '_>>> = HashMap::new();
    for n in elements() {
        if let Some(id) = n.attribute("id") {
            by_id.entry(id).or_default().push(n);
        }
    }
    let mut left = u64::from(nodes);
    for target in elements().filter(is_use).filter_map(fragment).filter_map(|id| by_id.get(id)).flatten() {
        // Parcours borné par ce qui reste : le coût total ne dépasse jamais `nodes` plus le nombre de `use`.
        for n in target.descendants() {
            if is_use(&n) {
                return Err(structure(&format!("{part}: SVG use refers to a use")));
            }
            left = left
                .checked_sub(1)
                .ok_or_else(|| structure(&format!("{part}: SVG use expands beyond {nodes} nodes")))?;
        }
    }
    Ok(())
}

fn is_svg(n: Node<'_, '_>, local: &str) -> bool {
    n.tag_name().namespace() == Some(SVG_NS) && n.tag_name().name() == local
}

/// Sort d'un nœud ou d'un attribut.
enum Verdict {
    Keep,
    /// Omis sans signalement : métadonnées, autre espace de noms.
    Silent,
    /// Omis et signalé.
    Unsafe,
}

fn element_verdict(n: Node<'_, '_>) -> Verdict {
    let name = n.tag_name().name();
    match n.tag_name().namespace() {
        Some(SVG_NS) if name == "style" && !style_safe(n) => Verdict::Unsafe,
        Some(SVG_NS) if ALLOWED.contains(&name) => Verdict::Keep,
        Some(SVG_NS) if name == "metadata" => Verdict::Silent,
        Some(SVG_NS) => Verdict::Unsafe,
        _ => Verdict::Silent,
    }
}

/// Feuille `<style>` sûre : seulement du texte (ni commentaire, ni instruction, ni élément, qui
/// couperaient la lecture de `css_safe`), sûr une fois tous ses morceaux réunis.
fn style_safe(n: Node<'_, '_>) -> bool {
    n.children().all(|c| c.is_text()) && css_safe(&n.children().filter_map(|c| c.text()).collect::<String>())
}

/// Nom de sortie d'un attribut gardé, ou son sort.
fn attr_verdict(a: &Attribute<'_, '_>) -> Result<&'static str, Verdict> {
    let (name, value) = (a.name(), a.value());
    match a.namespace() {
        None if name.get(..2).is_some_and(|p| p.eq_ignore_ascii_case("on")) => Err(Verdict::Unsafe),
        None if name == "href" => {
            if href_safe(value) {
                Ok("")
            } else {
                Err(Verdict::Unsafe)
            }
        }
        None if !css_safe(value) => Err(Verdict::Unsafe),
        None => Ok(""),
        Some(XLINK_NS) if name == "href" => {
            if href_safe(value) {
                Ok("xlink:")
            } else {
                Err(Verdict::Unsafe)
            }
        }
        Some(XML_NS) if name == "space" => Ok("xml:"),
        Some(_) => Err(Verdict::Silent),
    }
}

/// Lien sûr : fragment du document même, ou image raster en URI `data:`.
fn href_safe(v: &str) -> bool {
    let v = v.trim();
    if v.starts_with('#') {
        return true;
    }
    DATA_IMAGES.iter().any(|p| {
        v.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p))
            && matches!(v.as_bytes().get(p.len()), Some(b';' | b','))
    })
}

/// Mots CSS refusés partout : `@import` et les fonctions qui chargent une ressource ou un autre
/// élément (`image()`, `image-set()` et `-webkit-image-set()`, `cross-fade()`, `element()` et
/// `-moz-element()`, `src()`, `attr()` qui peut produire une URL), ou évaluent du script
/// (`expression()`). `url(` est traité à part.
const CSS_FORBIDDEN: [&str; 8] =
    ["@import", "image(", "image-set", "cross-fade(", "element(", "src(", "attr(", "expression("];

/// Valeur CSS ou de présentation sûre : ni échappement, ni mot de [`CSS_FORBIDDEN`], et chaque
/// `url(` vers un fragment.
fn css_safe(v: &str) -> bool {
    let lower = v.to_ascii_lowercase();
    if v.contains('\\') || CSS_FORBIDDEN.iter().any(|k| lower.contains(k)) {
        return false;
    }
    lower.split("url(").skip(1).all(|rest| rest.trim_start().trim_start_matches(['"', '\'']).starts_with('#'))
}

struct Writer {
    out: String,
    altered: bool,
    /// Taille de sortie au-delà de laquelle l'écriture s'arrête (le SVG sera refusé).
    max_bytes: usize,
}

impl Writer {
    fn element(&mut self, n: Node<'_, '_>, root: bool) {
        if self.out.len() > self.max_bytes {
            return;
        }
        let name = n.tag_name().name();
        self.out.push('<');
        self.out.push_str(name);
        if root {
            self.out.push_str(&format!(r#" xmlns="{SVG_NS}" xmlns:xlink="{XLINK_NS}""#));
        }
        for a in n.attributes() {
            match attr_verdict(&a) {
                Ok(prefix) => {
                    self.out.push(' ');
                    self.out.push_str(prefix);
                    self.out.push_str(a.name());
                    self.out.push_str("=\"");
                    escape(a.value(), true, self.max_bytes, &mut self.out);
                    self.out.push('"');
                }
                Err(Verdict::Unsafe) => self.altered = true,
                Err(_) => {}
            }
        }
        let start = self.out.len();
        self.out.push('>');
        for c in n.children() {
            if c.is_element() {
                match element_verdict(c) {
                    Verdict::Keep => self.element(c, false),
                    Verdict::Unsafe => self.altered = true,
                    Verdict::Silent => {}
                }
            } else if let Some(t) = c.text().filter(|_| c.is_text()) {
                escape(t, false, self.max_bytes, &mut self.out);
            }
        }
        if self.out.len() == start + 1 {
            self.out.truncate(start);
            self.out.push_str("/>");
        } else {
            self.out.push_str("</");
            self.out.push_str(name);
            self.out.push('>');
        }
    }
}

/// Échappe `s` pour un texte, ou pour une valeur d'attribut entre guillemets (`attr`) ; s'arrête
/// dès que `out` dépasse `max` octets.
fn escape(s: &str, attr: bool, max: usize, out: &mut String) {
    for c in s.chars() {
        if out.len() > max {
            return;
        }
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            '\t' if attr => out.push_str("&#9;"),
            '\n' if attr => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

#[cfg(test)]
#[path = "svg_tests.rs"]
mod svg_tests;
