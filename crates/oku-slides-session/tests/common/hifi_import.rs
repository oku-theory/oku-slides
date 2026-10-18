//! L'import hi-fi de `fixtures/import/minimal`, une aide des régressions de l'import (t-902, t-903).

use std::io::Write;
use std::path::{Path, PathBuf};

use oku_slides_session::import::{ImportRequest, ImportTarget, PackLookup, import};
use oku_slides_session::{Fonts, Platform, Session, Settings};

/// Le `.pptx` de `fixtures/import/minimal`, zippé dans `dir`.
fn minimal_pptx(dir: &Path) -> PathBuf {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let p = e.unwrap_or_else(|e| panic!("{e}")).path();
            if p.file_name().is_some_and(|n| n == ".DS_Store") {
                continue;
            }
            if p.is_dir() { walk(&p, out) } else { out.push(p) }
        }
    }
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/minimal"));
    let mut paths = Vec::new();
    walk(&root, &mut paths);
    paths.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for p in paths {
        let rel = p.strip_prefix(&root).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        w.start_file(rel, zip::write::SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip : {e}"));
        w.write_all(&std::fs::read(&p).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("zip : {e}"));
    }
    let bytes = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    let path = dir.join("minimal.pptx");
    std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("{e}"));
    path
}

/// Les fichiers de la bibliothèque `dir` (`templates`, `themes`) sous `data`, avec leurs octets, triés.
pub(crate) fn library_files(data: &Path, dir: &str) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries {
            let p = e.unwrap_or_else(|e| panic!("{e}")).path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                out.push((p, bytes));
            }
        }
    }
    let mut out = Vec::new();
    walk(&data.join(dir), &mut out);
    out.sort();
    out
}

/// Une session sur `data`, chargée de l'import hi-fi de `minimal`.
pub(crate) fn imported_hifi(data: &Path) -> Session {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    let request = ImportRequest {
        pptx: minimal_pptx(data),
        target: ImportTarget::Hifi { witness: Some(oku_slides_platform::config::ImportWitness::Office2pdf) },
        system_fonts: false,
        packs: PackLookup::Library,
    };
    let imported = import(&platform, &Fonts::default(), &request).unwrap_or_else(|e| panic!("import : {e}"));
    let mut s = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {}", e.one_line()));
    s.load_import(imported, "minimal").unwrap_or_else(|e| panic!("chargement : {}", e.one_line()));
    s
}
