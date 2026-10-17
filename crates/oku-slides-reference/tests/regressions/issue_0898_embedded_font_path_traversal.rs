//! Une police embarquée hors de `ppt/fonts/` laissait `office2pdf` écrire où
//! le `typeface` de `<p:embeddedFont>` le voulait (`../` ou chemin absolu).
//!
//! `office2pdf` 0.8.1 résout la police par la relation de `presentation.xml`
//! (n'importe quelle part nommée `{GUID}.fntdata`), puis l'écrit en
//! `temp_dir.join("{typeface}-{style}.{ext}")` sans rien refuser.
//!
//! `office2pdf` lit l'archive avec `zip` 0.6, qui ignore le nom Unicode
//! (extra field 0x7075) que `zip` 9 préfère : une part peut s'appeler
//! `image.png` pour le filtre et `{GUID}.fntdata` pour `office2pdf`.

use std::io::{Cursor, Write};
use std::path::Path;

use oku_slides_reference::{Office2Pdf, ReferenceRenderer};
use rstest::rstest;
use speculoos::prelude::*;
use zip::write::SimpleFileOptions;

/// Le GUID qui nomme la part de la police (sa clé de désobfuscation).
const GUID: &str = "{01234567-89AB-CDEF-0123-456789ABCDEF}";

/// Un `.pptx` minimal dont l'unique police embarquée vit dans `font_part`,
/// sous un autre nom Unicode quand `unicode_name` en donne un.
fn pptx_with_font(typeface: &str, font_part: &str, unicode_name: Option<&str>) -> Vec<u8> {
    let presentation = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldSz cx="12192000" cy="6858000"/><p:embeddedFontLst><p:embeddedFont><p:font typeface="{typeface}"/><p:regular r:id="rId9"/></p:embeddedFont></p:embeddedFontLst></p:presentation>"#
    );
    let rels = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId9" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/font" Target="/{font_part}"/></Relationships>"#
    );
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in
        [("ppt/presentation.xml", presentation.as_bytes()), ("ppt/_rels/presentation.xml.rels", rels.as_bytes())]
    {
        out.start_file(name, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("{e}"));
        out.write_all(bytes).unwrap_or_else(|e| panic!("{e}"));
    }
    let field = unicode_name.map(|unicode_name| unicode_path(font_part, unicode_name));
    let mut options = SimpleFileOptions::default().into_full_options();
    if let Some(field) = &field {
        options.add_extra_field(PLACEHOLDER, field, false).unwrap_or_else(|e| panic!("{e}"));
    }
    out.start_file(font_part, options).unwrap_or_else(|e| panic!("{e}"));
    out.write_all(b"pas une police").unwrap_or_else(|e| panic!("{e}"));
    let mut bytes = out.finish().unwrap_or_else(|e| panic!("{e}")).into_inner();
    if let Some(field) = &field {
        retag(&mut bytes, field);
    }
    bytes
}

/// L'identifiant de l'extra field Info-ZIP « Unicode Path ».
const UNICODE_PATH: u16 = 0x7075;

/// Un identifiant libre (« ok ») : `zip` 9 refuse d'écrire un « Unicode Path », dont il
/// vérifie le CRC avant de connaître le nom ; il est retouché après coup.
const PLACEHOLDER: u16 = 0x6F6B;

/// Renomme en « Unicode Path » chaque extra field `PLACEHOLDER` portant `field`
/// (en-tête local et répertoire central).
fn retag(zip: &mut [u8], field: &[u8]) {
    let len = u16::try_from(field.len()).unwrap_or_else(|e| panic!("{e}"));
    let needle: Vec<u8> = [&PLACEHOLDER.to_le_bytes()[..], &len.to_le_bytes(), field].concat();
    let starts: Vec<usize> =
        zip.windows(needle.len()).enumerate().filter(|(_, w)| *w == needle).map(|(i, _)| i).collect();
    assert_that!(starts).has_length(2);
    for start in starts {
        zip[start..start + 2].copy_from_slice(&UNICODE_PATH.to_le_bytes());
    }
}

/// Le contenu d'un extra field « Unicode Path » : version 1, CRC32 du nom de
/// l'en-tête, puis le nom Unicode qui le remplace.
fn unicode_path(header_name: &str, unicode_name: &str) -> Vec<u8> {
    let mut field = vec![1];
    field.extend_from_slice(&crc32(header_name.as_bytes()).to_le_bytes());
    field.extend_from_slice(unicode_name.as_bytes());
    field
}

/// CRC-32 (IEEE), bit à bit.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// Le `typeface` qui vise `target` : absolu, ou relatif par assez de `../`
/// pour remonter du dossier temporaire d'`office2pdf` jusqu'à la racine.
fn typeface_towards(target: &Path, relative: bool) -> String {
    let target = target.to_str().unwrap_or_else(|| panic!("chemin non UTF-8"));
    if relative { format!("{}{}", "../".repeat(64), target.trim_start_matches('/')) } else { target.to_owned() }
}

/// Les fichiers écrits dans `dir`.
fn written(dir: &Path) -> Vec<std::ffi::OsString> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{e}"))
        .map(|e| e.unwrap_or_else(|e| panic!("{e}")).file_name())
        .collect()
}

/// Témoin positif : sans la garde, `office2pdf` écrit bien la police au chemin
/// du `typeface`. S'il échoue, `office2pdf` a changé : revoir `without_fonts`.
#[test]
fn convert_bytes_without_guard_writes_at_the_typeface_path() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().canonicalize().unwrap().join("evil");
    let pptx = pptx_with_font(&typeface_towards(&target, false), &format!("ppt/media/{GUID}.fntdata"), None);

    let _ = office2pdf::convert_bytes(&pptx, office2pdf::config::Format::Pptx, &Default::default());

    assert_that!(written(dir.path())).contains(std::ffi::OsString::from("evil-regular.ttf"));
}

/// Prémisse du cas Unicode : `zip` 9 lit la police sous son nom Unicode.
#[test]
fn unicode_path_hides_the_font_from_zip_9() {
    let pptx = pptx_with_font("x", &format!("ppt/media/{GUID}.fntdata"), Some("ppt/media/image1.png"));
    let zip = zip::ZipArchive::new(Cursor::new(pptx)).unwrap();
    let names: Vec<String> = zip.file_names().map(|n| n.unwrap().into_owned()).collect();
    assert_that!(names).contains("ppt/media/image1.png".to_owned());
}

#[rstest]
#[case::relative_typeface(true, None)]
#[case::absolute_typeface(false, None)]
#[case::under_a_unicode_name(true, Some("ppt/media/image1.png"))]
fn render_of_a_hostile_embedded_font_writes_nothing_at_its_typeface_path(
    #[case] relative: bool,
    #[case] unicode_name: Option<&str>,
) {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().canonicalize().unwrap().join("evil");
    let font_part = format!("ppt/media/{GUID}.fntdata");
    let pptx = pptx_with_font(&typeface_towards(&target, relative), &font_part, unicode_name);

    // Sans slide, le rendu peut réussir ou échouer : seul compte ce qu'il écrit.
    let _rendered = Office2Pdf.render(&pptx, vec![], None);

    assert_that!(written(dir.path())).is_empty();
}
