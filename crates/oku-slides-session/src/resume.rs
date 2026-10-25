//! Reprise d'une session (un onglet) : son état.
//! Écrit à chaque modification dans son dossier, relu au lancement suivant.
//!
//! `state.json` dit le mode et l'état de l'interface ; chaque document
//! (`deck/`, `pack/`) tient dans un `deck.loro` (le replica : deck,
//! identité et historique complet ; pas la pile d'annulation) et un
//! `document.json` (fichier, état), chacun écrit d'un coup, plus ses assets,
//! polices, packs (`<hex>.json`, `<hex>.<nom>.typ`) et thèmes (`themes/<hex>.json`),
//! écrits une fois par nom.
//!
//! Un seul écrivain par dossier : la session de l'onglet. L'espace de travail
//! (`workspace.rs`) écrit seul `workspace.json`, au-dessus.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use snafu::{Backtrace, ResultExt, Snafu, ensure};

use oku_slides_core::{
    DeckReplica, PackFiles, PackKind, PackVersion, ReplicaError, ReplicaErrorKind, ReplicaLimits, SourceBundle,
    WidgetFile, WidgetRegistry,
};
use oku_slides_platform::storage::{Storage, StorageError};

use crate::files::read_document;
use crate::pack_scratch::PackScratch;
use crate::resume_writer::{AgentWrite, Job, Writer};
use crate::theme_edit::ThemeEdit;
use crate::views::{PackMode, UiState};
use crate::{DocDefaults, Document, EXTENSION, Mode, Session, SessionError};

pub(crate) const VERSION: u32 = 3;
pub(crate) const DECK_DIR: &str = "deck";
pub(crate) const PACK_DIR: &str = "pack";
/// Taille maximale d'un JSON relu par [`Resume::read_json`] : au-delà, illisible.
pub(crate) const MAX_JSON_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum ResumeError {
    #[snafu(display("cannot access {}", path.display()))]
    Access { path: PathBuf, source: StorageError, backtrace: Backtrace },
    #[snafu(display("unreadable {}", path.display()))]
    Json { path: PathBuf, source: serde_json::Error, backtrace: Backtrace },
    #[snafu(display("unsupported resume state version {version}"))]
    Version { version: u32, backtrace: Backtrace },
    #[snafu(display("cannot restore the session"))]
    Session { source: SessionError, backtrace: Backtrace },
    #[snafu(display("theme draft namespace {namespace} does not match a readable base"))]
    ThemeBase { namespace: String, backtrace: Backtrace },
    #[snafu(display("unreadable replica {}", path.display()))]
    Replica {
        path: PathBuf,
        #[snafu(source(from(ReplicaError, Box::new)))]
        source: Box<ReplicaError>,
        backtrace: Backtrace,
    },
}

pub(crate) type Result<T, E = ResumeError> = std::result::Result<T, E>;

impl ResumeError {
    /// L'état vient d'une version plus ancienne de l'app : état ou deck d'un format refusé.
    fn is_outdated(&self) -> bool {
        match self {
            Self::Version { version, .. } => *version < VERSION,
            Self::Replica { source, .. } => source.kind() == ReplicaErrorKind::UnsupportedVersion,
            Self::Access { .. } | Self::Json { .. } | Self::Session { .. } | Self::ThemeBase { .. } => false,
        }
    }
}

/// Ce qu'a donné la reprise d'un onglet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Resumed {
    /// L'état est relu.
    Restored,
    /// L'état, illisible, est mis de côté.
    Unreadable,
    /// L'état d'une version plus ancienne, avec du travail non enregistré, est mis de côté.
    Outdated,
}

/// Ce que la session éditait.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SavedMode {
    Deck,
    Pack(PackMode),
    /// L'aide, dans cette langue : relue depuis l'app, aucun document gardé.
    Help(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct State {
    pub version: u32,
    pub mode: SavedMode,
    #[serde(default, deserialize_with = "crate::views::kept_ui")]
    pub ui: UiState,
    /// Id du thème de l'aide (`SavedMode::Help`), relu avec elle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help_theme: Option<uuid::Uuid>,
    /// Le thème édité (`SavedMode::Pack`) ; absent : celui du deck.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_edit: Option<ThemeEdit>,
}

/// Octets d'un document (images, polices, packs, thèmes), partagés d'un envoi au
/// suivant tant qu'ils ne changent pas : une commande ne les recopie pas.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Blobs {
    pub assets: BTreeMap<String, Vec<u8>>,
    pub fonts: BTreeMap<String, Vec<u8>>,
    pub packs: BTreeMap<PackVersion, PackFiles>,
    pub themes: BTreeMap<PackVersion, PackFiles>,
}

