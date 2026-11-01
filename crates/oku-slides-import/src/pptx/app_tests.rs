use std::cell::RefCell;

use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::pptx::test_support::{CT, LIMITS, zip_of};

const RELS_WITH_APP: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;

/// Whether a package whose `app.xml` holds `app` (none if `None`) reads as PowerPoint 2007.
fn mso2007(app: Option<&str>) -> bool {
    let xml = app.map(|a| format!(r#"<Properties xmlns="urn:x">{a}</Properties>"#));
    let mut parts: Vec<(&str, &[u8])> = vec![
        ("[Content_Types].xml", CT.as_bytes()),
        ("_rels/.rels", RELS_WITH_APP.as_bytes()),
        ("ppt/presentation.xml", b"<p/>"),
    ];
    if let Some(x) = &xml {
        parts.push(("docProps/app.xml", x.as_bytes()));
    }
    let bytes = zip_of(&parts);
    let pkg = Package::open(&bytes, &LIMITS).unwrap_or_else(|e| panic!("{e}"));
    written_by_mso2007(&pkg, &RefCell::new(Sink::new())).unwrap_or_else(|e| panic!("{e}"))
}

#[rstest]
#[case::powerpoint_2007(
    Some("<Application>Microsoft Office PowerPoint</Application><AppVersion>12.0000</AppVersion>"),
    true
)]
#[case::powerpoint_2016(
    Some("<Application>Microsoft Office PowerPoint</Application><AppVersion>16.0000</AppVersion>"),
    false
)]
#[case::libreoffice(Some("<Application>LibreOffice/7.6</Application><AppVersion>12.0000</AppVersion>"), false)]
#[case::no_version(Some("<Application>Microsoft Office PowerPoint</Application>"), false)]
#[case::no_app_part(None, false)]
fn written_by_mso2007_reads_application_and_version(#[case] app: Option<&str>, #[case] expected: bool) {
    assert_that!(mso2007(app)).is_equal_to(expected);
}

#[test]
fn written_by_mso2007_unreadable_app_part_says_no_and_warns() {
    let bytes = zip_of(&[
        ("[Content_Types].xml", CT.as_bytes()),
        ("_rels/.rels", RELS_WITH_APP.as_bytes()),
        ("ppt/presentation.xml", b"<p/>"),
        ("docProps/app.xml", b"<Properties"),
    ]);
    let pkg = Package::open(&bytes, &LIMITS).unwrap_or_else(|e| panic!("{e}"));
    let sink = RefCell::new(Sink::new());
    let read = written_by_mso2007(&pkg, &sink).ok();
    let warned = sink.into_inner().finish().map(|(_, w)| w.len()).ok();
    assert_that!((read, warned)).is_equal_to((Some(false), Some(1)));
}
