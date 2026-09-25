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

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use slider_core::Slide;

/// Ensemble proposé par défaut.
pub const DEFAULT_SET: &str = "essentiel";

const BUILTIN: &[&str] =
    &[include_str!("../../../templates/essentiel.json"), include_str!("../../../templates/grilles.json")];

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
    pub count: usize,
}

impl TemplateSet {
    pub fn info(&self, builtin: bool) -> SetInfo {
        SetInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            builtin,
            count: self.templates.len(),
        }
    }
}

/// Ensembles intégrés, dans l'ordre.
pub fn builtin_sets() -> Vec<TemplateSet> {
    BUILTIN.iter().map(|s| serde_json::from_str(s).expect("ensemble de modèles intégré valide")).collect()
}

/// Les modèles de l'ensemble par défaut (compatibilité).
pub fn templates() -> Vec<Template> {
    builtin_sets().into_iter().next().map(|s| s.templates).unwrap_or_default()
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
#[derive(Debug, Clone, Default)]
pub struct Library {
    dir: Option<PathBuf>,
}

impl Library {
    /// `dir` : dossier des ensembles de l'utilisateur (créé à la première écriture).
    pub fn new(dir: Option<PathBuf>) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    fn user_sets(&self) -> Vec<TemplateSet> {
        let Some(dir) = &self.dir else { return Vec::new() };
        let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
        let mut sets: Vec<TemplateSet> = entries
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok())
            .collect();
        sets.sort_by_key(|s| s.name.to_lowercase());
        sets
    }

    pub fn is_builtin(id: &str) -> bool {
        builtin_sets().iter().any(|s| s.id == id)
    }

    /// Tous les ensembles : intégrés d'abord.
    pub fn list(&self) -> Vec<SetInfo> {
        let mut out: Vec<SetInfo> = builtin_sets().iter().map(|s| s.info(true)).collect();
        out.extend(self.user_sets().iter().map(|s| s.info(false)));
        out
    }

    /// Un ensemble par son identifiant (celui de l'utilisateur d'abord : il
    /// peut remplacer un intégré du même nom).
    pub fn get(&self, id: &str) -> Option<(TemplateSet, bool)> {
        if let Some(s) = self.user_sets().into_iter().find(|s| s.id == id) {
            return Some((s, false));
        }
        builtin_sets().into_iter().find(|s| s.id == id).map(|s| (s, true))
    }

    /// Identifiant libre, dérivé du nom.
    pub fn free_id(&self, name: &str) -> String {
        let base = slug(name);
        let taken: Vec<String> = self.list().into_iter().map(|s| s.id).collect();
        if !taken.contains(&base) {
            return base;
        }
        (2..).map(|n| format!("{base}-{n}")).find(|id| !taken.contains(id)).expect("un identifiant libre")
    }

    /// Enregistre un ensemble de l'utilisateur (écriture atomique).
    pub fn save(&self, set: &TemplateSet) -> std::io::Result<PathBuf> {
        let dir = self.dir.as_ref().ok_or_else(|| std::io::Error::other("pas de dossier pour les modèles"))?;
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", set.id));
        let tmp = dir.join(format!(".{}.json.tmp", set.id));
        std::fs::write(&tmp, serde_json::to_vec_pretty(set).map_err(std::io::Error::other)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(path)
    }

    /// Supprime un ensemble de l'utilisateur.
    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        let dir = self.dir.as_ref().ok_or_else(|| std::io::Error::other("pas de dossier pour les modèles"))?;
        std::fs::remove_file(dir.join(format!("{id}.json")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensembles_integres_valides_et_ids_uniques() {
        let sets = builtin_sets();
        assert!(sets.len() >= 2);
        for s in &sets {
            let mut ids: Vec<&str> = s.templates.iter().map(|t| t.id.as_str()).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), s.templates.len(), "identifiants en double dans {}", s.id);
            for t in &s.templates {
                let deck = slider_core::Deck {
                    version: 1,
                    theme: None,
                    params: Default::default(),
                    vars: Default::default(),
                    tokens: serde_json::Value::Null,
                    templates: None,
                    slides: vec![t.slide.clone()],
                };
                assert!(slider_core::validate(&deck, None).is_empty(), "{} / {}", s.id, t.id);
            }
        }
    }

    #[test]
    fn noms_de_fichiers() {
        assert_eq!(slug("Mes modèles à moi !"), "mes-modeles-a-moi");
        assert_eq!(slug("  "), "modeles");
    }

    #[test]
    fn enregistrer_relire_supprimer() {
        let dir = std::env::temp_dir().join(format!("echo-templates-{}", std::process::id()));
        let lib = Library::new(Some(dir.clone()));
        let mut set = builtin_sets().remove(0);
        set.id = lib.free_id("Mon ensemble");
        set.name = "Mon ensemble".into();
        lib.save(&set).unwrap();
        assert!(lib.list().iter().any(|s| s.id == "mon-ensemble" && !s.builtin));
        assert_eq!(lib.get("mon-ensemble").unwrap().0.templates.len(), set.templates.len());
        assert_eq!(lib.free_id("Mon ensemble"), "mon-ensemble-2");
        lib.delete("mon-ensemble").unwrap();
        assert!(lib.get("mon-ensemble").is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