impl Blobs {
    /// Mêmes octets que `sources` ? Un asset, un pack ou un thème est nommé par son
    /// empreinte : ses clés suffisent. Une police est nommée par son
    /// fichier : ses octets comptent aussi.
    fn matches(&self, sources: &SourceBundle) -> bool {
        self.assets.keys().eq(sources.assets.keys())
            && self.fonts == sources.fonts
            && self.packs.keys().eq(sources.packs.keys())
            && self.themes.keys().eq(sources.themes.keys())
    }
}

/// Un document à écrire hors du verrou de la session (fil de reprise).
pub(crate) struct DocSnapshot {
    /// Snapshot Loro du replica (`deck.loro`).
    replica: Vec<u8>,
    path: Option<PathBuf>,
    dirty: bool,
    saved_hash: Option<String>,
    blank: bool,
    blobs: Arc<Blobs>,
}

impl DocSnapshot {
    #[cfg(test)]
    pub(crate) fn blobs(&self) -> &Arc<Blobs> {
        &self.blobs
    }

    /// L'instantané de `doc` ; ses octets viennent de `cache` s'ils n'ont pas
    /// changé depuis un envoi précédent, sinon ils y entrent (copiés une fois).
    /// Échoue si le replica ne s'exporte pas : l'appelant n'écrit alors rien.
    #[cfg(test)]
    pub(crate) fn of(doc: &Document, cache: &mut Vec<Arc<Blobs>>) -> Result<Self, ReplicaError> {
        Self::with_blobs(doc, &doc.sources, cache)
    }

    /// Comme `of`, les images et polices prises à `sources` (celles du deck
    /// prêtées à l'aperçu de l'historique).
    pub(crate) fn with_blobs(
        doc: &Document,
        sources: &SourceBundle,
        cache: &mut Vec<Arc<Blobs>>,
    ) -> Result<Self, ReplicaError> {
        let replica = doc.sources.replica().snapshot()?;
        let blobs = if let Some(b) = cache.iter().find(|b| b.matches(sources)) {
            Arc::clone(b)
        } else {
            let b = Arc::new(Blobs {
                assets: sources.assets.clone(),
                fonts: sources.fonts.clone(),
                packs: sources.packs.clone(),
                themes: sources.themes.clone(),
            });
            // Deck and edited pack: two documents at most.
            if cache.len() >= 2 {
                cache.remove(0);
            }
            cache.push(Arc::clone(&b));
            b
        };
        Ok(Self {
            replica,
            path: doc.path.clone(),
            dirty: doc.dirty,
            saved_hash: doc.saved_hash.clone(),
            blank: doc.blank,
            blobs,
        })
    }
}

/// Juste assez de `state.json` pour vérifier la version avant de tenter de
/// lire le reste : une version future peut avoir une forme qu'on ne sait pas
/// désérialiser en `State`.
#[derive(Deserialize)]
struct Versioned {
    version: u32,
}

#[derive(Serialize)]
struct DocOut<'a> {
    path: Option<&'a Path>,
    dirty: bool,
    saved_hash: Option<&'a str>,
    blank: bool,
}

#[derive(Deserialize)]
struct DocIn {
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default)]
    dirty: bool,
    #[serde(default)]
    saved_hash: Option<String>,
    #[serde(default)]
    blank: bool,
}

/// Un nom de fichier sûr : un seul composant normal, ni vide ni caché (les
/// fichiers cachés sont ignorés à la lecture : en écrire un le perdrait).
fn safe_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && Path::new(name).file_name().is_some_and(|n| n == name)
}

/// Un dossier de reprise (celui d'un onglet, ou la racine) et ses fichiers.
#[derive(Debug, Clone)]
pub(crate) struct Resume {
    pub(crate) storage: Storage,
    pub(crate) dir: PathBuf,
}

impl Resume {
    pub fn new(storage: Storage, dir: PathBuf) -> Self {
        Self { storage, dir }
    }

    fn put(&self, path: &Path, bytes: Vec<u8>) -> Result<()> {
        self.storage.write_atomic(path, bytes).context(AccessSnafu { path })
    }

