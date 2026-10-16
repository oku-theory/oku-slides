use std::cell::RefCell;

use speculoos::prelude::*;

use super::*;
use crate::collect::Sink;
use crate::pptx::test_support::TestPkg;
use crate::pptx::xml::parse;
use crate::source::{FaceStyle, WarningKind};

const GUID_PART: &str = "ppt/fonts/{2C7F3E6A-1B2D-4C5E-8F90-A1B2C3D4E5F6}.odttf";

fn sfnt() -> Vec<u8> {
    let mut v = vec![0x00, 0x01, 0x00, 0x00];
    v.extend(0u8..60);
    v
}

/// sfnt minimal avec une table `OS/2` dont `fsType` = `fs`.
fn sfnt_with_os2(fs: u16) -> Vec<u8> {
    face_at(0, 10, fs)
}

/// Face sfnt placée à `base` dans son fichier, avec une table `OS/2` de longueur déclarée `len`
/// (`fsType` = `fs`, écrit qu'elle le couvre ou non) ; les décalages de tables sont absolus.
fn face_at(base: u32, len: u32, fs: u16) -> Vec<u8> {
    let mut v = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
    v.extend_from_slice(b"OS/2");
    v.extend_from_slice(&[0, 0, 0, 0]); // somme de contrôle
    v.extend_from_slice(&(base + 28).to_be_bytes());
    v.extend_from_slice(&len.to_be_bytes());
    v.extend_from_slice(&[0; 8]);
    v.extend_from_slice(&fs.to_be_bytes());
    v
}

#[test]
fn weight_class_reads_the_os2_weight_within_100_and_900() {
    let with_weight = |w: u16| {
        let mut v = sfnt_with_os2(0);
        v[28 + 4..28 + 6].copy_from_slice(&w.to_be_bytes());
        v
    };
    assert_that!((weight_class(&with_weight(600)), weight_class(&with_weight(1000)), weight_class(&sfnt())))
        .is_equal_to((Some(600), Some(900), None));
}

#[test]
fn is_sfnt_recognizes_truetype_and_opentype_only() {
    assert_that!((is_sfnt(&sfnt()), is_sfnt(b"OTTO...."), is_sfnt(b"LP\x00\x00eot"))).is_equal_to((true, true, false));
}

#[test]
fn fs_type_reads_the_os2_embedding_rights() {
    assert_that!((fs_type(&sfnt_with_os2(0x0004)), fs_type(&sfnt()))).is_equal_to((Some(0x0004), None));
}

#[test]
fn fs_type_outside_a_too_short_os2_table_is_none() {
    // La table déclare 8 octets : `fsType` (octets 8 et 9) n'en fait pas partie, même s'ils suivent.
    assert_that!(fs_type(&face_at(0, 8, 0x0002))).is_none();
}

#[test]
fn fs_type_of_a_collection_is_read_in_its_first_face() {
    let mut ttc = b"ttcf\x00\x01\x00\x00".to_vec();
    ttc.extend_from_slice(&1u32.to_be_bytes());
    ttc.extend_from_slice(&16u32.to_be_bytes());
    ttc.extend(face_at(16, 10, 0x0002));
    assert_that!((fs_type(&ttc), weight_class(&ttc))).is_equal_to((Some(0x0002), Some(100)));
}

#[test]
fn deobfuscate_uses_the_reversed_guid_bytes_as_key() {
    // Clé : GUID à l'envers → F6 E5 D4 C3 B2 A1 90 8F 5E 4C 2D 1B 6A 3E 7F 2C, répétée.
    let mut data = sfnt();
    assert_that!(deobfuscate(&mut data, GUID_PART)).is_true();
    // octets 0..4 : 00 01 00 00 ⊕ F6 E5 D4 C3 ; octet 16 (= 12) ⊕ F6 (la clé recommence).
    assert_that!((data[..4].to_vec(), data[16])).is_equal_to((vec![0xF6, 0xE4, 0xD4, 0xC3], 12 ^ 0xF6));
}

#[test]
fn deobfuscate_only_touches_the_first_32_bytes() {
    let clear = sfnt();
    let mut data = clear.clone();
    deobfuscate(&mut data, GUID_PART);
    assert_that!(data[32..].to_vec()).is_equal_to(clear[32..].to_vec());
}

#[test]
fn deobfuscate_without_guid_is_false() {
    assert_that!(deobfuscate(&mut sfnt(), "ppt/fonts/font1.fntdata")).is_false();
}

const PRES: &str = r#"<p:presentation xmlns:p="urn:p" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" saveSubsetFonts="1"><p:embeddedFontLst>
<p:embeddedFont><p:font typeface="Inter"/><p:regular r:id="rId1"/><p:bold r:id="rId2"/><p:italic r:id="rId3"/><p:boldItalic r:id="rId9"/></p:embeddedFont>
<p:embeddedFont><p:font typeface="Inter Copy"/><p:regular r:id="rId1"/></p:embeddedFont>
</p:embeddedFontLst></p:presentation>"#;
const RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="fonts/font1.fntdata"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="fonts/font2.fntdata"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="fonts/font3.fntdata"/>
<Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="fonts/missing.fntdata"/>
</Relationships>"#;

