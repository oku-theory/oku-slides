//! Le pack de widgets intégré `base`, embarqué : manifeste et fichiers de widgets.
//!
//! Seule copie des sources de `typst/oku-slides/packs/base` dans le binaire :
//! le rendu le rend seul (tests, `Renderer::with_builtins`), la session en fait
//! un pack intégré de la bibliothèque, à côté d'Essentiel et Grilles.

use oku_slides_core::{PackFiles, WidgetFile, kinds::base};

use crate::PackWidgets;
use crate::error::{BasePackSnafu, RegistryError};

macro_rules! base_widgets {
    ($($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../../../typst/oku-slides/packs/base/widgets/", $name, ".typ")))),*]
    };
}

/// Le manifeste du pack `base`.
pub(crate) const BASE_MANIFEST: &[u8] = include_bytes!("../../../typst/oku-slides/packs/base/widgets.json");

/// Les fichiers de widgets du pack `base`, par nom (`_commun` compris).
pub(crate) const BASE_WIDGETS: &[(&str, &[u8])] =
    base_widgets!("_commun", "text", "card", "image", "shape", "chart", "table", "stack");

/// Le pack `base` en fichiers de bibliothèque.
pub fn base_pack_files() -> Result<PackFiles, RegistryError> {
    let widgets = BASE_WIDGETS
        .iter()
        .map(|(name, bytes)| WidgetFile::parse(name).map(|f| (f, *bytes)).ok_or(*name))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|file| RegistryError::from(BasePackSnafu { file }.build()))?;
    PackFiles::from_bytes(BASE_MANIFEST, widgets).ok_or_else(|| BasePackSnafu { file: "widgets.json" }.build().into())
}

/// Le pack `base`, prêt à rendre (id = empreinte de sa version).
pub fn base_pack_widgets() -> Result<PackWidgets, RegistryError> {
    let files = base_pack_files()?;
    PackWidgets::new(files.version().hex(), base::namespace(), files.widgets().clone())
}

/// Les versions de `base` livrées par l'app, une empreinte hex par ligne.
const BASE_VERSIONS: &str = include_str!("../../../typst/oku-slides/packs/base/versions.txt");

/// Les versions livrées de `base` (lignes non vides de `versions.txt`).
pub(crate) fn shipped_base_versions() -> impl Iterator<Item = &'static str> {
    BASE_VERSIONS.lines().map(str::trim).filter(|l| !l.is_empty())
}

/// Vrai pour une version livrée du pack `base` : le repli ne l'écarte jamais.
///
/// L'id d'un pack rendu est l'empreinte de sa version : un `base` modifié,
/// ou un autre pack qui prendrait cet id, n'est pas de confiance.
pub fn is_trusted_pack(p: &PackWidgets) -> bool {
    p.namespace().as_str() == base::NAMESPACE && shipped_base_versions().any(|v| v == p.id())
}

#[cfg(test)]
#[path = "base_pack_tests.rs"]
mod base_pack_tests;
