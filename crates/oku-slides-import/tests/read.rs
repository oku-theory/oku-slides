//! Lecture de bout en bout de la fixture `minimal`, et ses dégradations.

mod common;

use oku_slides_import::source::{
    Align, Bullet, Fill, Item, ItemKind, Origin, PlaceholderLink, Role, SourceDeck, SourceSlide, Spacing, TextBox,
    WarningKind,
};
use oku_slides_import::{ImportErrorKind, ReadLimits, read_pptx};
use speculoos::prelude::*;

fn minimal() -> SourceDeck {
    read_pptx(&common::pptx("minimal"), &common::limits()).unwrap_or_else(|e| panic!("lecture de minimal : {e}"))
}

fn minimal_with(overrides: &[(&str, Option<&str>)]) -> Result<SourceDeck, oku_slides_import::ImportError> {
    read_pptx(&common::pptx_with("minimal", overrides), &common::limits())
}

fn item<'a>(s: &'a SourceSlide, name: &str) -> &'a Item {
    s.items.iter().find(|i| i.name == name).unwrap_or_else(|| panic!("pas d'item {name}"))
}

fn text(i: &Item) -> &TextBox {
    match &i.kind {
        ItemKind::Text(tb) => tb,
        ItemKind::Shape(s) => s.text.as_ref().unwrap_or_else(|| panic!("forme sans texte")),
        k => panic!("texte attendu, trouvé {k:?}"),
    }
}