    fn get(&self, path: &Path) -> Result<Vec<u8>> {
        self.storage.read(path).context(AccessSnafu { path })
    }

    /// Écrit `name` (JSON) dans le dossier, d'un coup.
    pub fn write_json(&self, name: &str, value: &impl Serialize) -> Result<()> {
        let path = self.dir.join(name);
        let bytes = serde_json::to_vec(value).context(JsonSnafu { path: &path })?;
        self.put(&path, bytes)
    }

    /// Relit `name` s'il existe ; une autre version que `version` est une erreur.
    ///
    /// Au-delà de [`MAX_JSON_BYTES`], le fichier est illisible, sans être lu.
    pub fn read_json<T: DeserializeOwned>(&self, name: &str, version: u32) -> Result<Option<T>> {
        let path = self.dir.join(name);
        if !self.storage.exists(&path).context(AccessSnafu { path: &path })? {
            return Ok(None);
        }
        let bytes = self.storage.read_limited(&path, MAX_JSON_BYTES).context(AccessSnafu { path: &path })?;
        // La version se lit à part : une version future peut avoir changé de
        // forme, illisible en `T` alors que sa version l'est bien.
        let versioned: Versioned = serde_json::from_slice(&bytes).context(JsonSnafu { path: &path })?;
        ensure!(versioned.version == version, VersionSnafu { version: versioned.version });
        Ok(Some(serde_json::from_slice(&bytes).context(JsonSnafu { path: &path })?))
    }

    pub fn write_state(&self, state: &State) -> Result<()> {
        self.write_json("state.json", state)
    }

    /// L'état du dernier lancement ; `None` au premier.
    pub fn read_state(&self) -> Result<Option<State>> {
        self.read_json("state.json", VERSION)
    }

    /// Écrit un document : assets et polices d'abord (le deck les cite), puis
    /// `deck.loro` et `document.json`, chacun d'un coup, et enfin les
    /// fichiers orphelins retirés — dans cet ordre, un fichier cité par le
    /// deck a toujours été écrit avant, et on ne perd jamais un fichier encore
    /// cité si on s'interrompt entre deux étapes.
    pub(crate) fn write_snapshot(&self, sub: &str, doc: &DocSnapshot) -> Result<()> {
        let out = DocOut {
            path: doc.path.as_deref(),
            dirty: doc.dirty,
            saved_hash: doc.saved_hash.as_deref(),
            blank: doc.blank,
        };
        let (assets, fonts) = (&doc.blobs.assets, &doc.blobs.fonts);
        let dir = self.dir.join(sub);
        // Le deck cite les assets par identifiant : on garde ce nom tel quel
        // (pas question de le recalculer depuis les octets), juste validé —
        // `sha256:` suivi d'hexadécimal ne sait pas sortir du dossier.
        let assets = assets.iter().filter_map(|(id, b)| match id.strip_prefix("sha256:") {
            Some(hex) if !hex.is_empty() && hex.bytes().all(|c| c.is_ascii_hexdigit()) => Some((hex.to_owned(), b)),
            _ => {
                tracing::warn!(id, "identifiant d'asset de reprise refusé");
                None
            }
        });
        let keep_assets = self.write_missing(&dir.join("assets"), assets)?;

        // Les polices sont adressées par contenu (`<hex>-<nom>`) : un même nom
        // avec un contenu différent écrit un fichier différent plutôt que de
        // garder le vieux fichier périmé (le fichier périmé est retiré plus
        // bas, avec les autres orphelins).
        let fonts = fonts.iter().filter_map(|(name, b)| {
            if !safe_name(name) {
                tracing::warn!(name, "nom de police de reprise refusé");
                return None;
            }
            let hash = oku_slides_core::asset_id(b);
            let hex = hash.strip_prefix("sha256:").unwrap_or(&hash);
            Some((format!("{hex}-{name}"), b))
        });
        let keep_fonts = self.write_missing(&dir.join("fonts"), fonts)?;

        // Les packs sont adressés par contenu : `<hex>.json` et un `<hex>.<nom>.typ` par fichier de widget.
        let packs_dir = dir.join("packs");
        let mut keep_packs = BTreeSet::new();
        for (version, files) in &doc.blobs.packs {
            let hex = version.hex();
            let json = packs_dir.join(format!("{hex}.json"));
            if !self.storage.exists(&json).context(AccessSnafu { path: &json })? {
                // Les widgets d'abord : un `<hex>.json` présent dit le pack complet.
                for (name, source) in files.widgets() {
                    self.put(&packs_dir.join(format!("{hex}.{}", name.file_name())), source.as_bytes().to_vec())?;
                }
                self.put(&json, files.pack_json().context(JsonSnafu { path: &json })?)?;
            }
            keep_packs.insert(format!("{hex}.json"));
            keep_packs.extend(files.widgets().keys().map(|n| format!("{hex}.{}", n.file_name())));
        }

        // Les thèmes aussi : `<hex>.json`, le `theme.json` de la version.
        let themes_dir = dir.join("themes");
        let mut keep_themes = BTreeSet::new();
        for (version, files) in &doc.blobs.themes {
            let json = themes_dir.join(format!("{}.json", version.hex()));
            if !self.storage.exists(&json).context(AccessSnafu { path: &json })? {
                self.put(&json, files.pack_json().context(JsonSnafu { path: &json })?)?;
            }
            keep_themes.insert(format!("{}.json", version.hex()));
        }

        self.put(&dir.join("deck.loro"), doc.replica.clone())?;
        let path = dir.join("document.json");
        let bytes = serde_json::to_vec(&out).context(JsonSnafu { path: &path })?;
        self.put(&path, bytes)?;

        self.prune(&dir.join("assets"), &keep_assets)?;
        self.prune(&dir.join("fonts"), &keep_fonts)?;
        self.prune(&packs_dir, &keep_packs)?;
        self.prune(&themes_dir, &keep_themes)?;
        Ok(())
    }

