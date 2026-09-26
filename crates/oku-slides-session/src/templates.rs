//! Ensembles de modèles de slides.
//!
//! Un ensemble regroupe des modèles (des slides complètes, avec un nom et une
//! description). Il est indépendant du thème : n'importe quel ensemble se rend
//! avec n'importe quel thème, puisque les modèles ne parlent qu'en jetons.
//!
//! - Ensembles intégrés : `templates/*.json` du dépôt, embarqués dans l'app,
//!   en lecture seule (les modifier en enregistre une copie).
//! - Ensembles de l'utilisateur : fichiers JSON dans le dossier de données de
//!   l'app (`templates/`), créés et modifiés par le mode d'édition des modèles.

use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt};

use oku_slides_platform::storage::Storage;

use crate::Result;
use crate::error::{NoTemplatesDirSnafu, TemplateJsonSnafu, WriteSnafu};

use oku_slides_core::Slide;

/// Ensemble proposé par défaut.
pub const DEFAULT_SET: &str = "essentiel";

/// Ensembles intégrés, lus une fois par processus.
static BUILTIN: LazyLock<Vec<TemplateSet>> = LazyLock::new(|| {
    [include_str!("../../../templates/essentiel.json"), include_str!("../../../templates/grilles.json")]
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

    /// Ensembles de l'utilisateur, par nom. Un fichier illisible est signalé
    /// dans les journaux et ignoré : il ne doit pas cacher les autres.
    fn user_sets(&self) -> Vec<TemplateSet> {
        let Some(dir) = &self.dir else { return Vec::new() };
        let files = self.storage.list_files(dir).unwrap_or_else(|e| {
            tracing::warn!(dir = %dir.display(), error = %oku_slides_platform::errors::one_line(&e), "modèles");
            Vec::new()
        });
        let mut sets: Vec<TemplateSet> = files
            .iter()
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .filter_map(|p| {
                let parsed = self
                    .storage
                    .read(p)
                    .map_err(|e| oku_slides_platform::errors::one_line(&e))
                    .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()));
                parsed.inspect_err(|e| tracing::warn!(path = %p.display(), error = %e, "ensemble illisible")).ok()
            })
            .collect();
        sets.sort_by_key(|s| s.name.to_lowercase());
        sets
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

    /// Enregistre un ensemble de l'utilisateur (écriture atomique).
    pub fn save(&self, set: &TemplateSet) -> Result<PathBuf> {
        let dir = self.dir.as_ref().context(NoTemplatesDirSnafu)?;
        let path = dir.join(format!("{}.json", set.id));
        let json = serde_json::to_vec_pretty(set).context(TemplateJsonSnafu { path: &path })?;
        self.storage.write_atomic(&path, json).context(WriteSnafu { path: &path })?;
        Ok(path)
    }

    /// Supprime un ensemble de l'utilisateur.
    pub fn delete(&self, id: &str) -> Result<()> {
        let dir = self.dir.as_ref().context(NoTemplatesDirSnafu)?;
        let path = dir.join(format!("{id}.json"));
        self.storage.delete(&path).context(WriteSnafu { path })?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "templates_tests.rs"]
mod templates_tests;
