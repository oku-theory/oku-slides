//! `docProps/app.xml`: which application wrote the file.
//!
//! PowerPoint 2007 wrote some booleans by its own draft schema, whose
//! defaults differ from the standard: an absent or bare `c:delete` there
//! means shown (t-1385). LibreOffice's oox tells such a file by the same
//! signs: an `Application` of Microsoft and an `AppVersion` of 12.

use std::cell::RefCell;

use crate::collect::Sink;
use crate::error::ImportError;
use crate::pptx::package::Package;
use crate::pptx::soft;
use crate::pptx::xml::{child, parse};

/// Relationship type of the extended properties, by its suffix.
const EXTENDED_PROPERTIES: &str = "/extended-properties";

/// True if `pkg` says PowerPoint 2007 wrote it.
///
/// A missing or unreadable `app.xml` says no (warned `UnreadablePart` when unreadable).
///
/// # Errors
///
/// Fails only on a read limit (`TooLarge`).
pub(crate) fn written_by_mso2007(pkg: &Package<'_>, sink: &RefCell<Sink>) -> Result<bool, ImportError> {
    let Some(part) = soft(pkg.rels(""), sink, "_rels/.rels")?
        .and_then(|r| r.by_type(EXTENDED_PROPERTIES).first().map(|(_, t)| (*t).to_owned()))
    else {
        return Ok(false);
    };
    if !pkg.exists(&part) {
        return Ok(false);
    }
    let Some(text) = soft(pkg.text(&part), sink, &part)? else { return Ok(false) };
    let Some(doc) = soft(parse(&text, &part, pkg.limits().max_xml_nodes), sink, &part)? else { return Ok(false) };
    let root = doc.root_element();
    let value = |name: &str| child(root, name).and_then(|n| n.text()).map(str::trim).unwrap_or_default().to_owned();
    Ok(value("Application").starts_with("Microsoft") && value("AppVersion").starts_with("12."))
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod app_tests;
