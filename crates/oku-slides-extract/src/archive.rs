//! The zip side of OOXML and ODF documents: the bounded archive, its XML parts and their
//! relationships, with import errors turned into refusals.

use std::collections::BTreeMap;

use oku_slides_core::knowledge::ExtractRefusal;
use oku_slides_import::bounded::{BoundedZip, parse_xml, resolve_path};
use oku_slides_import::{ImportError, ImportErrorKind, ReadLimits};
use roxmltree::{Document, Node};

/// `EncryptionInfo` in UTF-16LE: the stream name of an encrypted OOXML compound file.
const ENCRYPTION_INFO: &[u8] = b"E\0n\0c\0r\0y\0p\0t\0i\0o\0n\0I\0n\0f\0o\0";

/// One internal relationship of a part, its target resolved to a package path.
#[derive(Debug)]
pub(crate) struct Rel {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) target: String,
}

/// The refusal for a read error: a passed limit is `TooLarge`, anything else `Damaged`.
pub(crate) fn refusal(e: &ImportError) -> ExtractRefusal {
    match e.kind() {
        ImportErrorKind::TooLarge => ExtractRefusal::TooLarge,
        _ => ExtractRefusal::Damaged,
    }
}

/// Opens the archive. A compound file is an encrypted document or a legacy Office one; bytes
/// that are not a zip at all are another kind of file.
pub(crate) fn open<'b>(bytes: &'b [u8], read: &ReadLimits) -> Result<BoundedZip<'b>, ExtractRefusal> {
    BoundedZip::open(bytes, read).map_err(|e| match e.kind() {
        ImportErrorKind::Protected if bytes.windows(ENCRYPTION_INFO.len()).any(|w| w == ENCRYPTION_INFO) => {
            ExtractRefusal::Encrypted
        }
        ImportErrorKind::Protected => ExtractRefusal::Legacy,
        ImportErrorKind::Unreadable if !bytes.starts_with(b"PK") => ExtractRefusal::TypeMismatch,
        _ => refusal(&e),
    })
}

/// The UTF-8 text of a part.
pub(crate) fn text(zip: &BoundedZip<'_>, part: &str) -> Result<String, ExtractRefusal> {
    zip.text(part).map_err(|e| refusal(&e))
}

/// A part's XML, bounded by `max_xml_nodes`.
pub(crate) fn xml<'i>(zip: &BoundedZip<'_>, text: &'i str, part: &str) -> Result<Document<'i>, ExtractRefusal> {
    parse_xml(text, part, zip.limits().max_xml_nodes).map_err(|e| refusal(&e))
}

/// The attribute `local` of a node, whatever its namespace.
pub(crate) fn attr<'a>(node: Node<'a, '_>, local: &str) -> Option<&'a str> {
    node.attributes().find(|a| a.name() == local).map(|a| a.value())
}

/// The children of `node` whose local name is `local`.
pub(crate) fn children<'a, 'i>(node: Node<'a, 'i>, local: &'static str) -> impl Iterator<Item = Node<'a, 'i>> {
    node.children().filter(move |n| n.is_element() && n.tag_name().name() == local)
}

/// The internal relationships of `part` (none when it has no relationship part).
pub(crate) fn rels(zip: &BoundedZip<'_>, part: &str) -> Result<Vec<Rel>, ExtractRefusal> {
    Ok(rels_and_links(zip, part)?.0)
}

/// The internal relationships of `part`, and the targets of its external hyperlinks by id, as
/// written (none when it has no relationship part).
pub(crate) fn rels_and_links(
    zip: &BoundedZip<'_>,
    part: &str,
) -> Result<(Vec<Rel>, BTreeMap<String, String>), ExtractRefusal> {
    let path = match part.rsplit_once('/') {
        Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
        None => format!("_rels/{part}.rels"),
    };
    if !zip.exists(&path) {
        return Ok((Vec::new(), BTreeMap::new()));
    }
    let text = text(zip, &path)?;
    let doc = xml(zip, &text, &path)?;
    let (mut rels, mut links) = (Vec::new(), BTreeMap::new());
    for r in children(doc.root_element(), "Relationship") {
        let (Some(id), Some(kind), Some(target)) = (attr(r, "Id"), attr(r, "Type"), attr(r, "Target")) else {
            continue;
        };
        if attr(r, "TargetMode") != Some("External") {
            rels.push(Rel { id: id.to_owned(), kind: kind.to_owned(), target: resolve_path(part, target) });
        } else if kind.ends_with("/hyperlink") {
            links.insert(id.to_owned(), target.to_owned());
        }
    }
    Ok((rels, links))
}

/// The main part of an OOXML package (its `officeDocument`); a zip without one is another
/// kind of file.
pub(crate) fn main_part(zip: &BoundedZip<'_>) -> Result<String, ExtractRefusal> {
    if !zip.exists("_rels/.rels") {
        return Err(ExtractRefusal::TypeMismatch);
    }
    rels(zip, "")?
        .into_iter()
        .find(|r| r.kind.ends_with("/officeDocument"))
        .map(|r| r.target)
        .ok_or(ExtractRefusal::TypeMismatch)
}

#[cfg(test)]
#[path = "archive_tests.rs"]
mod archive_tests;
