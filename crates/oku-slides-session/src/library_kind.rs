//! Les sortes de versions que la bibliothèque range : packs de widgets, puis thèmes.
//!
//! Une sorte donne son dossier, le nom du manifeste d'une version, et ses
//! versions intégrées ; la [`crate::Library`] fait le reste.

use std::sync::LazyLock;

use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackScope, PackVersion};

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
    /// Portée : une version du deck seul n'entre jamais dans la bibliothèque.
    fn scope(&self) -> PackScope;
    /// Espace de noms, unique dans la bibliothèque ; `None` pour une sorte qui n'en a pas.
    fn namespace(&self) -> Option<&Namespace> {
        None
    }
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
    /// Arrivée avec un deck reçu, sans que l'utilisateur l'ait créée.
    pub received: bool,
}

impl<M: VersionManifest> StoredVersion<M> {
    /// `files` lus, d'une version créée ici ; sinon l'erreur, sur une ligne.
    pub(crate) fn read(files: PackFiles) -> Result<Self, String> {
        let manifest = M::from_files(&files).map_err(|e| e.to_string())?;
        Ok(Self { version: files.version(), files, manifest, received: false })
    }
}

/// Les packs de widgets (et leurs modèles).
#[derive(Debug, Clone)]
pub struct Templates;

/// Packs intégrés, lus une fois par processus, dans l'ordre : Essentiel, Grilles, puis `base`.
static BUILTIN_PACKS: LazyLock<Vec<StoredVersion<PackManifest>>> = LazyLock::new(|| {
    // Les fichiers du dépôt gardent leur nom `pack.json` jusqu'au lot 3 ; la bibliothèque écrit `widgets.json`.
    let templates =
        [include_str!("../../../templates/essentiel/pack.json"), include_str!("../../../templates/grilles/pack.json")]
            .map(|json| {
                PackFiles::from_bytes(json.as_bytes(), []).ok_or_else(|| "widgets.json is not a json object".to_owned())
            });
    // `base` : ses sources sont celles du rendu, qui les embarque.
    let base = oku_slides_render::base_pack_files().map_err(|e| oku_slides_platform::errors::one_line(&e));
    templates
        .into_iter()
        .chain([base])
        .filter_map(|files| {
            let read = files.and_then(StoredVersion::read);
            // Couvert par les tests : n'arrive qu'avec un fichier du dépôt cassé.
            read.inspect_err(|e| tracing::error!(error = %e, "pack intégré illisible")).ok()
        })
        .collect()
});

impl LibraryKind for Templates {
    const DIR: &'static str = "widgets";
    const MANIFEST_FILE: &'static str = "widgets.json";
    const HAS_WIDGETS: bool = true;
    type Manifest = PackManifest;

    fn builtins() -> &'static [StoredVersion<PackManifest>] {
        &BUILTIN_PACKS
    }
}
