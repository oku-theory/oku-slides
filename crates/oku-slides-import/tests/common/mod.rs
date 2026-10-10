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
