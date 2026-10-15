use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::source::FontFace;

macro_rules! font {
    ($file:literal) => {
        include_bytes!(concat!("../../../../assets/fonts/", $file)).as_slice()
    };
}

const CARLITO: [&[u8]; 4] = [
    font!("carlito-400.ttf"),
    font!("carlito-700.ttf"),
    font!("carlito-400-italic.ttf"),
    font!("carlito-700-italic.ttf"),
];
const CALADEA: [&[u8]; 2] = [font!("caladea-400.ttf"), font!("caladea-700.ttf")];
const LIBERATION: [&[u8]; 6] = [
    font!("liberation-sans-400.ttf"),
    font!("liberation-sans-700.ttf"),
    font!("liberation-sans-400-italic.ttf"),
    font!("liberation-sans-700-italic.ttf"),
    font!("liberation-serif-400.ttf"),
    font!("liberation-mono-400.ttf"),
];
const INTER: [&[u8]; 3] = [font!("inter-300.ttf"), font!("inter-400.ttf"), font!("inter-700.ttf")];

/// `data` avec `fsType` (table `OS/2`, décalage 8) remplacé.
fn with_fs_type(data: &[u8], fs_type: u16) -> Vec<u8> {
    let mut out = data.to_vec();
    let be = |o: usize| u32::from_be_bytes([out[o], out[o + 1], out[o + 2], out[o + 3]]) as usize;
    let tables = usize::from(u16::from_be_bytes([out[4], out[5]]));
    let rec = (0..tables).map(|i| 12 + 16 * i).find(|&r| &out[r..r + 4] == b"OS/2").unwrap();
    let at = be(rec + 8) + 8;
    out[at..at + 2].copy_from_slice(&fs_type.to_be_bytes());
    out
}

/// `data` (une police simple) en collection d'une face : en-tête `ttcf`, décalages des tables reportés.
fn collection(data: &[u8]) -> Vec<u8> {
    let mut font = data.to_vec();
    let tables = usize::from(u16::from_be_bytes([font[4], font[5]]));
    for i in 0..tables {
        let at = 12 + 16 * i + 8;
        let offset = u32::from_be_bytes([font[at], font[at + 1], font[at + 2], font[at + 3]]) + 16;
        font[at..at + 4].copy_from_slice(&offset.to_be_bytes());
    }
    let mut out = b"ttcf\x00\x01\x00\x00\x00\x00\x00\x01\x00\x00\x00\x10".to_vec();
    out.extend(font);
    out
}

/// L'app livre Carlito, Caladea, Liberation et Inter ; le système n'a rien.
fn app() -> FontCatalog {
    let mut c = FontCatalog::new();
    for data in CARLITO.iter().chain(&CALADEA).chain(&LIBERATION).chain(&INTER) {
        c.add(FontOrigin::App, data.to_vec());
    }
    c
}

/// Le système a Liberation Sans ; l'app, Carlito.
fn system() -> FontCatalog {
    let mut c = FontCatalog::new();
    for data in &LIBERATION[..4] {
        c.add(FontOrigin::System, data.to_vec());
    }
    for data in CARLITO {
        c.add(FontOrigin::App, data.to_vec());
    }
    c
}

fn source(family: &str, faces: &[(&[u8], FaceStyle)]) -> SourceFont {
    SourceFont {
        family: family.into(),
        faces: faces
            .iter()
            .map(|(data, style)| FontFace {
                style: *style,
                weight: None,
                fs_type: Some(0),
                subset: false,
                data: data.to_vec(),
            })
            .collect(),
    }
}

fn used(family: &str) -> SourceFont {
    source(family, &[])
}

fn restricted_carlito(family: &str) -> SourceFont {
    let mut f = source(family, &[(CARLITO[0], FaceStyle::Regular)]);
    f.faces[0].fs_type = Some(0x0002);
    f
}

fn subset_caladea(family: &str) -> SourceFont {
    let mut f = source(family, &[(CALADEA[0], FaceStyle::Regular)]);
    f.faces[0].subset = true;
    f
}

fn embedded_caladea(family: &str) -> SourceFont {
    source(family, &[(CALADEA[0], FaceStyle::Regular), (CALADEA[1], FaceStyle::Bold)])
}

fn choice(used: Option<&str>, decision: FontDecision, restricted: bool) -> (Option<String>, FontDecision, bool) {
    (used.map(str::to_owned), decision, restricted)
}

const APP: FontDecision = FontDecision::Available { origin: FontOrigin::App };
const SUB_APP: FontDecision = FontDecision::Substituted { origin: FontOrigin::App };

