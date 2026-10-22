//! Les sortes de versions que la bibliothèque range : packs de widgets, de layouts, puis thèmes.
//!
//! Une sorte donne son dossier, le nom du manifeste d'une version, et ses
//! versions intégrées ; la [`crate::Library`] fait le reste.

use std::sync::LazyLock;

use uuid::Uuid;

use oku_slides_core::{Namespace, PackFiles, PackKind, PackScope, PackVersion};
use oku_slides_render::{BuiltinPack, RegistryError};

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
    /// La sorte de pack d'une sorte à fichiers de widgets (`widgets/` ou `layouts/<nom>.typ`) ; `None` sans fichiers.
    const PACK_KIND: Option<PackKind>;
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

/// Une version intégrée lue de `files`, journalisée et écartée si elle est illisible.
fn builtin_version(files: Result<PackFiles, RegistryError>) -> Option<StoredVersion<PackManifest>> {
    let read = files.map_err(|e| oku_slides_platform::errors::one_line(&e)).and_then(StoredVersion::read);
    // Couvert par les tests : n'arrive qu'avec un fichier du dépôt cassé.
    read.inspect_err(|e| tracing::error!(error = %e, "pack intégré illisible")).ok()
}

/// Les packs de widgets d'éléments.
#[derive(Debug, Clone)]
pub struct Widgets;

/// Le pack de widgets intégré `base`, lu une fois par processus ; ses sources sont celles du rendu.
static BUILTIN_WIDGETS: LazyLock<Vec<StoredVersion<PackManifest>>> =
    LazyLock::new(|| builtin_version(BuiltinPack::Base.files()).into_iter().collect());

impl LibraryKind for Widgets {
    const DIR: &'static str = "widgets";
    const MANIFEST_FILE: &'static str = "widgets.json";
    const PACK_KIND: Option<PackKind> = Some(PackKind::Widgets);
    type Manifest = PackManifest;

    fn builtins() -> &'static [StoredVersion<PackManifest>] {
        &BUILTIN_WIDGETS
    }
}

/// Les packs de layouts de slide.
#[derive(Debug, Clone)]
pub struct Layouts;

/// Le pack de layouts intégré `essentiel`, lu une fois par processus ; ses sources sont celles du rendu.
static BUILTIN_LAYOUTS: LazyLock<Vec<StoredVersion<PackManifest>>> =
    LazyLock::new(|| builtin_version(BuiltinPack::Essentiel.files()).into_iter().collect());

impl LibraryKind for Layouts {
    const DIR: &'static str = "layouts";
    const MANIFEST_FILE: &'static str = "layouts.json";
    const PACK_KIND: Option<PackKind> = Some(PackKind::Layouts);
    type Manifest = PackManifest;

    fn builtins() -> &'static [StoredVersion<PackManifest>] {
        &BUILTIN_LAYOUTS
    }
}
