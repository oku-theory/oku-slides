use speculoos::prelude::*;

use super::*;
use crate::source::ItemKind;
use crate::test_support::{item, source_slide, text_item};

/// Une image `w × h` d'une seule couleur.
fn plain(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
    (0..w * h).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect()
}

/// `rgba` (`w` de large) avec le rectangle `x, y, rw, rh` peint en `rgb`.
fn with_rect(mut rgba: Vec<u8>, w: u32, (x, y, rw, rh): (u32, u32, u32, u32), rgb: [u8; 3]) -> Vec<u8> {
    for row in y..y + rh {
        for col in x..x + rw {
            let i = ((row * w + col) * 4) as usize;
            rgba[i..i + 3].copy_from_slice(&rgb);
        }
    }
    rgba
}

fn image(w: u32, h: u32, rgba: &[u8]) -> Image<'_> {
    Image { width: w, height: h, rgba }
}

fn region(item: &str, (x, y, w, h): (f64, f64, f64, f64)) -> Region {
    Region { item: item.to_owned(), x, y, w, h }
}

fn items(s: &SlideScore) -> Vec<&str> {
    s.regions.iter().map(|r| r.item.as_str()).collect()
}

#[test]
fn score_of_identical_images_is_zero() {
    let white = plain(40, 20, [255, 255, 255]);
    let s = score(image(40, 20, &white), image(40, 20, &white), &[region("a", (0.0, 0.0, 20.0, 20.0))]).unwrap();
    assert_that!((s.score, s.regions[0].score, s.regions[0].shift)).is_equal_to((0.0, 0.0, [0, 0]));
}

#[test]
fn score_of_white_against_black_is_one_hundred() {
    let (white, black) = (plain(20, 10, [255, 255, 255]), plain(20, 10, [0, 0, 0]));
    let s = score(image(20, 10, &white), image(20, 10, &black), &[]).unwrap();
    assert_that!(s.score).is_close_to(100.0, 0.01);
}

#[test]
fn score_designates_the_region_that_differs() {
    let white = plain(100, 50, [255, 255, 255]);
    let marked = with_rect(white.clone(), 100, (60, 20, 12, 12), [255, 0, 0]);
    // Les noms vont à rebours de l'ordre alphabétique, qui départage les égalités.
    let regions = [region("a-gauche", (0.0, 0.0, 50.0, 50.0)), region("z-droite", (50.0, 0.0, 50.0, 50.0))];
    let s = score(image(100, 50, &white), image(100, 50, &marked), &regions).unwrap();
    assert_that!(items(&s)).is_equal_to(vec!["z-droite", "a-gauche"]);
}

#[test]
fn score_realigns_a_small_shift_and_reports_it() {
    let white = plain(60, 40, [255, 255, 255]);
    let (a, b) =
        (with_rect(white.clone(), 60, (20, 10, 12, 12), [0, 0, 0]), with_rect(white, 60, (22, 11, 12, 12), [0, 0, 0]));
    let s = score(image(60, 40, &a), image(60, 40, &b), &[region("carre", (10.0, 0.0, 40.0, 40.0))]).unwrap();
    assert_that!((s.regions[0].score, s.regions[0].shift)).is_equal_to((0.0, [2, 1]));
}

#[test]
fn score_keeps_a_shift_beyond_max_shift_as_a_gap() {
    let white = plain(80, 40, [255, 255, 255]);
    let (a, b) =
        (with_rect(white.clone(), 80, (10, 10, 12, 12), [0, 0, 0]), with_rect(white, 80, (40, 10, 12, 12), [0, 0, 0]));
    let s = score(image(80, 40, &a), image(80, 40, &b), &[region("carre", (0.0, 0.0, 80.0, 40.0))]).unwrap();
    assert_that!(s.regions[0].score).is_greater_than(20.0);
}

#[test]
fn score_gives_a_local_gap_its_full_weight_in_a_large_region() {
    let white = plain(400, 100, [255, 255, 255]);
    let marked = with_rect(white.clone(), 400, (300, 40, 16, 16), [0, 0, 0]);
    let s =
        score(image(400, 100, &white), image(400, 100, &marked), &[region("cadre", (0.0, 0.0, 400.0, 100.0))]).unwrap();
    // Sur tout le cadre, l'écart moyen vaudrait 0,64 ; sa pire tuile le porte entier.
    assert_that!(s.regions[0].score).is_greater_than(40.0);
}