#[rstest]
#[case::embedded(embedded_caladea("Corporate"), app(), choice(Some("Caladea"), FontDecision::Embedded, false), 2)]
#[case::embedded_restricted_then_substitute(
    restricted_carlito("Calibri"),
    app(),
    choice(Some("Carlito"), SUB_APP, true),
    0
)]
#[case::embedded_restricted_then_missing(
    restricted_carlito("Corporate"),
    app(),
    choice(None, FontDecision::Missing, true),
    0
)]
#[case::embedded_subset_without_catalog(
    subset_caladea("Corporate"),
    app(),
    choice(Some("Caladea"), FontDecision::Embedded, false),
    1
)]
#[case::embedded_subset_behind_catalog(subset_caladea("Caladea"), app(), choice(Some("Caladea"), APP, false), 0)]
#[case::catalog_app(used("Carlito"), app(), choice(Some("Carlito"), APP, false), 0)]
#[case::catalog_system(used("Liberation Sans"), system(), choice(Some("Liberation Sans"), FontDecision::Available { origin: FontOrigin::System }, false), 4)]
#[case::substitute_calibri(used("Calibri"), app(), choice(Some("Carlito"), SUB_APP, false), 0)]
#[case::substitute_calibri_light(used("Calibri Light"), app(), choice(Some("Carlito"), SUB_APP, false), 0)]
#[case::substitute_cambria(used("Cambria"), app(), choice(Some("Caladea"), SUB_APP, false), 0)]
#[case::substitute_arial(used("Arial"), app(), choice(Some("Liberation Sans"), SUB_APP, false), 0)]
#[case::substitute_helvetica(used("Helvetica"), app(), choice(Some("Liberation Sans"), SUB_APP, false), 0)]
#[case::substitute_times(used("Times New Roman"), app(), choice(Some("Liberation Serif"), SUB_APP, false), 0)]
#[case::substitute_courier(used("Courier New"), app(), choice(Some("Liberation Mono"), SUB_APP, false), 0)]
#[case::substitute_from_system(used("Arial"), system(), choice(Some("Liberation Sans"), FontDecision::Substituted { origin: FontOrigin::System }, false), 4)]
#[case::case_insensitive_substitute(used("CALIBRI"), app(), choice(Some("Carlito"), SUB_APP, false), 0)]
#[case::case_insensitive_catalog(used("liberation sans"), app(), choice(Some("Liberation Sans"), APP, false), 0)]
#[case::substitute_not_shipped(used("Cambria"), system(), choice(None, FontDecision::Missing, false), 0)]
#[case::absent(used("Wingdings"), app(), choice(None, FontDecision::Missing, false), 0)]
fn resolve_one_family_decides_and_embeds(
    #[case] font: SourceFont,
    #[case] catalog: FontCatalog,
    #[case] expected: (Option<String>, FontDecision, bool),
    #[case] files: usize,
) {
    let plan = resolve(&[font], &[], &catalog);
    let got: Vec<_> = plan.choices.iter().map(|c| (c.used.clone(), c.decision, c.restricted)).collect();
    assert_that!(got).is_equal_to(vec![expected]);
    assert_that!(plan.files.len()).is_equal_to(files);
}

#[test]
fn resolve_system_family_names_files_after_family_and_style() {
    let plan = resolve(&[used("Liberation Sans")], &[], &system());
    let names: Vec<_> = plan.files.keys().map(String::as_str).collect();
    assert_that!(names).is_equal_to(vec![
        "liberation-sans-bold-italic.ttf",
        "liberation-sans-bold.ttf",
        "liberation-sans-italic.ttf",
        "liberation-sans-regular.ttf",
    ]);
    assert_that!(plan.files.get("liberation-sans-bold.ttf").map(Vec::as_slice)).is_equal_to(Some(LIBERATION[1]));
}

#[test]
fn resolve_embedded_faces_are_named_after_their_own_family() {
    let plan = resolve(&[embedded_caladea("Corporate")], &[], &FontCatalog::new());
    let names: Vec<_> = plan.files.keys().map(String::as_str).collect();
    assert_that!(names).is_equal_to(vec!["caladea-bold.ttf", "caladea-regular.ttf"]);
}

#[test]
fn resolve_twice_gives_the_same_plan() {
    let fonts = [embedded_caladea("Corporate"), used("Liberation Sans"), used("Arial")];
    assert_that!(resolve(&fonts, &["Calibri"], &system())).is_equal_to(resolve(&fonts, &["Calibri"], &system()));
}

#[test]
fn resolve_same_style_twice_suffixes_the_second_file() {
    let editable = with_fs_type(CALADEA[0], 0x0008);
    let font = source("Caladea", &[(CALADEA[0], FaceStyle::Regular), (&editable, FaceStyle::Regular)]);
    let plan = resolve(&[font], &[], &FontCatalog::new());
    let names: Vec<_> = plan.files.keys().map(String::as_str).collect();
    assert_that!(names).is_equal_to(vec!["caladea-regular-2.ttf", "caladea-regular.ttf"]);
}

