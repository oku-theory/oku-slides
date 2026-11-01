//! Regression (t-1385): PowerPoint 2007 wrote `<c:delete/>` for a shown axis (its schema
//! defaulted to false); read as deleted, its charts lost their axes. The reader now takes the
//! default from `docProps/app.xml`, and the flag must reach the chart reader through the import.

use oku_slides_import::read_pptx;
use oku_slides_import::source::{Axes, ItemKind};
use rstest::rstest;
use speculoos::prelude::*;

use crate::common;

const RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties" Target="docProps/app.xml"/></Relationships>"#;

/// The axes of kitchen's chart once both axes carry a bare `<c:delete/>`, written by `app`.
fn axes_written_by(app: &str) -> Axes {
    let chart = common::text("kitchen/ppt/charts/chart1.xml")
        .replacen("</c:barChart>", r#"<c:axId val="1"/><c:axId val="2"/></c:barChart>"#, 1)
        .replacen(
            "</c:plotArea>",
            r#"<c:catAx><c:axId val="1"/><c:delete/><c:axPos val="b"/></c:catAx><c:valAx><c:axId val="2"/><c:delete/><c:axPos val="l"/></c:valAx></c:plotArea>"#,
            1,
        );
    let props = format!(
        r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties">{app}</Properties>"#
    );
    let bytes = common::pptx_plus(
        "kitchen",
        &[("ppt/charts/chart1.xml", Some(&chart)), ("_rels/.rels", Some(RELS))],
        &[("docProps/app.xml", props.as_bytes())],
    );
    let deck = read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("read: {e}"));
    let chart = deck.slides.iter().flat_map(|s| &s.items).find_map(|i| match &i.kind {
        ItemKind::Chart(c) => Some(c.axes),
        _ => None,
    });
    chart.unwrap_or_else(|| panic!("no chart in kitchen"))
}

#[rstest]
#[case::powerpoint_2007(
    "<Application>Microsoft Office PowerPoint</Application><AppVersion>12.0000</AppVersion>",
    Axes { x: true, y: true }
)]
#[case::powerpoint_2016(
    "<Application>Microsoft Office PowerPoint</Application><AppVersion>16.0000</AppVersion>",
    Axes { x: false, y: false }
)]
fn issue_1385_bare_delete_follows_the_app_that_wrote_the_file(#[case] app: &str, #[case] expected: Axes) {
    assert_that!(axes_written_by(app)).is_equal_to(expected);
}
