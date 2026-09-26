//! Reprise : l'état de la session, écrit à chaque modification sous
//! `<data>/resume/`, relu au lancement suivant.
//!
//! `state.json` dit le mode et l'état de l'interface ; chaque document
//! (`deck/`, `templates/`) tient dans un `document.json` (deck, historique,
//! fichier) écrit d'un coup, plus ses assets et polices, écrits une fois par
//! nom.
//!
//! Le dossier de reprise suppose un seul écrivain (une seule instance de
//! l'app) : rien n'y protège contre des écritures concurrentes.

// Rien ici n'est encore appelé hors des tests : la Session le branchera en
// Task 4. Sans ça, le build de production (sans les tests) voit tout le
// module comme mort.
#![cfg_attr(not(test), expect(dead_code, reason = "branché en Task 4"))]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use snafu::{Backtrace, ResultExt, Snafu, ensure};

use oku_slides_core::{Deck, History, SourceBundle};
use oku_slides_platform::storage::{Storage, StorageError};

use crate::views::{TemplateMode, UiState};
use crate::{Document, EXTENSION, SessionError};

pub(crate) const VERSION: u32 = 1;
pub(crate) const DECK_DIR: &str = "deck";
// Mort aussi dans les tests (aucun ne s'en sert encore) : le laisser au
// module-level `expect` ci-dessus l'y ferait manquer sa cible.
#[cfg_attr(test, expect(dead_code, reason = "branché en Task 4"))]
pub(crate) const TEMPLATES_DIR: &str = "templates";

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
    #[cfg_attr(test, expect(dead_code, reason = "branché en Task 4"))]
    Session { source: SessionError, backtrace: Backtrace },
}

type Result<T, E = ResumeError> = std::result::Result<T, E>;

/// Ce que la session éditait.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SavedMode {
    Deck,
    Templates(TemplateMode),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct State {
    pub version: u32,
    pub mode: SavedMode,
    #[serde(default)]
    pub ui: UiState,
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
    deck: &'a Deck,
    history: &'a History,
}

#[derive(Deserialize)]
struct DocIn {
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default)]
    dirty: bool,
    #[serde(default)]
    saved_hash: Option<String>,
    deck: Deck,
    /// Lu à part : un historique illisible ne coûte pas le deck.
    #[serde(default)]
    history: serde_json::Value,
}

/// Un nom de fichier sûr : un seul composant normal, ni vide ni caché (les
/// fichiers cachés sont ignorés à la lecture : en écrire un le perdrait).
fn safe_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && Path::new(name).file_name().is_some_and(|n| n == name)
}

/// Le dossier de reprise et ses fichiers.
#[derive(Debug)]
pub(crate) struct Resume {
    storage: Storage,
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

    pub fn write_state(&self, state: &State) -> Result<()> {
        let path = self.dir.join("state.json");
        let bytes = serde_json::to_vec(state).context(JsonSnafu { path: &path })?;
        self.put(&path, bytes)
    }

    /// L'état du dernier lancement ; `None` au premier.
    pub fn read_state(&self) -> Result<Option<State>> {
        let path = self.dir.join("state.json");
        if !self.storage.exists(&path).context(AccessSnafu { path: &path })? {
            return Ok(None);
        }
        let bytes = self.get(&path)?;
        // La version se lit à part : une version future peut avoir changé de
        // forme, illisible en `State` alors que sa version l'est bien.
        let versioned: Versioned = serde_json::from_slice(&bytes).context(JsonSnafu { path: &path })?;
        ensure!(versioned.version == VERSION, VersionSnafu { version: versioned.version });
        let state: State = serde_json::from_slice(&bytes).context(JsonSnafu { path: &path })?;
        Ok(Some(state))
    }

    /// Écrit un document : assets et polices d'abord (le deck les cite), puis
    /// `document.json` d'un coup, et enfin les fichiers orphelins retirés —
    /// dans cet ordre, un fichier cité par `document.json` a toujours été
    /// écrit avant, et on ne perd jamais un fichier encore cité si on
    /// s'interrompt entre les deux étapes.
    pub fn write_doc(&self, sub: &str, doc: &Document) -> Result<()> {
        let dir = self.dir.join(sub);
        // Le deck cite les assets par identifiant : on garde ce nom tel quel
        // (pas question de le recalculer depuis les octets), juste validé —
        // `sha256:` suivi d'hexadécimal ne sait pas sortir du dossier.
        let assets = doc.sources.assets.iter().filter_map(|(id, b)| match id.strip_prefix("sha256:") {
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
        let fonts = doc.sources.fonts.iter().filter_map(|(name, b)| {
            if !safe_name(name) {
                tracing::warn!(name, "nom de police de reprise refusé");
                return None;
            }
            let hash = oku_slides_core::asset_id(b);
            let hex = hash.strip_prefix("sha256:").unwrap_or(&hash);
            Some((format!("{hex}-{name}"), b))
        });
        let keep_fonts = self.write_missing(&dir.join("fonts"), fonts)?;

        let path = dir.join("document.json");
        let out = DocOut {
            path: doc.path.as_deref(),
            dirty: doc.dirty,
            saved_hash: doc.saved_hash.as_deref(),
            deck: &doc.sources.deck,
            history: &doc.history,
        };
        let bytes = serde_json::to_vec(&out).context(JsonSnafu { path: &path })?;
        self.put(&path, bytes)?;

        self.prune(&dir.join("assets"), &keep_assets)?;
        self.prune(&dir.join("fonts"), &keep_fonts)?;
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

    /// Relit un document ; son historique garde au plus `history_limit` transactions.
    pub fn read_doc(&self, sub: &str, history_limit: usize) -> Result<Document> {
        let dir = self.dir.join(sub);
        let path = dir.join("document.json");
        let raw: DocIn = serde_json::from_slice(&self.get(&path)?).context(JsonSnafu { path: &path })?;
        let mut history = serde_json::from_value::<History>(raw.history).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "historique de reprise illisible");
            History::new(history_limit)
        });
        history.set_limit(history_limit);
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
        Ok(Document {
            sources: SourceBundle { deck: raw.deck, assets, fonts },
            history,
            path,
            dirty: raw.dirty,
            notice: None,
            saved_hash: raw.saved_hash,
        })
    }
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod resume_tests;
