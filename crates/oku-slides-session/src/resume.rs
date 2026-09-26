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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use snafu::{Backtrace, ResultExt, Snafu, ensure};

use oku_slides_core::{Deck, History, SourceBundle};
use oku_slides_platform::storage::{Storage, StorageError};

use crate::files::read_document;
use crate::views::{TemplateMode, UiState};
use crate::{Document, EXTENSION, Mode, Session, SessionError};

pub(crate) const VERSION: u32 = 1;
pub(crate) const DECK_DIR: &str = "deck";
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

    /// Déplace tout le dossier de reprise vers un dossier voisin
    /// `<nom>.failed-<horodatage>[-<n>]`, pour ne jamais écraser du travail
    /// non enregistré qu'on n'a pas su relire ; renvoie ce nouveau dossier.
    ///
    /// Le suffixe numérique évite qu'un deuxième échec dans la même seconde
    /// (relance rapide après un premier échec) vise le même dossier voisin :
    /// un renommage vers un dossier déjà pris le remplacerait plutôt que de
    /// le garder, ce qui perdrait justement ce qu'on essaie de préserver.
    fn quarantine(&self) -> Result<PathBuf> {
        let secs =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default();
        let name = self.dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "resume".into());
        let mut to = self.dir.with_file_name(format!("{name}.failed-{secs}"));
        let mut n = 1u32;
        while self.storage.exists(&to).context(AccessSnafu { path: &self.dir })? {
            to = self.dir.with_file_name(format!("{name}.failed-{secs}-{n}"));
            n += 1;
        }
        self.storage.rename(&self.dir, &to).context(AccessSnafu { path: &self.dir })?;
        Ok(to)
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