fn eot_xor(font: &[u8], flags: u32) -> Vec<u8> {
    let mut v = vec![0u8; 82];
    v[0..4].copy_from_slice(&u32::try_from(82 + font.len()).unwrap().to_le_bytes());
    v[4..8].copy_from_slice(&u32::try_from(font.len()).unwrap().to_le_bytes());
    v[12..16].copy_from_slice(&flags.to_le_bytes());
    v[34..36].copy_from_slice(&0x504Cu16.to_le_bytes());
    v.extend(font.iter().map(|b| b ^ 0x50));
    v
}

#[test]
fn collect_reads_eot_and_plain_faces_and_reports_unreadable_and_missing_ones() {
    let regular = eot_xor(&sfnt_with_os2(0x0008), crate::pptx::eot::XOR);
    let compressed = eot_xor(&sfnt(), crate::pptx::eot::XOR | crate::pptx::eot::COMPRESSED);
    let t = TestPkg::with(&[
        ("ppt/_rels/presentation.xml.rels", RELS.as_bytes()),
        ("ppt/fonts/font1.fntdata", regular.as_slice()),
        ("ppt/fonts/font2.fntdata", sfnt().as_slice()),
        ("ppt/fonts/font3.fntdata", compressed.as_slice()),
    ]);
    let pkg = t.pkg();
    let rels = pkg.rels("ppt/presentation.xml").unwrap();
    let doc = parse(PRES, "p", 1000).unwrap();
    let sink = RefCell::new(Sink::new());
    let fonts = collect(doc.root_element(), &rels, &pkg, BTreeSet::from(["Calibri".to_owned()]), &sink).unwrap();
    assert_that!(fonts.iter().map(|f| f.family.as_str()).collect::<Vec<_>>()).is_equal_to(vec![
        "Calibri",
        "Inter",
        "Inter Copy",
    ]);
    let inter = &fonts[1];
    assert_that!(inter.faces.iter().map(|f| (f.style, f.fs_type, f.subset)).collect::<Vec<_>>())
        .is_equal_to(vec![(FaceStyle::Regular, Some(0x0008), true), (FaceStyle::Bold, None, true)]);
    let (_, w) = sink.into_inner().finish().unwrap();
    let kinds: Vec<_> = w.into_iter().map(|w| w.kind).collect();
    assert_that!(kinds.contains(&WarningKind::FontEmbeddedUnreadable("Inter".into()))).is_true();
    assert_that!(
        kinds.iter().filter(|k| matches!(k, WarningKind::MissingPart(p) if p.ends_with("missing.fntdata"))).count()
    )
    .is_equal_to(1);
}

#[test]
fn collect_counts_copies_of_a_shared_part_against_the_total_budget() {
    // `Inter` et `Inter Copy` partagent font1 : lue une fois (dans le budget), copiée deux fois (hors budget).
    let mut big = sfnt();
    big.resize(4000, 0);
    let t = TestPkg::with(&[
        ("ppt/_rels/presentation.xml.rels", RELS.as_bytes()),
        ("ppt/fonts/font1.fntdata", big.as_slice()),
    ]);
    let budget =
        (crate::pptx::test_support::CT.len() + crate::pptx::test_support::ROOT_RELS.len() + RELS.len() + 4100) as u64;
    let pkg =
        Package::open(t.bytes(), &crate::ReadLimits { max_total_bytes: budget, ..crate::pptx::test_support::LIMITS })
            .unwrap();
    let rels = pkg.rels("ppt/presentation.xml").unwrap();
    let doc = parse(PRES, "p", 1000).unwrap();
    let sink = RefCell::new(Sink::new());
    let r = collect(doc.root_element(), &rels, &pkg, BTreeSet::new(), &sink);
    assert_that!(r.map_err(|e| e.kind()).err()).is_equal_to(Some(ImportErrorKind::TooLarge));
}

#[test]
fn used_lists_east_asian_and_complex_families_of_runs() {
    use crate::source::{ItemKind, Styled};
    use crate::test_support::{item, para, run_with, source_slide, text_box};
    let r = run_with("x", |s| {
        s.font_ea = Some(Styled::explicit("Yu Gothic".into()));
        s.font_cs = Some(Styled::explicit("Traditional Arabic".into()));
    });
    let slide =
        source_slide(1, None, vec![item("s1/2", (0.0, 0.0, 1.0, 1.0), ItemKind::Text(text_box(vec![para(vec![r])])))]);
    let families: Vec<String> = used(&[], &[], &[slide]).into_iter().collect();
    assert_that!(families).is_equal_to(vec![
        "Calibri".to_owned(),
        "Traditional Arabic".to_owned(),
        "Yu Gothic".to_owned(),
    ]);
}
