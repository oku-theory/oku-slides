//! Régression (t-692, lot 4) : un `<a:ext cx>` négatif, gardé par le lecteur, donnait à
//! `metrics::score` un rectangle à l'envers et faisait paniquer l'import hi-fi (tranche
//! `owner[x0..x1]` avec `x1 < x0`).

use oku_slides_import::metrics::{Image, regions, score};
use oku_slides_import::read_pptx;
use speculoos::prelude::*;

use crate::common;

#[test]
fn score_of_a_shape_with_a_negative_width_skips_it_without_panicking() {
    let slide = common::text("minimal/ppt/slides/slide3.xml").replace(r#"cx="3048000""#, r#"cx="-3048000""#);
    let deck = read_pptx(&common::pptx_with("minimal", &[("ppt/slides/slide3.xml", Some(&slide))]), &common::limits())
        .unwrap_or_else(|e| panic!("lecture : {e}"));
    let page = deck.slides.iter().find(|s| s.number == 3).unwrap_or_else(|| panic!("pas de slide 3"));
    assert_that!(page.items.iter().any(|i| i.frame.w < 0.0)).is_true();
    let white: Vec<u8> = (0..960 * 540).flat_map(|_| [255, 255, 255, 255]).collect();
    let img = Image { width: 960, height: 540, rgba: &white };
    let s = score(img, img, &regions(page, 960.0 / deck.size.w)).unwrap_or_else(|| panic!("pas de score"));
    assert_that!(s.regions.iter().all(|r| r.score == 0.0)).is_true();
}
