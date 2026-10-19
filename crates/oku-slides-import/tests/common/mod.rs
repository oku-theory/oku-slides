//! Fixtures d'import : dossiers OOXML décompressés, zippés à la volée, et fichiers binaires.

#![allow(dead_code, reason = "module partagé : chaque binaire de test n'en utilise qu'une partie")]

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use oku_slides_import::ReadLimits;
use zip::write::SimpleFileOptions;

/// Bornes des tests de bout en bout : celles de [`ReadLimits::default`] (une borne ajoutée plus tard suit).
pub fn limits() -> ReadLimits {
    ReadLimits::default()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("lecture de {}: {e}", dir.display()));
    for e in entries {
        let p = e.unwrap_or_else(|e| panic!("entrée de {}: {e}", dir.display())).path();
        // `.DS_Store` (Finder) seulement : `_rels/.rels` commence aussi par un point.
        if p.file_name().is_some_and(|n| n == ".DS_Store") {
            continue;
        }
        if p.is_dir() { walk(&p, out) } else { out.push(p) }
    }
}

/// Le `.pptx` de la fixture `name` (un dossier), avec des parts remplacées (`Some`) ou retirées (`None`).
pub fn pptx_with(name: &str, overrides: &[(&str, Option<&str>)]) -> Vec<u8> {
    pptx_plus(name, overrides, &[])
}