    /// Écrit les fichiers du dossier absents (nom déjà validé par l'appelant)
    /// et renvoie leurs noms.
    fn write_missing<'a>(
        &self,
        dir: &Path,
        files: impl Iterator<Item = (String, &'a Vec<u8>)>,
    ) -> Result<BTreeSet<String>> {
        let mut keep = BTreeSet::new();
        for (name, bytes) in files {
            let path = dir.join(&name);
            if !self.storage.exists(&path).context(AccessSnafu { path: &path })? {
                self.put(&path, bytes.clone())?;
            }
            keep.insert(name);
        }
        Ok(keep)
    }

    /// Supprime les fichiers du dossier qui ne sont plus dans `keep`.
    fn prune(&self, dir: &Path, keep: &BTreeSet<String>) -> Result<()> {
        for path in self.storage.list_files(dir).context(AccessSnafu { path: dir })? {
            let stale = path.file_name().is_some_and(|n| !keep.contains(&*n.to_string_lossy()));
            if stale {
                self.storage.delete(&path).context(AccessSnafu { path: &path })?;
            }
        }
        Ok(())
    }

    fn read_files(&self, dir: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
        let mut files = BTreeMap::new();
        for path in self.storage.list_files(dir).context(AccessSnafu { path: dir })? {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if !name.is_empty() && !name.starts_with('.') {
                files.insert(name, self.get(&path)?);
            }
        }
        Ok(files)
    }

    /// Déplace ce dossier (celui d'un onglet, ou la racine) vers un dossier
    /// voisin `<nom>.failed-<horodatage>[-<n>]`, pour ne jamais écraser du
    /// travail non enregistré qu'on n'a pas su relire.
    /// Renvoie ce nouveau dossier.
    ///
    /// Le suffixe numérique évite qu'un deuxième échec dans la même seconde
    /// (relance rapide après un premier échec) vise le même dossier voisin :
    /// un renommage vers un dossier déjà pris le remplacerait plutôt que de
    /// le garder, ce qui perdrait justement ce qu'on essaie de préserver.
    pub(crate) fn quarantine(&self) -> Result<PathBuf> {
        let secs =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
        let name = self.dir.file_name().map_or_else(|| "resume".into(), |n| n.to_string_lossy().into_owned());
        let mut to = self.dir.with_file_name(format!("{name}.failed-{secs}"));
        let mut n = 1u32;
        while self.storage.exists(&to).context(AccessSnafu { path: &self.dir })? {
            to = self.dir.with_file_name(format!("{name}.failed-{secs}-{n}"));
            n += 1;
        }
        self.storage.rename(&self.dir, &to).context(AccessSnafu { path: &self.dir })?;
        Ok(to)
    }