#[test]
fn resolve_merges_wanted_families_without_case_duplicates() {
    let plan = resolve(&[used("Calibri")], &["calibri", "Cambria", ""], &app());
    let families: Vec<_> = plan.choices.iter().map(|c| c.family.as_str()).collect();
    assert_that!(families).is_equal_to(vec!["Calibri", "Cambria"]);
    assert_that!(plan.used("CAMBRIA")).is_equal_to(Some("Caladea"));
    assert_that!(plan.used("Wingdings")).is_none();
}

#[test]
fn resolve_restricted_system_face_is_skipped_and_reported() {
    let mut catalog = FontCatalog::new();
    catalog.add(FontOrigin::System, with_fs_type(CALADEA[0], 0x0002));
    let plan = resolve(&[used("Caladea")], &[], &catalog);
    let got: Vec<_> = plan.choices.iter().map(|c| (c.decision, c.restricted)).collect();
    assert_that!(got).is_equal_to(vec![(FontDecision::Missing, true)]);
    assert_that!(plan.files.is_empty()).is_true();
}

#[test]
fn resolve_light_and_regular_faces_keeps_the_closest_to_400() {
    let plan = resolve(&[used("Inter")], &[], &{
        let mut c = FontCatalog::new();
        INTER.iter().for_each(|d| _ = c.add(FontOrigin::System, d.to_vec()));
        c
    });
    assert_that!(plan.files.get("inter-regular.ttf").map(Vec::as_slice)).is_equal_to(Some(INTER[1]));
    assert_that!(plan.files.len()).is_equal_to(2);
}

#[test]
fn catalog_add_reads_each_face_of_a_collection() {
    let mut catalog = FontCatalog::new();
    assert_that!(catalog.add(FontOrigin::System, collection(CALADEA[0]))).is_equal_to(1);
    let plan = resolve(&[used("Caladea")], &[], &catalog);
    let names: Vec<_> = plan.files.keys().map(String::as_str).collect();
    assert_that!(names).is_equal_to(vec!["caladea-regular.ttc"]);
}

#[rstest]
#[case(b"ttcf\x00\x01\x00\x00\x00\x00\x00\x02\xff\xff\xff\xff\x00\x00\x00\x00".to_vec())]
#[case(b"not a font".to_vec())]
#[case(Vec::new())]
fn catalog_add_garbage_reads_nothing(#[case] data: Vec<u8>) {
    let mut catalog = FontCatalog::new();
    assert_that!(catalog.add(FontOrigin::System, data)).is_equal_to(0);
}

#[test]
fn catalog_contains_ignores_case_and_style_suffixes() {
    let catalog = app();
    assert_that!(catalog.contains("liberation SERIF")).is_true();
    assert_that!(catalog.contains("Calibri")).is_false();
}

#[rstest]
#[case("Calibri", Some("Carlito"))]
#[case(" calibri light ", Some("Carlito"))]
#[case("Times", Some("Liberation Serif"))]
#[case("Comic Sans MS", None)]
fn substitute_maps_metric_compatible_families(#[case] family: &str, #[case] by: Option<&str>) {
    assert_that!(substitute(family)).is_equal_to(by);
}

#[rstest]
#[case("Liberation Sans", "liberation-sans")]
#[case("Noto Sans CJK JP", "noto-sans-cjk-jp")]
#[case("../../etc", "etc")]
#[case("思源黑体", "font")]
#[case("  Font  Name!! ", "font-name")]
fn slug_keeps_a_safe_file_stem(#[case] family: &str, #[case] stem: &str) {
    assert_that!(slug(family)).is_equal_to(stem.to_owned());
}

#[test]
fn slug_of_a_long_family_is_capped() {
    assert_that!(slug(&"a".repeat(200)).len()).is_equal_to(MAX_SLUG);
}

#[test]
fn resolve_keeps_the_line_metrics_of_each_used_family() {
    let plan = resolve(&[used("Calibri"), used("Arial"), used("Wingdings")], &[], &app());
    let cap = |f: &str| plan.metrics(f).map(|m| (m.cap_height * 1000.0).round());
    assert_that!((cap("calibri"), cap("Arial"), cap("Wingdings"))).is_equal_to((Some(642.0), Some(688.0), None));
    let ascender = plan.metrics("Arial").map(|m| (m.ascender * 1000.0).round());
    assert_that!(ascender).is_equal_to(Some(905.0));
}

#[test]
fn sought_lists_each_family_and_its_substitute_in_lowercase() {
    let found = sought(&[used("Calibri"), used("Wingdings"), used(" ")], &["Times New Roman", "calibri"]);
    let expected: BTreeSet<String> =
        ["calibri", "carlito", "liberation serif", "times new roman", "wingdings"].map(str::to_owned).into();
    assert_that!(found).is_equal_to(expected);
}