fn unreadable_parts(d: &SourceDeck) -> Vec<String> {
    d.warnings
        .iter()
        .filter_map(|w| match &w.kind {
            WarningKind::UnreadablePart(p) => Some(p.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn minimal_has_its_structure_and_no_warning() {
    let d = minimal();
    assert_that!((d.size.w, d.size.h, d.masters.len(), d.themes.len())).is_equal_to((960.0, 540.0, 1, 1));
    assert_that!(d.layouts.iter().map(|l| l.kind.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["title", "obj"]);
    assert_that!(d.slides.iter().map(|s| s.layout.as_deref()).collect::<Vec<_>>()).is_equal_to(vec![
        Some("l1"),
        Some("l2"),
        Some("l2"),
    ]);
    assert_that!(d.warnings.clone()).is_empty();
}

#[test]
fn title_slide_inherits_frame_and_style_from_its_layout() {
    let d = minimal();
    let t = item(&d.slides[0], "Titre 1");
    assert_that!((t.role, t.frame.x)).is_equal_to((Some(Role::Title), 120.0));
    let p = &text(t).paragraphs[0];
    assert_that!((p.runs[0].text.as_str(), p.runs[0].style.size, p.style.align)).is_equal_to((
        "Rapport annuel",
        60.0,
        Align::Center,
    ));
}

#[test]
fn content_slide_inherits_master_frame_and_bullets() {
    let d = minimal();
    let s = &d.slides[1];
    let title = item(s, "Titre 1");
    assert_that!((title.frame.x, text(title).paragraphs[0].runs[0].style.size)).is_equal_to((66.0, 44.0));
    assert_that!(text(title).paragraphs[0].runs[0].style.font.origin.clone()).is_equal_to(Origin::scheme("major"));
    let body = text(item(s, "Espace réservé du contenu 2"));
    let bullets: Vec<String> = body
        .paragraphs
        .iter()
        .map(|p| match &p.style.bullet {
            Bullet::Char { char, .. } => char.clone(),
            _ => String::new(),
        })
        .collect();
    assert_that!(bullets).is_equal_to(vec!["•".to_owned(), "–".to_owned()]);
    assert_that!(body.paragraphs.iter().map(|p| p.runs[0].style.size).collect::<Vec<_>>())
        .is_equal_to(vec![28.0, 24.0]);
    assert_that!(body.paragraphs[0].style.line_spacing).is_equal_to(Spacing::Percent(1.0));
}

#[test]
fn slide_placeholder_links_to_layout_and_master_placeholders() {
    let d = minimal();
    assert_that!(item(&d.slides[1], "Espace réservé du contenu 2").placeholder.clone())
        .is_equal_to(Some(PlaceholderLink { idx: Some(1), layout: Some("l2/3".into()), master: Some("m1/3".into()) }));
}

#[test]
fn plain_textbox_takes_other_style_size_and_presentation_spacing() {
    let d = minimal();
    // otherStyle (16 pt) l'emporte sur defaultTextStyle (20 pt) ; spc n'existe que dans defaultTextStyle.
    let r = &text(item(&d.slides[1], "ZoneTexte 3")).paragraphs[0].runs[0];
    assert_that!((r.style.size, r.style.char_spacing, r.style.color.value.to_hex(), r.style.color.origin.clone()))
        .is_equal_to((16.0, 1.0, "#ff0000".to_owned(), Origin::Explicit));
}

#[test]
fn styled_shape_gets_theme_fill_and_font_ref_color() {
    let d = minimal();
    let ItemKind::Shape(s) = &item(&d.slides[1], "Rectangle 4").kind else { panic!("forme attendue") };
    assert_that!(matches!(&s.fill, Fill::Solid { color } if color.origin == Origin::scheme("accent1"))).is_true();
    let r = &s.text.as_ref().unwrap_or_else(|| panic!("texte attendu")).paragraphs[0].runs[0];
    assert_that!((r.style.color.origin.clone(), r.style.bold)).is_equal_to((Origin::scheme("lt1"), true));
}

#[test]
fn dark_slide_color_map_reaches_text_and_inherited_background() {
    let d = minimal();
    let s = &d.slides[2];
    let title = &text(item(s, "Titre 1")).paragraphs[0].runs[0];
    let plain = &text(item(s, "ZoneTexte 2")).paragraphs[0].runs[0];
    assert_that!((title.style.color.origin.slot(), plain.style.color.origin.slot()))
        .is_equal_to((Some("lt1"), Some("lt1")));
    // Fond hérité du master (bgRef bg1) : bg1 → dk1 sur cette slide, donc noir sous un texte clair.
    let Fill::Solid { color } = &s.background else { panic!("fond plein attendu") };
    assert_that!((color.origin.clone(), color.value.to_hex()))
        .is_equal_to((Origin::Scheme { name: "bg1".into(), slot: "dk1".into(), mods: vec![] }, "#000000".to_owned()));
}

#[test]
fn slide_background_and_notes_are_resolved() {
    let d = minimal();
    assert_that!(matches!(&d.slides[1].background, Fill::Solid { color } if color.origin.slot() == Some("lt1")))
        .is_true();
    assert_that!((d.slides[1].notes.as_str(), d.slides[0].notes.as_str()))
        .is_equal_to(("Dire bonjour.\nPuis\nenchaîner.", ""));
}

#[test]
fn missing_theme_falls_back_to_office_with_a_warning() {
    let d = minimal_with(&[("ppt/theme/theme1.xml", None)]).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(d.themes[0].name.as_str()).is_equal_to("Office");
    assert_that!(unreadable_parts(&d)).contains("ppt/theme/theme1.xml".to_owned());
}

#[test]
fn unreadable_slide_is_omitted_with_a_warning() {
    let d = minimal_with(&[("ppt/slides/slide2.xml", Some("<p:sld"))]).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(d.slides.iter().map(|s| s.id.as_str()).collect::<Vec<_>>()).is_equal_to(vec!["s1", "s3"]);
    assert_that!(unreadable_parts(&d)).contains("ppt/slides/slide2.xml".to_owned());
}

#[test]
fn unreadable_layout_leaves_its_slides_without_layout() {
    let d =
        minimal_with(&[("ppt/slideLayouts/slideLayout2.xml", Some("<p:sldLayout"))]).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(d.slides.iter().map(|s| s.layout.as_deref()).collect::<Vec<_>>()).is_equal_to(vec![
        Some("l1"),
        None,
        None,
    ]);
    assert_that!(unreadable_parts(&d)).contains("ppt/slideLayouts/slideLayout2.xml".to_owned());
}

#[test]
fn missing_master_is_fatal_unreadable() {
    let err = minimal_with(&[("ppt/slideMasters/slideMaster1.xml", None)]).map(|_| ()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::Unreadable);
}

#[test]
fn too_many_slides_is_too_large() {
    let err =
        read_pptx(&common::pptx("minimal"), &ReadLimits { max_slides: 2, ..common::limits() }).map(|_| ()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn oversized_structure_part_is_too_large_not_a_warning() {
    // Le master (2 465 octets) dépasse seul la borne : une limite n'est jamais dégradée en `UnreadablePart`.
    let err = read_pptx(&common::pptx("minimal"), &ReadLimits { max_part_bytes: 2_400, ..common::limits() })
        .map(|_| ())
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn notes_part_over_the_node_limit_is_too_large() {
    // Les notes dépassent seules la borne de nœuds : fatal, comme toute part liée.
    let notes = common::text("minimal/ppt/notesSlides/notesSlide1.xml")
        .replace("<a:p><a:r><a:t>Dire", &format!("{}<a:p><a:r><a:t>Dire", "<a:p/>".repeat(20_000)));
    let limits = ReadLimits { max_xml_nodes: 10_000, ..common::limits() };
    read_pptx(&common::pptx("minimal"), &limits).unwrap_or_else(|e| panic!("minimal tient dans la borne : {e}"));
    let err = read_pptx(&common::pptx_with("minimal", &[("ppt/notesSlides/notesSlide1.xml", Some(&notes))]), &limits)
        .map(|_| ())
        .unwrap_err();
    assert_that!(err.kind()).is_equal_to(ImportErrorKind::TooLarge);
}

#[test]
fn minimal_snapshot_matches() {
    insta::assert_json_snapshot!("minimal", minimal());
}

#[test]
fn minimal_lists_its_font_families() {
    let fams: Vec<String> = minimal().fonts.into_iter().map(|f| f.family).collect();
    // Arial : police de la puce « • » du bodyStyle du master.
    assert_that!(fams).is_equal_to(vec!["Arial".to_owned(), "Calibri".to_owned(), "Calibri Light".to_owned()]);
}

/// `minimal` avec un second master (copie du premier) qui cite seul `slideLayout2`, rendu illisible.
fn second_master_with_unreadable_layout() -> SourceDeck {
    let pres = common::text("minimal/ppt/presentation.xml")
        .replace("</p:sldMasterIdLst>", r#"<p:sldMasterId id="2147483651" r:id="rId6"/></p:sldMasterIdLst>"#);
    let pres_rels = common::text("minimal/ppt/_rels/presentation.xml.rels").replace(
        "</Relationships>",
        r#"<Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster2.xml"/></Relationships>"#,
    );
    let rels1 = common::text("minimal/ppt/slideMasters/_rels/slideMaster1.xml.rels");
    let layout2 = rels1.lines().find(|l| l.contains("slideLayout2")).unwrap_or_else(|| panic!("layout 2 attendu"));
    let rels2 = rels1.lines().filter(|l| !l.contains("slideLayout1")).collect::<Vec<_>>().join("\n");
    let rels1 = rels1.replace(layout2, "");
    let master = common::text("minimal/ppt/slideMasters/slideMaster1.xml");
    let bytes = common::pptx_plus(
        "minimal",
        &[
            ("ppt/presentation.xml", Some(&pres)),
            ("ppt/_rels/presentation.xml.rels", Some(&pres_rels)),
            ("ppt/slideMasters/_rels/slideMaster1.xml.rels", Some(&rels1)),
            ("ppt/slideLayouts/slideLayout2.xml", Some("<p:sldLayout")),
        ],
        &[
            ("ppt/slideMasters/slideMaster2.xml", master.as_bytes()),
            ("ppt/slideMasters/_rels/slideMaster2.xml.rels", rels2.as_bytes()),
        ],
    );
    read_pptx(&bytes, &common::limits()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn slide_with_unreadable_layout_keeps_the_master_citing_that_layout() {
    let d = second_master_with_unreadable_layout();
    assert_that!(d.masters.len()).is_equal_to(2);
    let link = item(&d.slides[1], "Espace réservé du contenu 2").placeholder.clone();
    assert_that!(link.and_then(|l| l.master)).is_equal_to(Some("m2/3".to_owned()));
    assert_that!(unreadable_parts(&d)).contains("ppt/slides/slide2.xml: layout".to_owned());
}

#[test]
fn unreadable_master_relationships_warn_once_and_keep_the_master() {
    let d = minimal_with(&[("ppt/slideMasters/_rels/slideMaster1.xml.rels", Some("<Relationships"))])
        .unwrap_or_else(|e| panic!("{e}"));
    let master = unreadable_parts(&d).iter().filter(|p| *p == "ppt/slideMasters/slideMaster1.xml").count();
    assert_that!((d.masters.len(), master)).is_equal_to((1, 1));
}
