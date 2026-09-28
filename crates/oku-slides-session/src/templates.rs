//! Ensembles de modèles de slides.
//!
//! Un ensemble regroupe des modèles (des slides complètes, avec un nom et une
//! description). Il est indépendant du thème : n'importe quel ensemble se rend
//! avec n'importe quel thème, puisque les modèles ne parlent qu'en jetons.
//!
//! Un ensemble est un pack : un dossier `<id>/` qui contient `pack.json`
//! (l'ensemble) et, s'il en déclare, `widgets.typ` (ses widgets, qui
//! surchargent ceux de base pour les decks qui choisissent ce pack).
//!
//! - Ensembles intégrés : `templates/<id>/pack.json` du dépôt, embarqués dans
//!   l'app, en lecture seule (les modifier en enregistre une copie). Ils
//!   n'ont pas de widgets.
//! - Ensembles de l'utilisateur : dossiers dans le dossier de données de
//!   l'app (`templates/`), créés et modifiés par le mode d'édition des
//!   modèles. Un ancien fichier `<id>.json` n'est pas lu.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};

use oku_slides_platform::storage::Storage;

use crate::Result;
use crate::error::{NoTemplateSetSnafu, NoTemplatesDirSnafu, TemplateJsonSnafu, WriteSnafu};

use oku_slides_core::Slide;
use oku_slides_render::is_pack_id;

/// Ensemble proposé par défaut.
pub const DEFAULT_SET: &str = "essentiel";

/// Ensembles intégrés, lus une fois par processus.
static BUILTIN: LazyLock<Vec<TemplateSet>> = LazyLock::new(|| {
    [include_str!("../../../templates/essentiel/pack.json"), include_str!("../../../templates/grilles/pack.json")]
        .iter()
        .filter_map(|s| match serde_json::from_str(s) {
            Ok(set) => Some(set),
            // Couvert par les tests : n'arrive qu'avec un fichier du dépôt cassé.
            Err(e) => {
                tracing::error!(error = %e, "ensemble de modèles intégré illisible");
                None
            }
        })
        .collect()
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub slide: Slide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateSet {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub templates: Vec<Template>,
}

/// Résumé d'un ensemble, pour les listes.
#[derive(Debug, Clone, Serialize)]
pub struct SetInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub builtin: bool,
    /// L'ensemble d'un deck qui n'en choisit pas ([`DEFAULT_SET`]).
    pub default: bool,
    pub count: usize,
}

impl TemplateSet {
    pub fn info(&self, builtin: bool) -> SetInfo {
        SetInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            builtin,
            default: self.id == DEFAULT_SET,
            count: self.templates.len(),
        }
    }
}

/// Ensembles intégrés, dans l'ordre.
pub fn builtin_sets() -> &'static [TemplateSet] {
    &BUILTIN
}

/// Identifiant de fichier à partir d'un nom : minuscules, tirets.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c.to_ascii_lowercase(),
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_owned();
    if out.is_empty() { "modeles".into() } else { out }
}

/// Fichier de l'ensemble, dans le dossier d'un pack.
const PACK_FILE: &str = "pack.json";
/// Widgets d'un pack, dans son dossier (facultatif).
const WIDGETS_FILE: &str = "widgets.typ";

/// Bibliothèque : ensembles intégrés + ensembles de l'utilisateur.
#[derive(Debug, Clone)]
pub struct Library {
    dir: Option<PathBuf>,
    storage: Storage,
}

impl Library {
    /// `dir` : dossier des ensembles de l'utilisateur (créé à la première écriture).
    pub fn new(dir: Option<PathBuf>, storage: Storage) -> Self {
        Self { dir, storage }
    }

    /// Dossier du pack `id` de l'utilisateur ; `None` sans dossier des
    /// modèles ou pour un identifiant qui n'est pas un nom de dossier.
    fn pack_dir(&self, id: &str) -> Option<PathBuf> {
        let dir = self.dir.as_ref()?;
        is_pack_id(id).then(|| dir.join(id))
    }

    /// Ensembles de l'utilisateur, par nom : un dossier par pack, son
    /// identifiant est le nom du dossier. Un pack illisible est signalé dans
    /// les journaux et ignoré : il ne doit pas cacher les autres.
    fn user_sets(&self) -> Vec<TemplateSet> {
        let Some(dir) = &self.dir else { return Vec::new() };
        let folders = self.storage.list_dirs(dir).unwrap_or_else(|e| {
            tracing::warn!(dir = %dir.display(), error = %oku_slides_platform::errors::one_line(&e), "modèles");
            Vec::new()
        });
        let mut sets: Vec<TemplateSet> = folders
            .iter()
            .filter_map(|folder| {
                let id = folder.file_name()?.to_str().filter(|id| is_pack_id(id))?.to_owned();
                let path = folder.join(PACK_FILE);
                let parsed = self
                    .storage
                    .read(&path)
                    .map_err(|e| oku_slides_platform::errors::one_line(&e))
                    .and_then(|b| serde_json::from_slice::<TemplateSet>(&b).map_err(|e| e.to_string()));
                let mut set = parsed
                    .inspect_err(|e| tracing::warn!(path = %path.display(), error = %e, "pack illisible"))
                    .ok()?;
                // Le dossier fait foi : un pack copié sous un autre nom est ce nom.
                set.id = id;
                Some(set)
            })
            .collect();
        sets.sort_by_key(|s| s.name.to_lowercase());
        sets
    }

