//! Régression (t-692) : une part trop grande était redécompressée à chaque référence (bombe zip).
//!
//! Mille images pointent vers une même part de `max_part_bytes + 1` octets nuls (quelques Ko
//! compressés) : avant le correctif, chaque référence la décompressait à nouveau, sans que rien
//! ne compte dans `max_total_bytes` (8 Go décompressés ici avant `TooLarge`). Après : la première
//! lecture trop grande épuise le paquet, les suivantes ne lisent plus rien.
//!
//! Ce test de bout en bout vérifie l'issue (`TooLarge`) ; le nombre d'octets décompressés, seul
//! témoin déterministe d'une relecture, est compté par
//! `pptx::context::context_tests::oversized_parts_read_many_times_are_decompressed_once`.

use oku_slides_import::{ImportErrorKind, ReadLimits, read_pptx};
use speculoos::prelude::*;

use crate::common;

const PART: u64 = 8 << 20;
const REFS: usize = 1000;

fn bomb() -> Vec<u8> {
    let pic = |i: usize| {
        format!(
            r#"<p:pic><p:nvPicPr><p:cNvPr id="{}" name="p"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="rIdB"/></p:blipFill><p:spPr/></p:pic>"#,
            100 + i
        )
    };
    let slide = common::text("minimal/ppt/slides/slide1.xml")
        .replace("</p:spTree>", &format!("{}</p:spTree>", (0..REFS).map(pic).collect::<String>()));
    let rels = common::text("minimal/ppt/slides/_rels/slide1.xml.rels").replace(
        "</Relationships>",
        r#"<Relationship Id="rIdB" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/bomb.png"/></Relationships>"#,
    );
    let image = vec![0u8; usize::try_from(PART + 1).unwrap_or_else(|e| panic!("{e}"))];
    common::pptx_plus(
        "minimal",
        &[("ppt/slides/slide1.xml", Some(&slide)), ("ppt/slides/_rels/slide1.xml.rels", Some(&rels))],
        &[("ppt/media/bomb.png", &image)],
    )
}

#[test]
fn oversized_part_referenced_many_times_is_decompressed_once() {
    let bytes = bomb();
    let limits = ReadLimits { max_part_bytes: PART, max_total_bytes: 64 << 20, ..ReadLimits::default() };
    let kind = read_pptx(&bytes, &limits).map(|_| ()).map_err(|e| e.kind()).err();
    assert_that!(kind).is_equal_to(Some(ImportErrorKind::TooLarge));
}
