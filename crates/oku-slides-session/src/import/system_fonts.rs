//! Polices du système pour le catalogue de l'import hi-fi.
//!
//! `fontdb` les énumère (noms seuls, fichiers projetés en mémoire) ; seuls
//! les fichiers des familles cherchées sont copiés dans le catalogue : un
//! système en compte des centaines.

use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use fontdb::{Database, Source};
use oku_slides_import::fonts::{FontCatalog, FontOrigin};

/// Ajoute à `catalog` les fichiers de police du système dont une face est de
/// l'une des `families` (en minuscules, voir [`matches`]) ; rend le nombre de
/// faces ajoutées. Un fichier (une collection) n'est lu qu'une fois.
pub(crate) fn add(catalog: &mut FontCatalog, families: &BTreeSet<String>) -> usize {
    if families.is_empty() {
        return 0;
    }
    let mut db = Database::new();
    db.load_system_fonts();
    let mut read: HashSet<PathBuf> = HashSet::new();
    let mut added = 0;
    for face in db.faces() {
        if !face.families.iter().any(|(name, _)| matches(families, name)) {
            continue;
        }
        let path = match &face.source {
            Source::File(path) | Source::SharedFile(path, _) => Some(path.clone()),
            Source::Binary(_) => None,
        };
        if path.is_some_and(|p| !read.insert(p)) {
            continue;
        }
        if let Some(data) = db.with_face_data(face.id, |data, _| data.to_vec()) {
            added += catalog.add(FontOrigin::System, data);
        }
    }
    tracing::debug!(families = families.len(), faces = added, "polices du système pour l'import");
    added
}

/// Le nom de famille `name` (tel que `fontdb` le donne) répond-il à l'une des
/// `families` ? Égal, casse ignorée, ou son préfixe suivi d'une espace :
/// `fontdb` donne la famille typographique (« Calibri ») d'une face que la
/// présentation nomme par sa famille de style (« Calibri Light »).
fn matches(families: &BTreeSet<String>, name: &str) -> bool {
    let name = name.trim().to_lowercase();
    !name.is_empty()
        && families
            .iter()
            .any(|f| f == &name || f.strip_prefix(name.as_str()).is_some_and(|rest| rest.starts_with(' ')))
}

#[cfg(test)]
#[path = "system_fonts_tests.rs"]
mod system_fonts_tests;
