//! Paquets et contextes de test en mémoire : seule source des aides de test du lecteur.

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use zip::write::SimpleFileOptions;

use crate::ReadLimits;
use crate::error::ImportError;
use crate::pptx::context::{PartCtx, PartSpec, Shared};
use crate::pptx::package::Package;
use crate::pptx::text::empty_levels;
use crate::pptx::theme::ThemeData;
use crate::source::{Asset, Warning};

pub(crate) const LIMITS: ReadLimits = ReadLimits {
    max_input_bytes: 64 << 20,
    max_part_bytes: 1 << 20,
    max_total_bytes: 8 << 20,
    max_parts: 1000,
    max_xml_nodes: 1_000_000,
    max_slides: 1000,
    max_output_bytes: 8 << 20,
    max_replica_bytes: 8 << 20,
};

pub(crate) const THEME: &str = include_str!("../../../../fixtures/import/minimal/ppt/theme/theme1.xml");
pub(crate) const CT: &str = r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/></Types>"#;
pub(crate) const ROOT_RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#;

/// Zip des parts données, dans l'ordre.
pub(crate) fn zip_of(parts: &[(&str, &[u8])]) -> Vec<u8> {
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in parts {
        w.start_file(*name, SimpleFileOptions::default()).unwrap();
        w.write_all(data).unwrap();
    }
    w.finish().unwrap().into_inner()
}

/// Octets d'un paquet de test ; `pkg()` l'ouvre (le `Package` emprunte ces octets).
#[derive(Debug)]
pub(crate) struct TestPkg(Vec<u8>);

impl TestPkg {
    /// Présentation minimale (types, relation racine, `presentation.xml` vide) plus `parts`.
    pub(crate) fn with(parts: &[(&str, &[u8])]) -> Self {
        let mut all: Vec<(&str, &[u8])> = vec![
            ("[Content_Types].xml", CT.as_bytes()),
            ("_rels/.rels", ROOT_RELS.as_bytes()),
            ("ppt/presentation.xml", b"<p/>"),
        ];
        all.extend_from_slice(parts);
        Self(zip_of(&all))
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn pkg(&self) -> Package<'_> {
        Package::open(&self.0, &LIMITS).unwrap()
    }
}

/// Graphique à barres empilées de `kitchen` (plus un type de combinaison).
pub(crate) const BAR: &str = include_str!("../../../../fixtures/import/kitchen/ppt/charts/chart1.xml");

/// Image PNG factice.
pub(crate) const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-fake";

/// Résultat de `f`, avertissements et assets d'un contexte de test.
pub(crate) type CtxOut<R> = (R, Vec<Warning>, BTreeMap<String, Asset>);

/// Exécute `f` dans le contexte de la slide 1 de `pkg` (avec `tableStyles.xml` s'il est donné) ; rend aussi avertissements et assets, ou l'erreur fatale notée.
pub(crate) fn run_ctx<R>(
    pkg: &Package<'_>,
    part: &str,
    table_styles: Option<&str>,
    f: impl FnOnce(&PartCtx<'_>) -> R,
) -> Result<CtxOut<R>, ImportError> {
    let theme = ThemeData::parse(THEME, "ppt/theme/theme1.xml", LIMITS.max_xml_nodes).unwrap();
    let styles = table_styles.map(|s| crate::pptx::xml::parse(s, "ppt/tableStyles.xml", LIMITS.max_xml_nodes).unwrap());
    let shared = Shared::new(pkg, styles, empty_levels());
    let r = {
        let spec =
            PartSpec { owner: "s1".into(), slide: Some(1), colors: theme.default_ctx(), theme: &theme, inherit: None };
        let cx = PartCtx::new(&shared, part, spec).unwrap();
        f(&cx)
    };
    let (assets, warnings) = shared.sink.into_inner().finish()?;
    Ok((r, warnings, assets))
}

/// `run_ctx` sans styles de tableau, qui panique sur une erreur fatale.
pub(crate) fn with_ctx<R>(pkg: &Package<'_>, part: &str, f: impl FnOnce(&PartCtx<'_>) -> R) -> CtxOut<R> {
    run_ctx(pkg, part, None, f).unwrap()
}