    /// Ne garde que les `keep` plus récents des dossiers mis de côté de `dir`.
    ///
    /// Candidats : `<base>.failed-<secondes>[-<n>]` où `is_base(<base>)`
    /// (voir [`failed_to_prune`]) ; rien d'autre n'est touché. Chaque
    /// suppression, ou son échec, est journalisée.
    pub(crate) fn prune_failed(storage: &Storage, dir: &Path, is_base: impl Fn(&str) -> bool, keep: usize) {
        let listed = match storage.list_dirs(dir) {
            Ok(listed) => listed,
            Err(e) => {
                tracing::warn!(
                    path = %dir.display(),
                    error = %oku_slides_platform::errors::one_line(&e),
                    "reprises mises de côté non listées : rien n'est supprimé"
                );
                return;
            }
        };
        let names: Vec<String> =
            listed.iter().filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).collect();
        for name in failed_to_prune(&names, is_base, keep) {
            let path = dir.join(&name);
            match storage.delete_dir(&path) {
                Ok(()) => tracing::info!(path = %path.display(), "ancienne reprise mise de côté supprimée"),
                Err(e) => tracing::warn!(
                    path = %path.display(),
                    error = %oku_slides_platform::errors::one_line(&e),
                    "ancienne reprise mise de côté non supprimée"
                ),
            }
        }
    }

    /// Le deck gardé (`deck/document.json`) avait-il du travail non enregistré ?
    ///
    /// Lu seul, sans le replica : un état refusé se lit encore ici. Illisible : faux, journalisé.
    pub(crate) fn deck_unsaved(&self) -> bool {
        let path = self.dir.join(DECK_DIR).join("document.json");
        let read = self
            .get(&path)
            .and_then(|bytes| serde_json::from_slice::<DocIn>(&bytes).context(JsonSnafu { path: &path }));
        read.inspect_err(
            |e| tracing::warn!(error = %oku_slides_platform::errors::one_line(e), "document de reprise illisible"),
        )
        .is_ok_and(|doc| doc.dirty)
    }

    /// Relit un document : son replica (`deck.loro`, au plus
    /// `limits.max_bytes`, compacté selon `limits.compact_after_secs`), avec une pile d'annulation neuve et l'auteur de
    /// `defaults`, écrit ensuite avec les widgets de `registry`. Sans
    /// `deck.loro` (ancien format), illisible.
    pub fn read_doc(
        &self,
        sub: &str,
        defaults: &DocDefaults,
        limits: impl Into<ReplicaLimits>,
        registry: &Arc<WidgetRegistry>,
    ) -> Result<Document> {
        let dir = self.dir.join(sub);
        let path = dir.join("document.json");
        let raw: DocIn = serde_json::from_slice(&self.get(&path)?).context(JsonSnafu { path: &path })?;
        let loro = dir.join("deck.loro");
        // Borné à la lecture : un fichier démesuré n'est pas chargé en mémoire.
        let limits = limits.into();
        let max = u64::try_from(limits.max_bytes).unwrap_or(u64::MAX);
        let bytes = self.storage.read_limited(&loro, max).context(AccessSnafu { path: &loro })?;
        let (replica, read) =
            DeckReplica::from_snapshot(&bytes, limits, registry.clone()).context(ReplicaSnafu { path: &loro })?;
        let assets =
            self.read_files(&dir.join("assets"))?.into_iter().map(|(n, b)| (format!("sha256:{n}"), b)).collect();
        // Le nom sur disque est `<hex>-<nom>` (adressage par contenu) : on ne
        // garde que le nom d'origine, après le premier tiret.
        let fonts = self
            .read_files(&dir.join("fonts"))?
            .into_iter()
            .map(|(n, b)| {
                let name = match n.split_once('-') {
                    Some((_, name)) => name.to_owned(),
                    None => n,
                };
                (name, b)
            })
            .collect();
        // Un fichier qui n'est plus un `.oku-slides.pdf` absolu n'est pas une
        // cible d'enregistrement valable (déplacé, ou `document.json` altéré) :
        // on ouvre quand même le document, juste sans fichier associé.
        let path = match raw.path {
            Some(p) if p.is_absolute() && p.file_name().is_some_and(|n| n.to_string_lossy().ends_with(EXTENSION)) => {
                Some(p)
            }
            Some(p) => {
                tracing::warn!(path = %p.display(), "fichier de reprise invalide");
                None
            }
            None => None,
        };
        let raw_packs = self.read_files(&dir.join("packs"))?;
        let mut packs = BTreeMap::new();
        for (name, bytes) in &raw_packs {
            let Some(hex) = name.strip_suffix(".json") else { continue };
            let prefix = format!("{hex}.");
            let widgets = raw_packs
                .iter()
                .filter_map(|(n, b)| WidgetFile::from_file_name(n.strip_prefix(&prefix)?).map(|w| (w, b.as_slice())));
            // Vérifiée par son empreinte, qui dit aussi sa sorte : un pack altéré est ignoré.
            let widgets: Vec<_> = widgets.collect();
            let verified = PackKind::ALL
                .into_iter()
                .find_map(|kind| PackFiles::verified(kind, hex, bytes, widgets.iter().map(|(n, b)| (n.clone(), *b))));
            if let Some((version, files)) = verified {
                packs.insert(version, files);
            } else {
                tracing::warn!(version = hex, "pack de reprise altéré ignoré");
            }
        }
        let mut themes = BTreeMap::new();
        for (name, bytes) in self.read_files(&dir.join("themes"))? {
            let Some(hex) = name.strip_suffix(".json") else { continue };
            // Vérifié par son empreinte : un thème altéré est ignoré.
            if let Some(files) = PackFiles::verified_theme(hex, &bytes) {
                themes.insert(files.version(), files);
            } else {
                tracing::warn!(version = hex, "thème de reprise altéré ignoré");
            }
        }
        let (sources, issues) = SourceBundle::from_replica(replica, read, assets, fonts, packs, themes);
        for issue in &issues {
            tracing::warn!(issue = %issue, "partie illisible de la reprise mise de côté");
        }
        let mut doc = Document::new(sources, path, defaults);
        doc.dirty = raw.dirty;
        doc.saved_hash = raw.saved_hash;
        doc.blank = raw.blank;
        Ok(doc)
    }
}

