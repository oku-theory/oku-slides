//! Les packs intégrés, embarqués : `base` (widgets) et `essentiel` (layouts).
//!
//! Seule copie des sources de `typst/oku-slides/packs/{base,essentiel}` dans le
//! binaire : le rendu les rend comme tout pack (tests, `Renderer::with_builtins`),
//! la session en fait des packs intégrés de la bibliothèque.

use oku_slides_core::{Namespace, PackFiles, PackKind, PackManifest, WidgetFile, kinds};
use serde_json::{Map, Value};

use crate::error::{BuiltinPackSnafu, RegistryError};
use crate::{PackSet, PackWidgets};

/// Les fichiers `<dossier>/<nom>.typ` d'un pack intégré, par nom.
macro_rules! pack_files {
    ($dir:literal: $($name:literal),* $(,)?) => {
        &[$(($name, include_bytes!(concat!("../../../typst/oku-slides/packs/", $dir, "/", $name, ".typ")))),*]
    };
}

/// Les fichiers de widgets du pack `base`, par nom (`_commun` compris).
pub(crate) const BASE_WIDGETS: &[(&str, &[u8])] =
    pack_files!("base/widgets": "_commun", "text", "card", "image", "shape", "chart", "table", "stack");

/// Les fichiers de layouts du pack `essentiel`, par nom (`_commun` compris).
const ESSENTIEL_LAYOUTS: &[(&str, &[u8])] =
    pack_files!("essentiel/layouts": "_commun", "title", "toc", "chapter", "content", "end", "blank");

/// Un pack intégré : livré avec l'app, de confiance dans ses versions livrées.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinPack {
    /// Le pack de widgets `base` : texte, carte, image, forme, graphique, tableau, pile.
    Base,
    /// Le pack de layouts `essentiel` : couverture, sommaire, chapitre, contenu, fin, vide.
    Essentiel,
}

impl BuiltinPack {
    /// Les packs intégrés, dans l'ordre de déclaration.
    pub const ALL: [BuiltinPack; 2] = [BuiltinPack::Base, BuiltinPack::Essentiel];

    /// L'espace de noms du pack.
    pub fn namespace(self) -> Namespace {
        match self {
            BuiltinPack::Base => kinds::base::namespace(),
            BuiltinPack::Essentiel => kinds::essentiel::namespace(),
        }
    }

    /// La sorte du pack : widgets pour `base`, layouts pour `essentiel`.
    pub fn kind(self) -> PackKind {
        match self {
            BuiltinPack::Base => PackKind::Widgets,
            BuiltinPack::Essentiel => PackKind::Layouts,
        }
    }

    /// Le manifeste embarqué.
    fn manifest(self) -> &'static [u8] {
        match self {
            BuiltinPack::Base => include_bytes!("../../../typst/oku-slides/packs/base/widgets.json"),
            BuiltinPack::Essentiel => include_bytes!("../../../typst/oku-slides/packs/essentiel/layouts.json"),
        }
    }

    /// Les fichiers embarqués, par nom.
    fn sources(self) -> &'static [(&'static str, &'static [u8])] {
        match self {
            BuiltinPack::Base => BASE_WIDGETS,
            BuiltinPack::Essentiel => ESSENTIEL_LAYOUTS,
        }
    }

    /// Les versions livrées, une empreinte hex par ligne (`versions.txt`).
    fn versions(self) -> &'static str {
        match self {
            BuiltinPack::Base => include_str!("../../../typst/oku-slides/packs/base/versions.txt"),
            BuiltinPack::Essentiel => include_str!("../../../typst/oku-slides/packs/essentiel/versions.txt"),
        }
    }

    /// Le chemin d'un fichier du pack, pour les erreurs (`essentiel/layouts.json`).
    fn path(self, file: &str) -> String {
        format!("{}/{file}", self.namespace())
    }

    /// Le pack en fichiers de bibliothèque, lus tels qu'embarqués.
    pub fn files(self) -> Result<PackFiles, RegistryError> {
        let widgets = self
            .sources()
            .iter()
            .map(|(name, bytes)| WidgetFile::parse(name).map(|f| (f, *bytes)).ok_or(*name))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|file| RegistryError::from(BuiltinPackSnafu { file: self.path(file) }.build()))?;
        PackFiles::from_bytes(self.kind(), self.manifest(), widgets)
            .ok_or_else(|| BuiltinPackSnafu { file: self.path(self.kind().manifest_file()) }.build().into())
    }

    /// Le manifeste du pack, lu dans ses fichiers embarqués.
    fn read_manifest(self, files: &PackFiles) -> Result<PackManifest, RegistryError> {
        PackManifest::from_files(files).map_err(|_| self.broken_manifest())
    }

    /// Erreur d'un manifeste embarqué illisible.
    fn broken_manifest(self) -> RegistryError {
        BuiltinPackSnafu { file: self.path(self.kind().manifest_file()) }.build().into()
    }

    /// Le pack prêt à rendre : id = empreinte de sa version, espace de noms du manifeste.
    pub fn widgets(self) -> Result<PackWidgets, RegistryError> {
        let files = self.files()?;
        let manifest = self.read_manifest(&files)?;
        PackWidgets::new(files.version().hex(), manifest.namespace, self.kind(), files.widgets().clone())
    }

    /// Les défauts des jetons de composant du pack, relatifs à `component.<namespace>`.
    pub fn component_defaults(self) -> Result<Map<String, Value>, RegistryError> {
        self.read_manifest(&self.files()?)?.component_defaults().map_err(|_| self.broken_manifest())
    }

    /// Les versions livrées (lignes non vides de `versions.txt`).
    pub fn shipped_versions(self) -> impl Iterator<Item = &'static str> {
        self.versions().lines().map(str::trim).filter(|l| !l.is_empty())
    }
}

/// Les deux packs intégrés, `base` et `essentiel`, prêts à rendre.
pub fn builtin_pack_set() -> Result<PackSet, RegistryError> {
    PackSet::new(BuiltinPack::ALL.into_iter().map(BuiltinPack::widgets).collect::<Result<Vec<_>, _>>()?)
}

/// Les défauts de composant d'un pack (relatifs à `component.<namespace>`), avec son espace de noms.
type NamespaceDefaults = (Namespace, Map<String, Value>);

/// Les défauts de composant de chaque pack intégré, par espace de noms.
///
/// La première couche de `/theme.json` (`tokens::theme_layer`), dans l'ordre de [`BuiltinPack::ALL`].
pub fn builtin_component_defaults() -> Result<Vec<NamespaceDefaults>, RegistryError> {
    BuiltinPack::ALL.into_iter().map(|p| Ok((p.namespace(), p.component_defaults()?))).collect()
}

/// Vrai pour une version livrée d'un pack intégré : le repli ne l'écarte jamais.
///
/// L'id d'un pack rendu est l'empreinte de sa version : un pack intégré
/// modifié, ou un autre pack qui prendrait cet id, n'est pas de confiance.
pub fn is_trusted_pack(p: &PackWidgets) -> bool {
    BuiltinPack::ALL.into_iter().any(|b| b.namespace() == *p.namespace() && b.shipped_versions().any(|v| v == p.id()))
}

#[cfg(test)]
#[path = "builtin_packs_tests.rs"]
mod builtin_packs_tests;