/// Bornes : `file` octets par fichier, `total` en tout.
fn caps(file: usize, total: usize) -> Caps {
    Caps { file, total }
}

#[test]
fn resolve_too_large_system_family_falls_to_the_app_substitute() {
    // Arial : son substitut Liberation Sans n’est que du système, et dépasse la borne.
    let small = LIBERATION[..4].iter().map(|d| d.len()).min().unwrap() - 1;
    let plan = resolve_within(&[used("Arial")], &[], &system(), caps(small, usize::MAX));
    let c = &plan.choices[0];
    assert_that!((c.decision, c.too_large)).is_equal_to((FontDecision::Missing, true));
    assert_that!(plan.files.is_empty()).is_true();
    // Calibri prend Carlito de l'app, jamais embarquée : la borne ne compte pas.
    let plan = resolve_within(&[used("Calibri")], &[], &system(), caps(1, 1));
    let c = &plan.choices[0];
    assert_that!((c.decision, c.too_large)).is_equal_to((SUB_APP, false));
}

#[test]
fn resolve_too_large_embedded_faces_fall_to_the_catalog() {
    let font = embedded_caladea("Caladea");
    let plan = resolve_within(&[font], &[], &app(), caps(CALADEA[0].len() - 1, usize::MAX));
    let c = &plan.choices[0];
    assert_that!((c.used.as_deref(), c.decision, c.too_large)).is_equal_to((Some("Caladea"), APP, true));
    assert_that!(plan.files.is_empty()).is_true();
}

#[test]
fn resolve_stops_embedding_past_the_total() {
    // Deux familles embarquées : la seconde ne tient plus dans le total.
    let total = CALADEA[0].len() + CALADEA[1].len();
    let fonts = [embedded_caladea("Alpha"), source("Beta", &[(CARLITO[0], FaceStyle::Regular)])];
    let plan = resolve_within(&fonts, &[], &FontCatalog::new(), caps(usize::MAX, total));
    let got: Vec<_> = plan.choices.iter().map(|c| (c.decision, c.too_large)).collect();
    assert_that!(got).is_equal_to(vec![(FontDecision::Embedded, false), (FontDecision::Missing, true)]);
    assert_that!(plan.files.len()).is_equal_to(2);
}

#[test]
fn resolve_default_caps_are_documented_sizes() {
    assert_that!((MAX_EMBEDDED_FILE, MAX_EMBEDDED_TOTAL)).is_equal_to((32 << 20, 64 << 20));
}

#[test]
fn settle_fallback_takes_the_body_font_when_found() {
    let mut plan = resolve(&[used("Wingdings")], &["Calibri"], &app());
    plan.settle_fallback(Some("Calibri"), &app());
    assert_that!(plan.fallback.as_deref()).is_equal_to(Some("Carlito"));
    let missing = plan.choices.iter().find(|c| c.family == "Wingdings").unwrap();
    assert_that!(missing.fallback.as_deref()).is_equal_to(Some("Carlito"));
}

#[test]
fn settle_fallback_without_the_body_font_takes_a_sans_app_font() {
    let mut catalog = FontCatalog::new();
    for data in &INTER {
        catalog.add(FontOrigin::App, data.to_vec());
    }
    let mut plan = resolve(&[], &["Corporate"], &catalog);
    plan.settle_fallback(Some("Corporate"), &catalog);
    assert_that!(plan.fallback.as_deref()).is_equal_to(Some("Inter"));
    assert_that!(plan.choices[0].fallback.as_deref()).is_equal_to(Some("Inter"));
    // Les métriques d'une famille introuvable sont celles du repli.
    assert_that!(plan.metrics("Corporate")).is_some();
}

#[test]
fn settle_fallback_ignores_system_fonts_and_may_find_nothing() {
    let mut catalog = FontCatalog::new();
    for data in &LIBERATION[..4] {
        catalog.add(FontOrigin::System, data.to_vec());
    }
    let mut plan = resolve(&[], &["Corporate"], &catalog);
    plan.settle_fallback(Some("Corporate"), &catalog);
    assert_that!(plan.fallback).is_none();
    assert_that!(plan.choices[0].fallback).is_none();
}

#[test]
fn catalog_add_keeps_shared_bytes_without_copy() {
    let data: std::sync::Arc<[u8]> = CARLITO[0].into();
    let mut c = FontCatalog::new();
    c.add(FontOrigin::App, ArcBytes(data.clone()));
    assert_that!(c.faces[0].data.as_ref().as_ref().as_ptr()).is_equal_to(data.as_ptr());
}

/// Des octets partagés, comme ceux des polices du rendu.
struct ArcBytes(std::sync::Arc<[u8]>);

impl AsRef<[u8]> for ArcBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
