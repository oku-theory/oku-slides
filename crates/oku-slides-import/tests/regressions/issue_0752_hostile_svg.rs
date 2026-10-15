//! Régression (t-752) : un SVG du fichier était servi tel quel (script, liens externes, entités),
//! et le type d'un média venait de son extension.

use oku_slides_import::read_pptx;
use oku_slides_import::source::{ItemKind, WarningKind};
use speculoos::prelude::*;

use crate::common;

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0-raster";
const HOSTILE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" onload="alert(1)"><script>alert(2)</script><image xlink:href="https://evil.example/x.png"/><rect width="1" height="1" style="fill:url(https://evil.example/p)"/></svg>"#;
const BOMB: &str = r#"<!DOCTYPE svg [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">]><svg xmlns="http://www.w3.org/2000/svg"><text>&b;</text></svg>"#;

fn pic(id: u32, raster: &str, svg: &str) -> String {
    format!(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="{id}" name="Icône"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="{raster}"><a:extLst><a:ext uri="{{96DAC541-7B7A-43D3-8B79-37D633B846F1}}"><asvg:svgBlip xmlns:asvg="http://schemas.microsoft.com/office/drawing/2016/SVG/main" r:embed="{svg}"/></a:ext></a:extLst></a:blip></p:blipFill><p:spPr/></p:pic>"#
    )
}

fn rel(id: &str, target: &str) -> String {
    format!(
        r#"<Relationship Id="{id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="{target}"/>"#
    )
}

#[test]
fn hostile_svg_is_served_clean_and_bomb_falls_back_to_raster() {
    let shapes = format!("{}{}", pic(50, "rIdP", "rIdS"), pic(51, "rIdP", "rIdB"));
    let rels = format!(
        "{}{}{}",
        rel("rIdP", "../media/raster.svg"),
        rel("rIdS", "../media/hostile.png"),
        rel("rIdB", "../media/bomb.svg")
    );
    let bytes = common::minimal_slide1_plus(
        &shapes,
        &rels,
        &[
            ("ppt/media/raster.svg", PNG),
            ("ppt/media/hostile.png", HOSTILE.as_bytes()),
            ("ppt/media/bomb.svg", BOMB.as_bytes()),
        ],
    );
    let deck = read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("lecture : {e}"));
    let assets: Vec<_> = deck.slides[0]
        .items
        .iter()
        .filter_map(|i| match &i.kind {
            ItemKind::Picture(p) => Some(&deck.assets[&p.asset]),
            _ => None,
        })
        .collect();
    let mimes: Vec<_> = assets.iter().map(|a| a.mime.as_str()).collect();
    assert_that!(mimes).is_equal_to(vec!["image/svg+xml", "image/png"]);
    let svg = String::from_utf8_lossy(&assets[0].bytes);
    assert_that!(["alert", "evil.example", "script", "onload"].iter().any(|n| svg.contains(n))).is_false();
    assert_that!(svg.contains("<rect width=\"1\" height=\"1\"/>")).is_true();
    let unsafe_svg: Vec<_> = deck
        .warnings
        .iter()
        .filter_map(|w| match &w.kind {
            WarningKind::UnsafeSvg(p) => Some(p.as_str()),
            _ => None,
        })
        .collect();
    assert_that!(unsafe_svg).is_equal_to(vec!["ppt/media/hostile.png", "ppt/media/bomb.svg"]);
}