/// Comme [`pptx_with`], plus des parts binaires ajoutées à la fin (`added`).
pub fn pptx_plus(name: &str, overrides: &[(&str, Option<&str>)], added: &[(&str, &[u8])]) -> Vec<u8> {
    let dir = root().join(name);
    let mut files = Vec::new();
    walk(&dir, &mut files);
    files.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for f in files {
        let rel = f.strip_prefix(&dir).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        let data = match overrides.iter().find(|(p, _)| *p == rel) {
            Some((_, None)) => continue,
            Some((_, Some(text))) => text.as_bytes().to_vec(),
            None => fs::read(&f).unwrap_or_else(|e| panic!("lecture de {}: {e}", f.display())),
        };
        w.start_file(rel, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip: {e}"));
        w.write_all(&data).unwrap_or_else(|e| panic!("zip: {e}"));
    }
    for (rel, data) in added {
        w.start_file(*rel, SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip: {e}"));
        w.write_all(data).unwrap_or_else(|e| panic!("zip: {e}"));
    }
    w.finish().unwrap_or_else(|e| panic!("zip: {e}")).into_inner()
}

/// Le `.pptx` de la fixture `name`.
pub fn pptx(name: &str) -> Vec<u8> {
    pptx_with(name, &[])
}

/// Un fichier binaire de `fixtures/import/`.
pub fn file(name: &str) -> Vec<u8> {
    fs::read(root().join(name)).unwrap_or_else(|e| panic!("lecture de {name}: {e}"))
}

/// Texte d'un fichier de `fixtures/import/` (une part d'une fixture décompressée, par exemple).
pub fn text(name: &str) -> String {
    fs::read_to_string(root().join(name)).unwrap_or_else(|e| panic!("lecture de {name}: {e}"))
}

/// Le registre des widgets de base (`fixtures/widgets/base-registry.json`).
pub fn registry() -> oku_slides_core::WidgetRegistry {
    let text = fs::read_to_string(root().join("../widgets/base-registry.json")).unwrap_or_else(|e| panic!("{e}"));
    oku_slides_core::WidgetRegistry::from_probe(serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// L'épinglage de `base` dans les tests : l'id du pack de la fixture, une version quelconque.
pub fn base_pin() -> oku_slides_core::PackPin {
    oku_slides_core::PackPin {
        id: "3b1f6c2e-5a4d-4e8b-9c71-0d2e4f6a8b13".parse().unwrap_or_else(|e| panic!("{e}")),
        version: oku_slides_core::PackVersion::from_hex(&"0".repeat(64)).unwrap_or_else(|| panic!("version")),
        requires: vec![],
    }
}

/// Les sortes des éléments de `list` et de leurs descendants.
fn collect_kinds(list: &[oku_slides_core::Element], out: &mut Vec<String>) {
    for e in list {
        out.push(e.kind.clone());
        for (_, children) in e.params.slots() {
            collect_kinds(children, out);
        }
    }
}

/// Les sortes de tous les éléments des slides de `deck`, descendants compris.
pub fn deck_kinds(deck: &oku_slides_core::Deck) -> Vec<String> {
    let mut out = Vec::new();
    for s in &deck.slides {
        for (_, list) in s.params.slots() {
            collect_kinds(list, &mut out);
        }
    }
    out
}

/// Les modèles du pack intégré `name` (`templates/<name>/pack.json`), dans l'ordre du pack.
pub fn templates(name: &str) -> Vec<oku_slides_core::Template> {
    let path = root().join(format!("../../templates/{name}/pack.json"));
    let pack: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
            .unwrap_or_else(|e| panic!("{e}"));
    serde_json::from_value(pack["templates"].clone()).unwrap_or_else(|e| panic!("{e}"))
}

/// Les jetons effectifs du thème intégré `name` (`papier`, `pro`) : les défauts, puis les siens.
pub fn tokens(name: &str) -> serde_json::Map<String, serde_json::Value> {
    let typst = root().join("../../typst/oku-slides");
    let read = |p: PathBuf| -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
            .unwrap_or_else(|e| panic!("{e}"))
    };
    let serde_json::Value::Object(defaults) = read(typst.join("src/default-tokens.json")) else { panic!("jetons") };
    let theme = read(typst.join(format!("themes/{name}.json")));
    oku_slides_core::tokens::effective_tokens(
        &defaults,
        theme["tokens"].as_object().unwrap_or_else(|| panic!("tokens de {name}")),
    )
}

/// Les polices livrées avec l'app (`assets/fonts/*.ttf`), sans police système.
pub fn app_fonts() -> oku_slides_import::fonts::FontCatalog {
    let dir = root().join("../../assets/fonts");
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap_or_else(|e| panic!("{e}")).path())
        .filter(|p| p.extension().is_some_and(|x| x == "ttf"))
        .collect();
    paths.sort();
    let mut catalog = oku_slides_import::fonts::FontCatalog::new();
    for p in paths {
        catalog.add(oku_slides_import::fonts::FontOrigin::App, fs::read(&p).unwrap_or_else(|e| panic!("{e}")));
    }
    catalog
}

/// Le `.pptx` de `minimal` dont la slide 1 reçoit `shapes` en fin d'arbre et `rels` en fin de relations.
pub fn minimal_slide1(shapes: &str, rels: &str) -> Vec<u8> {
    minimal_slide1_plus(shapes, rels, &[])
}

/// Comme [`minimal_slide1`], plus des parts binaires ajoutées (`added`).
pub fn minimal_slide1_plus(shapes: &str, rels: &str, added: &[(&str, &[u8])]) -> Vec<u8> {
    let slide = text("minimal/ppt/slides/slide1.xml").replace("</p:spTree>", &format!("{shapes}</p:spTree>"));
    let all_rels = text("minimal/ppt/slides/_rels/slide1.xml.rels")
        .replace("</Relationships>", &format!("{rels}</Relationships>"));
    pptx_plus(
        "minimal",
        &[("ppt/slides/slide1.xml", Some(&slide)), ("ppt/slides/_rels/slide1.xml.rels", Some(&all_rels))],
        added,
    )
}

/// Sorte d'un item (`text`, `group[shape,text]`, `opaque(Media)`…), libellé des résumés de deck.
pub fn kind(i: &oku_slides_import::source::Item) -> String {
    use oku_slides_import::source::ItemKind;
    match &i.kind {
        ItemKind::Text(_) => "text".into(),
        ItemKind::Shape(_) => "shape".into(),
        ItemKind::Picture(_) => "picture".into(),
        ItemKind::Table(_) => "table".into(),
        ItemKind::Chart(_) => "chart".into(),
        ItemKind::Group { items, .. } => format!("group[{}]", items.iter().map(kind).collect::<Vec<_>>().join(",")),
        ItemKind::SmartArt(sa) => format!("smartart[{}]", sa.nodes.len()),
        ItemKind::Opaque(o) => format!("opaque({:?})", o.reason),
    }
}

/// Une ligne par slide (sortes de ses items) puis une par avertissement.
pub fn summary(d: &oku_slides_import::source::SourceDeck) -> Vec<String> {
    let mut out: Vec<String> = d
        .slides
        .iter()
        .map(|s| format!("{}: {}", s.id, s.items.iter().map(kind).collect::<Vec<_>>().join(" ")))
        .collect();
    out.extend(d.warnings.iter().map(|w| format!("warning: {:?}", w.kind)));
    out
}