/// Horodatage et suffixe d'un dossier `<base>.failed-<secondes>[-<n>]`.
fn failed_stamp(name: &str, is_base: &impl Fn(&str) -> bool) -> Option<(u64, u64)> {
    let (base, rest) = name.split_once(".failed-")?;
    if !is_base(base) {
        return None;
    }
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let (secs, n) = match rest.split_once('-') {
        Some((secs, n)) if digits(secs) && digits(n) => (secs, n),
        None if digits(rest) => (rest, "0"),
        _ => return None,
    };
    Some((secs.parse().ok()?, n.parse().ok()?))
}

/// Les dossiers mis de côté à supprimer : tous sauf les `keep` plus récents.
///
/// Ordre : horodatage puis suffixe, en nombres (`-100-10` après `-100-2`).
/// Un nom hors du motif n'est jamais candidat. Renvoyés du plus ancien au
/// plus récent.
pub(crate) fn failed_to_prune(names: &[String], is_base: impl Fn(&str) -> bool, keep: usize) -> Vec<String> {
    let mut failed: Vec<((u64, u64), &String)> =
        names.iter().filter_map(|n| Some((failed_stamp(n, &is_base)?, n))).collect();
    failed.sort();
    let excess = failed.len().saturating_sub(keep);
    failed.into_iter().take(excess).map(|(_, n)| n.clone()).collect()
}

impl Session {
    /// Active la reprise dans `dir`, le dossier d'un onglet.
    ///
    /// Relit l'état qui s'y trouve, ou y écrit l'état courant s'il n'y en a
    /// pas. Renvoie `false` si l'état est illisible : il ne doit jamais
    /// coûter le travail non enregistré qu'il contenait peut-être, le dossier
    /// est donc déplacé de côté (jamais écrasé) ; la session repart d'un deck
    /// vierge et n'écrit rien. L'espace de travail retire alors l'onglet au
    /// lancement, ou le garde vierge s'il le relit plus tard (`workspace_slot.rs`).
    #[must_use]
    pub fn resume(&mut self, dir: PathBuf) -> bool {
        self.resume_checked(dir) == Resumed::Restored
    }