/// `run_ctx` avec `tableStyles.xml` sur la slide 1 (tâche 12).
pub(crate) fn with_table_styles<R>(
    pkg: &Package<'_>,
    styles: &str,
    f: impl FnOnce(&PartCtx<'_>) -> R,
) -> (R, Vec<Warning>) {
    let (r, w, _) = run_ctx(pkg, "ppt/slides/slide1.xml", Some(styles), f).unwrap();
    (r, w)
}

pub(crate) const DATA: &str = include_str!("../../../../fixtures/import/kitchen/ppt/diagrams/data1.xml");
pub(crate) const DRAWING: &str = include_str!("../../../../fixtures/import/kitchen/ppt/diagrams/drawing1.xml");

/// Relations de `ppt/slides/slide1.xml` des tests d'arbre de formes.
pub(crate) const SLIDE_RELS: &str = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
<Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/missing.png"/>
<Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.svg"/>
<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
<Relationship Id="rId6" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/diagramData" Target="../diagrams/data1.xml"/>
<Relationship Id="rId7" Type="http://schemas.microsoft.com/office/2007/relationships/diagramDrawing" Target="../diagrams/drawing1.xml"/>
<Relationship Id="rId8" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="https://images.example/photo.png" TargetMode="External"/>
</Relationships>"#;

/// Paquet d'une slide de test : `SLIDE_RELS`, image PNG et SVG, graphique, SmartArt, plus `extra`.
pub(crate) fn slide_pkg(extra: &[(&str, &[u8])]) -> TestPkg {
    let mut parts: Vec<(&str, &[u8])> = vec![
        ("ppt/slides/_rels/slide1.xml.rels", SLIDE_RELS.as_bytes()),
        ("ppt/media/image1.png", PNG),
        ("ppt/media/image1.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
        ("ppt/charts/chart1.xml", BAR.as_bytes()),
        ("ppt/diagrams/data1.xml", DATA.as_bytes()),
        ("ppt/diagrams/drawing1.xml", DRAWING.as_bytes()),
    ];
    parts.extend_from_slice(extra);
    TestPkg::with(&parts)
}

/// Lit `inner` comme un `spTree` de la slide 1 de `slide_pkg(&[])`.
pub(crate) fn tree(inner: &str) -> (Vec<crate::pptx::shapes::ParsedItem>, Vec<Warning>, BTreeMap<String, Asset>) {
    tree_in(&slide_pkg(&[]), inner)
}

/// Comme `tree`, dans un paquet donné.
pub(crate) fn tree_in(
    t: &TestPkg,
    inner: &str,
) -> (Vec<crate::pptx::shapes::ParsedItem>, Vec<Warning>, BTreeMap<String, Asset>) {
    let xml = format!(
        r#"<p:spTree xmlns:p="urn:p" xmlns:a="urn:a" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:mc="urn:mc" xmlns:m="urn:m" xmlns:asvg="urn:asvg"><p:nvGrpSpPr/><p:grpSpPr/>{inner}</p:spTree>"#
    );
    let doc = crate::pptx::xml::parse(&xml, "t", LIMITS.max_xml_nodes).unwrap();
    let pkg = t.pkg();
    with_ctx(&pkg, "ppt/slides/slide1.xml", |cx| {
        crate::pptx::shapes::parse_tree(doc.root_element(), cx, &crate::pptx::shapes::GroupCtx::ROOT)
    })
}

pub(crate) const MASTER: &str = include_str!("../../../../fixtures/import/minimal/ppt/slideMasters/slideMaster1.xml");
pub(crate) const LAYOUT: &str = include_str!("../../../../fixtures/import/minimal/ppt/slideLayouts/slideLayout1.xml");

/// Master de `minimal` et le layout donné, lus avec le thème `THEME`.
pub(crate) fn master_and_layout(
    layout_xml: &str,
) -> (crate::pptx::master::MasterInfo, crate::pptx::master::LayoutInfo) {
    const M: &str = "ppt/slideMasters/slideMaster1.xml";
    const L: &str = "ppt/slideLayouts/slideLayout1.xml";
    let t = TestPkg::with(&[(M, MASTER.as_bytes()), (L, layout_xml.as_bytes())]);
    let pkg = t.pkg();
    let theme = ThemeData::parse(THEME, "theme", LIMITS.max_xml_nodes).unwrap();
    let shared = Shared::new(&pkg, None, empty_levels());
    let m = crate::pptx::master::parse_master(&shared, M, 1, 0, &theme).unwrap();
    let l = crate::pptx::master::parse_layout(&shared, L, 1, &m, &theme).unwrap();
    shared.sink.into_inner().finish().unwrap();
    (m, l)
}

pub(crate) const TABLE_STYLES: &str = include_str!("../../../../fixtures/import/kitchen/ppt/tableStyles.xml");