impl Session {
    /// Active la reprise dans `dir` : relit l'état du dernier lancement, puis
    /// ouvre `file` (« ouvrir avec… »), sauf s'il y a du travail non
    /// enregistré : il passe avant, et un avis le dit.
    ///
    /// Une reprise illisible ne doit jamais coûter le travail non enregistré
    /// qu'elle contenait peut-être : avant de repartir d'un deck vierge, le
    /// dossier en échec est déplacé de côté plutôt qu'écrasé. S'il n'a même
    /// pas pu être déplacé, on n'écrit rien du tout ce lancement-ci (la
    /// reprise reste désactivée), pour ne prendre aucun risque de plus.
    pub fn resume(&mut self, dir: PathBuf, file: Option<&Path>) {
        let resume = Resume::new(self.storage.clone(), dir);
        let keep = match self.restore(&resume) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "reprise impossible : deck vierge");
                // `restore` peut avoir échoué après avoir déjà basculé en mode
                // modèles (son propre rendu a échoué) : on repart d'un mode
                // cohérent avant `new_deck`, qui ne touche qu'au deck.
                self.mode = Mode::Deck;
                if let Err(e) = self.new_deck() {
                    tracing::warn!(error = %e.one_line(), "deck vierge");
                }
                match resume.quarantine() {
                    Ok(moved) => {
                        tracing::warn!(path = %moved.display(), "reprise en échec mise de côté");
                        true
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %oku_slides_platform::errors::one_line(&e),
                            "reprise en échec non déplacée : rien n'est écrit"
                        );
                        false
                    }
                }
            }
        };
        if let Some(file) = file {
            self.open_at_launch(file);
        }
        self.resume = keep.then_some(resume);
        self.persist(true);
    }

    fn restore(&mut self, resume: &Resume) -> Result<()> {
        let Some(state) = resume.read_state()? else { return Ok(()) };
        let deck = self.fresh_from_disk(resume.read_doc(DECK_DIR, self.history_limit)?);
        let templates = match state.mode {
            SavedMode::Deck => None,
            SavedMode::Templates(edit) => match resume.read_doc(TEMPLATES_DIR, self.history_limit) {
                Ok(doc) => Some((doc, edit)),
                Err(e) => {
                    tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "modèles de reprise illisibles");
                    None
                }
            },
        };
        self.load_document(deck).context(SessionSnafu)?;
        if let Some((doc, edit)) = templates {
            let stash = std::mem::replace(&mut self.doc, doc);
            self.mode = Mode::Templates { stash: Box::new(stash), edit };
            self.rerender().context(SessionSnafu)?;
        }
        self.ui = state.ui;
        Ok(())
    }

    /// Un document non modifié dont le fichier a changé depuis est relu sur
    /// le disque, sans historique ; absent ou illisible, la copie de travail reste.
    fn fresh_from_disk(&self, saved: Document) -> Document {
        let Some(path) = saved.path.as_ref().filter(|_| !saved.dirty) else { return saved };
        match read_document(&self.storage, path) {
            Ok(opened) if saved.saved_hash.as_deref() != Some(opened.hash.as_str()) => {
                let mut doc = Document::new(opened.sources, Some(path.clone()), self.history_limit);
                doc.saved_hash = Some(opened.hash);
                doc.notice = opened.modified_elsewhere.then(|| self.localizer.tr("notice-modified-elsewhere"));
                doc
            }
            Ok(_) => saved,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e.one_line(), "fichier du document illisible");
                saved
            }
        }
    }

    /// Travail non enregistré : le deck, ou l'ensemble de modèles en cours.
    fn unsaved(&self) -> bool {
        self.doc.dirty || matches!(&self.mode, Mode::Templates { stash, .. } if stash.dirty)
    }

    fn open_at_launch(&mut self, file: &Path) {
        if self.unsaved() {
            // Le fichier demandé au lancement est déjà celui affiché (avec du
            // travail non enregistré) : pas la peine de dire qu'il n'a pas
            // été ouvert, l'utilisateur le regarde déjà.
            let dirty_path = match &self.mode {
                Mode::Templates { stash, .. } if stash.dirty => stash.path.as_deref(),
                _ => self.doc.path.as_deref(),
            };
            if dirty_path != Some(file) {
                let notice = Some(self.localizer.tr("notice-resume-kept-unsaved"));
                // L'avis porte sur le document affiché (la capture le montre
                // toujours), et aussi sur le deck mis de côté quand c'est lui
                // qui porte le travail non enregistré, pour qu'il survive à
                // `close_templates`.
                self.doc.notice = notice.clone();
                if let Mode::Templates { stash, .. } = &mut self.mode
                    && stash.dirty
                {
                    stash.notice = notice;
                }
            }
            return;
        }
        let opened = if matches!(self.mode, Mode::Templates { .. }) {
            self.close_templates().and_then(|()| self.open(file))
        } else {
            self.open(file)
        };
        if let Err(e) = opened {
            tracing::warn!(path = %file.display(), error = %e.one_line(), "ouverture au démarrage");
        }
    }

    /// Écrit l'état pour la reprise ; `documents` : aussi les documents.
    pub(crate) fn persist(&self, documents: bool) {
        let Some(resume) = &self.resume else { return };
        if let Err(e) = self.write_resume(resume, documents) {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "état de reprise non enregistré");
        }
    }

    fn write_resume(&self, resume: &Resume, documents: bool) -> Result<()> {
        let (deck, mode) = match &self.mode {
            Mode::Deck => (&self.doc, SavedMode::Deck),
            Mode::Templates { stash, edit } => (&**stash, SavedMode::Templates(edit.clone())),
        };
        if documents {
            resume.write_doc(DECK_DIR, deck)?;
            if matches!(self.mode, Mode::Templates { .. }) {
                resume.write_doc(TEMPLATES_DIR, &self.doc)?;
            }
        }
        resume.write_state(&State { version: VERSION, mode, ui: self.ui.clone() })
    }
}

#[cfg(test)]
#[path = "resume_tests.rs"]
mod resume_tests;
