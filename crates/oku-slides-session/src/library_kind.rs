//! Les sortes de versions que la bibliothèque range : modèles, puis thèmes.
//!
//! Une sorte donne son dossier, le nom du manifeste d'une version, et ses
//! versions intégrées ; la [`crate::Library`] fait le reste.

use std::sync::LazyLock;

use uuid::Uuid;

use oku_slides_core::{PackFiles, PackVersion};

use crate::pack::PackManifest;

/// Le manifeste d'une version, lu de ses fichiers.
pub trait VersionManifest: Sized + Clone + std::fmt::Debug + 'static {
    /// Lit le manifeste de `files`.
    fn from_files(files: &PackFiles) -> Result<Self, serde_json::Error>;
    fn id(&self) -> Uuid;
    fn name(&self) -> &str;
    /// Version dont celle-ci est issue ; `None` pour une première version.
    fn parent(&self) -> Option<&PackVersion>;
    /// Création, UTC à la seconde : `AAAA-MM-JJTHH:MM:SSZ`.
    fn created(&self) -> &str;
}

/// Une sorte de version rangée dans la bibliothèque.
pub trait LibraryKind: std::fmt::Debug + Clone + 'static {
    /// Dossier sous `<data>` et nom du manifeste dans une version.
    const DIR: &'static str;
    const MANIFEST_FILE: &'static str;
    /// Les sortes à fichiers de widgets les lisent (`widgets/<nom>.typ`).
    const HAS_WIDGETS: bool;
    type Manifest: VersionManifest;
    /// Versions intégrées à l'app, dans l'ordre.
    fn builtins() -> &'static [StoredVersion<Self::Manifest>];
}

/// Une version lue et vérifiée : empreinte, fichiers, manifeste lu.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredVersion<M> {
    pub version: PackVersion,
    pub files: PackFiles,
    pub manifest: M,
}

impl<M: VersionManifest> StoredVersion<M> {
    /// `files` lus ; sinon l'erreur, sur une ligne.
    pub(crate) fn read(files: PackFiles) -> Result<Self, String> {
        let manifest = M::from_files(&files).map_err(|e| e.to_string())?;
        Ok(Self { version: files.version(), files, manifest })
    }
}

/// Les packs de modèles.
#[derive(Debug, Clone)]
pub struct Templates;

/// Packs intégrés, lus une fois par processus, dans l'ordre.
static BUILTIN_PACKS: LazyLock<Vec<StoredVersion<PackManifest>>> = LazyLock::new(|| {
    [include_str!("../../../templates/essentiel/pack.json"), include_str!("../../../templates/grilles/pack.json")]
        .iter()
        .filter_map(|json| {
            let read = PackFiles::from_bytes(json.as_bytes(), [])
                .ok_or_else(|| "pack.json is not a json object".to_owned())
                .and_then(StoredVersion::read);
            // Couvert par les tests : n'arrive qu'avec un fichier du dépôt cassé.
            read.inspect_err(|e| tracing::error!(error = %e, "pack intégré illisible")).ok()
        })
        .collect()
});

impl LibraryKind for Templates {
    const DIR: &'static str = "templates";
    const MANIFEST_FILE: &'static str = "pack.json";
    const HAS_WIDGETS: bool = true;
    type Manifest = PackManifest;

    fn builtins() -> &'static [StoredVersion<PackManifest>] {
        &BUILTIN_PACKS
    }
}