#[test]
fn score_gives_the_gap_to_the_item_on_top() {
    let white = plain(100, 60, [255, 255, 255]);
    let marked = with_rect(white.clone(), 100, (40, 20, 16, 16), [0, 0, 0]);
    let regions = [region("carte", (0.0, 0.0, 100.0, 60.0)), region("texte", (30.0, 10.0, 40.0, 40.0))];
    let s = score(image(100, 60, &white), image(100, 60, &marked), &regions).unwrap();
    let carte = s.regions.iter().find(|r| r.item == "carte").map(|r| r.score);
    assert_that!((items(&s)[0], carte)).is_equal_to(("texte", Some(0.0)));
}

#[test]
fn score_skips_a_region_with_too_few_pixels_of_its_own() {
    let white = plain(40, 20, [255, 255, 255]);
    let s = score(image(40, 20, &white), image(40, 20, &white), &[region("filet", (0.0, 0.0, 30.0, 2.0))]).unwrap();
    assert_that!(s.regions).is_empty();
}

#[test]
fn score_keeps_the_worst_regions_only() {
    let white = plain(200, 40, [255, 255, 255]);
    // r1, r3 et r4 portent un carré noir, de plus en plus grand ; r0 et r2 sont identiques.
    let mut marked = white.clone();
    for (i, side) in [(1, 8), (3, 12), (4, 16)] {
        marked = with_rect(marked, 200, (i * 40 + 4, 4, side, side), [0, 0, 0]);
    }
    let regions: Vec<Region> =
        (0..5).map(|i| region(&format!("r{i}"), (f64::from(i) * 40.0, 0.0, 40.0, 40.0))).collect();
    let s = score(image(200, 40, &white), image(200, 40, &marked), &regions).unwrap();
    assert_that!(items(&s)).is_equal_to(vec!["r4", "r3", "r1"]);
}

#[test]
fn score_of_a_region_with_a_negative_width_is_skipped() {
    let white = plain(40, 20, [255, 255, 255]);
    let s = score(image(40, 20, &white), image(40, 20, &white), &[region("envers", (30.0, 0.0, -20.0, 20.0))]).unwrap();
    assert_that!(s.regions).is_empty();
}

#[test]
fn score_counts_the_topmost_max_regions_only() {
    let white = plain(64, 64, [255, 255, 255]);
    let marked = with_rect(white.clone(), 64, (0, 0, 16, 16), [0, 0, 0]);
    // La région du dessous, seule à différer, passe sous la borne : aucune ne la porte.
    let mut regions = vec![region("dessous", (0.0, 0.0, 16.0, 16.0))];
    regions.extend((0..MAX_REGIONS).map(|i| region(&format!("r{i}"), (32.0, 32.0, 32.0, 32.0))));
    let s = score(image(64, 64, &white), image(64, 64, &marked), &regions).unwrap();
    assert_that!(s.regions.iter().all(|r| r.item != "dessous" && r.score == 0.0)).is_true();
}

#[test]
fn worst_is_the_score_of_the_first_region() {
    let white = plain(100, 50, [255, 255, 255]);
    let marked = with_rect(white.clone(), 100, (60, 20, 12, 12), [255, 0, 0]);
    let regions = [region("a", (0.0, 0.0, 50.0, 50.0)), region("b", (50.0, 0.0, 50.0, 50.0))];
    let s = score(image(100, 50, &white), image(100, 50, &marked), &regions).unwrap();
    assert_that!(s.worst()).is_equal_to(Some(s.regions[0].score));
}

#[test]
fn score_skips_a_region_outside_the_image() {
    let white = plain(20, 10, [255, 255, 255]);
    let s = score(image(20, 10, &white), image(20, 10, &white), &[region("dehors", (30.0, 0.0, 50.0, 50.0))]).unwrap();
    assert_that!(s.regions).is_empty();
}

#[test]
fn score_of_images_of_different_sizes_is_none() {
    let (a, b) = (plain(20, 10, [255, 255, 255]), plain(20, 11, [255, 255, 255]));
    assert_that!(score(image(20, 10, &a), image(20, 11, &b), &[])).is_none();
}

#[test]
fn score_of_truncated_bytes_is_none() {
    let a = plain(20, 10, [255, 255, 255]);
    assert_that!(score(image(20, 10, &a), image(20, 10, &a[..40]), &[])).is_none();
}

#[test]
fn regions_open_groups_and_scale_frames() {
    let child = text_item("s1/3", (10.0, 20.0, 30.0, 40.0), "x");
    let group = item("s1/2", (0.0, 0.0, 100.0, 100.0), ItemKind::Group { items: vec![child], smartart: false });
    let slide = source_slide(1, None, vec![text_item("s1/1", (1.0, 2.0, 3.0, 4.0), "y"), group]);
    let r = regions(&slide, 0.5);
    assert_that!(r).is_equal_to(vec![region("s1/1", (0.5, 1.0, 1.5, 2.0)), region("s1/3", (5.0, 10.0, 15.0, 20.0))]);
}