    /// Source des widgets du pack `id` (`widgets.typ` de son dossier) ;
    /// `None` pour un pack sans widgets (dont les intégrés). Un fichier
    /// présent mais illisible est signalé dans les journaux : le pack se
    /// rend alors avec les widgets de base.
    pub fn widgets(&self, id: &str) -> Option<String> {
        let path = self.pack_dir(id)?.join(WIDGETS_FILE);
        match self.storage.len(&path) {
            Ok(Some(_)) => {}
            Ok(None) => return None,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %oku_slides_platform::errors::one_line(&e), "widgets du pack");
                return None;
            }
        }
        let read = self
            .storage
            .read(&path)
            .map_err(|e| oku_slides_platform::errors::one_line(&e))
            .and_then(|b| String::from_utf8(b).map_err(|e| e.to_string()));
        read.inspect_err(|e| tracing::warn!(path = %path.display(), error = %e, "widgets du pack illisibles")).ok()
    }

    pub fn is_builtin(id: &str) -> bool {
        BUILTIN.iter().any(|s| s.id == id)
    }

    /// Tous les ensembles : intégrés d'abord.
    pub fn list(&self) -> Vec<SetInfo> {
        let mut out: Vec<SetInfo> = BUILTIN.iter().map(|s| s.info(true)).collect();
        out.extend(self.user_sets().iter().map(|s| s.info(false)));
        out
    }

    /// Un ensemble par son identifiant (celui de l'utilisateur d'abord : il
    /// peut remplacer un intégré du même nom). Un intégré est prêté, pas copié.
    pub fn get(&self, id: &str) -> Option<(Cow<'static, TemplateSet>, bool)> {
        if let Some(s) = self.user_sets().into_iter().find(|s| s.id == id) {
            return Some((Cow::Owned(s), false));
        }
        BUILTIN.iter().find(|s| s.id == id).map(|s| (Cow::Borrowed(s), true))
    }

    /// Identifiant libre, dérivé du nom.
    pub fn free_id(&self, name: &str) -> String {
        let base = slug(name);
        let taken: Vec<String> = self.list().into_iter().map(|s| s.id).collect();
        if !taken.contains(&base) {
            return base;
        }
        let mut n = 2;
        loop {
            let id = format!("{base}-{n}");
            if !taken.contains(&id) {
                return id;
            }
            n += 1;
        }
    }

    /// Enregistre un ensemble de l'utilisateur : `pack.json` dans le dossier
    /// de son pack, créé au besoin (écriture atomique). Ses widgets, s'il en
    /// a, restent.
    pub fn save(&self, set: &TemplateSet) -> Result<PathBuf> {
        self.dir.as_ref().context(NoTemplatesDirSnafu)?;
        let path = self.pack_dir(&set.id).context(NoTemplateSetSnafu { id: &set.id })?.join(PACK_FILE);
        let json = serde_json::to_vec_pretty(set).context(TemplateJsonSnafu { path: &path })?;
        self.storage.write_atomic(&path, json).context(WriteSnafu { path: &path })?;
        Ok(path)
    }

    /// Écrit `source` pour les widgets du pack `id` de l'utilisateur
    /// (`widgets.typ` de son dossier, écriture atomique).
    pub fn save_widgets(&self, id: &str, source: &str) -> Result<PathBuf> {
        self.dir.as_ref().context(NoTemplatesDirSnafu)?;
        let path = self.pack_dir(id).context(NoTemplateSetSnafu { id })?.join(WIDGETS_FILE);
        self.storage.write_atomic(&path, source.as_bytes()).context(WriteSnafu { path: &path })?;
        Ok(path)
    }

    /// Supprime un ensemble de l'utilisateur : le dossier de son pack.
    pub fn delete(&self, id: &str) -> Result<()> {
        self.dir.as_ref().context(NoTemplatesDirSnafu)?;
        let path = self.pack_dir(id).context(NoTemplateSetSnafu { id })?;
        self.storage.delete_dir(&path).context(WriteSnafu { path })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "templates_tests.rs"]
mod templates_tests;