    /// [`Self::resume`], qui dit aussi pourquoi un état est mis de côté.
    ///
    /// `Outdated` : un état d'une version plus ancienne de l'app (état ou deck
    /// d'un format refusé) dont le deck avait du travail non enregistré.
    pub(crate) fn resume_checked(&mut self, dir: PathBuf) -> Resumed {
        // Un seul écrivain par dossier : l'ancien fil finit et s'arrête d'abord.
        self.resume = None;
        let resume = Resume::new(self.storage.clone(), dir);
        if let Err(e) = self.restore(&resume) {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "reprise impossible");
            let outdated = e.is_outdated() && resume.deck_unsaved();
            // `restore` peut avoir échoué après avoir déjà basculé en mode
            // pack (son propre rendu a échoué) : on repart d'un mode
            // cohérent avant `new_deck`, qui ne touche qu'au deck.
            self.mode = Mode::Deck;
            if let Err(e) = self.new_deck() {
                tracing::warn!(error = %e.one_line(), "deck vierge");
            }
            match resume.quarantine() {
                Ok(moved) => tracing::warn!(path = %moved.display(), "reprise en échec mise de côté"),
                Err(e) => tracing::warn!(
                    error = %oku_slides_platform::errors::one_line(&e),
                    "reprise en échec non déplacée"
                ),
            }
            self.conversation = None;
            // Rien n'est écrit à la place, même si `resume` était déjà activée.
            self.resume = None;
            return if outdated { Resumed::Outdated } else { Resumed::Unreadable };
        }
        self.resume = Some(Writer::new(resume));
        self.persist(true);
        self.flush_resume();
        Resumed::Restored
    }

    /// Active la reprise d'un onglet neuf dans `dir`, sans rien y relire.
    ///
    /// Le deck courant (un fichier tout juste ouvert) est gardé. Un état
    /// déjà présent, ou illisible, n'est pas le sien : mis de côté, jamais
    /// relu ni écrasé, puis la reprise démarre à neuf. Seul un reste
    /// impossible à déplacer laisse la session sans reprise (`false`).
    #[must_use]
    pub(crate) fn resume_fresh(&mut self, dir: PathBuf) -> bool {
        self.conversation = None;
        self.resume = None;
        let resume = Resume::new(self.storage.clone(), dir);
        match resume.read_state() {
            Ok(None) => {}
            found => {
                if let Err(e) = &found {
                    tracing::warn!(error = %oku_slides_platform::errors::one_line(e), "reste de reprise illisible");
                }
                match resume.quarantine() {
                    Ok(moved) => tracing::warn!(path = %moved.display(), "reste de reprise mis de côté"),
                    Err(e) => {
                        tracing::warn!(
                            error = %oku_slides_platform::errors::one_line(&e),
                            "reste de reprise non déplacé : rien n'est écrit"
                        );
                        self.resume = None;
                        return false;
                    }
                }
            }
        }
        self.resume = Some(Writer::new(resume));
        self.persist(true);
        self.flush_resume();
        true
    }

    fn restore(&mut self, resume: &Resume) -> Result<()> {
        self.conversation = resume.read_conversation();
        let Some(state) = resume.read_state()? else { return Ok(()) };
        if let SavedMode::Help(locale) = &state.mode {
            self.open_help(locale, state.help_theme).context(SessionSnafu)?;
            self.ui = state.ui.sanitized(false);
            return Ok(());
        }
        let max = self.source_limits.replica();
        let deck = self.fresh_from_disk(resume.read_doc(DECK_DIR, &self.doc_defaults, max, &self.registry)?);
        let pack = match state.mode {
            SavedMode::Deck | SavedMode::Help(_) => None,
            SavedMode::Pack(edit) => match resume.read_doc(PACK_DIR, &self.doc_defaults, max, &self.registry) {
                Ok(doc) => Some((doc, edit)),
                Err(e) => {
                    tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "pack de reprise illisible");
                    None
                }
            },
        };
        self.load_document(deck).context(SessionSnafu)?;
        if let Some((doc, edit)) = pack {
            let theme = match state.theme_edit {
                Some(theme) => theme,
                None => self.initial_theme_edit().context(SessionSnafu)?.0,
            };
            let stash = std::mem::replace(&mut self.doc, doc);
            self.mode = Mode::Pack {
                stash: Box::new(stash),
                edit,
                scratch: PackScratch::default(),
                theme,
                switches: Vec::new(),
            };
            self.embed_theme_base();
            // La bibliothèque a pu changer depuis : une liste refusée, ou un espace
            // de noms qui n'est pas celui de la base, fait échouer la reprise (mise
            // de côté), jamais réparée en silence.
            if let Mode::Pack { theme, .. } = &self.mode {
                let base = self.base_manifest().map(|m| m.namespace);
                ensure!(
                    base.as_ref() == Some(&theme.namespace),
                    ThemeBaseSnafu { namespace: theme.namespace.as_str() }
                );
                self.check_theme_extends(theme, &theme.extends).context(SessionSnafu)?;
            }
            self.sync_widgets().context(SessionSnafu)?;
            self.rerender().context(SessionSnafu)?;
        }
        self.ui = state.ui.sanitized(matches!(self.mode, Mode::Pack { .. }));
        Ok(())
    }

    /// Un document non modifié dont le fichier a changé depuis est relu sur
    /// le disque, sans historique ; absent ou illisible, la copie de travail reste.
    fn fresh_from_disk(&self, saved: Document) -> Document {
        let Some(path) = saved.path.as_ref().filter(|_| !saved.dirty) else { return saved };
        match read_document(&self.storage, path, self.source_limits, &self.registry) {
            Ok(opened) if saved.saved_hash.as_deref() != Some(opened.hash.as_str()) => {
                let notice = opened.notice(&self.localizer);
                let mut doc = Document::new(opened.sources, Some(path.clone()), &self.doc_defaults);
                doc.saved_hash = Some(opened.hash);
                doc.notice = notice;
                doc
            }
            Ok(_) => saved,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e.one_line(), "fichier du document illisible");
                saved
            }
        }
    }

    /// Envoie l'état à écrire pour la reprise ; `documents` : aussi les documents.
    ///
    /// Ne bloque pas : le fil de reprise écrit (voir `resume_writer`). Seul
    /// le snapshot du replica est copié ; images et polices inchangées sont
    /// partagées avec l'envoi précédent. Un replica qui ne s'exporte pas est
    /// journalisé, et ce document n'est pas écrit.
    pub(crate) fn persist(&mut self, documents: bool) {
        self.persist_with(documents, None);
    }

    /// Comme [`Session::persist`], avec la conversation de l'agent à écrire ou effacer.
    pub(crate) fn persist_with(&mut self, documents: bool, agent: Option<AgentWrite>) {
        let Some(writer) = &self.resume else { return };
        // Pendant l'historique, le deck mis de côté est écrit en mode deck (un
        // onglet rouvert n'y revient pas), avec les images et polices prêtées
        // à l'aperçu.
        let (deck, blobs, mode, theme_edit) = match &self.mode {
            // L'aide se relit depuis l'app : seule sa langue est gardée.
            Mode::Help { locale } => {
                let state = State {
                    version: VERSION,
                    mode: SavedMode::Help((*locale).to_owned()),
                    ui: self.ui.clone(),
                    help_theme: {
                        let deck = self.doc.sources.deck();
                        deck.theme.as_ref().and_then(|ns| deck.themes.get(ns)).map(|pin| pin.id)
                    },
                    theme_edit: None,
                };
                writer.send(Job { docs: Vec::new(), state, agent });
                return;
            }
            Mode::Deck => (&self.doc, &self.doc, SavedMode::Deck, None),
            Mode::Pack { stash, edit, theme, .. } => {
                (&**stash, &**stash, SavedMode::Pack(edit.clone()), Some(theme.clone()))
            }
            Mode::History { stash, .. } => (&**stash, &self.doc, SavedMode::Deck, None),
        };
        let mut docs = Vec::new();
        if documents {
            let pack = matches!(self.mode, Mode::Pack { .. }).then_some((PACK_DIR, &self.doc, &self.doc));
            for (sub, doc, blobs) in std::iter::once((DECK_DIR, deck, blobs)).chain(pack) {
                match DocSnapshot::with_blobs(doc, &blobs.sources, &mut self.sent_blobs) {
                    Ok(snapshot) => docs.push((sub, snapshot)),
                    Err(e) => tracing::warn!(
                        sub,
                        error = %oku_slides_platform::errors::one_line(&e),
                        "replica non exporté : document de reprise non écrit"
                    ),
                }
            }
        }
        let state = State { version: VERSION, mode, ui: self.ui.clone(), help_theme: None, theme_edit };
        writer.send(Job { docs, state, agent });
    }
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod resume_tests;
