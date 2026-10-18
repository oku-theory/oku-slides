//! Les thèmes du deck seul embarqués dans ses sources : ceux de « cette présentation ».
//!
//! Le thème qu'un import génère (`PackScope::Deck`) n'entre jamais dans la
//! bibliothèque ; ses versions restent embarquées (les sources sont en ajout
//! seul). Le deck peut donc le reprendre après en avoir choisi un autre : la
//! liste du snapshot (`deck_themes`) l'offre, et `set_theme`, le brouillon du
//! mode modèles et `edit_theme` le résolvent après la bibliothèque.
//!
//! Pour chaque id, la version offerte est celle qu'utilise le deck s'il porte
//! ce thème, sinon la feuille de la chaîne des `parent` parmi les versions
//! embarquées (la plus récente par `created` si plusieurs ; sans feuille, des
//! `parent` en boucle, la plus récente de toutes).
//!
//! Le snapshot demande cette liste à chaque fois : seule la clé `scope` de
//! chaque version est lue d'abord, les manifestes du deck seul seuls en entier.

use std::collections::{BTreeMap, BTreeSet};

use snafu::OptionExt;
use uuid::Uuid;

use oku_slides_core::{PackVersion, ThemeRef};

use crate::error::UnknownThemeSnafu;
use crate::theme::{ThemeInfo, ThemeManifest};
use crate::{Result, Session};

impl Session {
    /// Les thèmes du deck seul embarqués dans les sources, un par id, par nom.
    pub fn deck_only_themes(&self) -> Vec<ThemeInfo> {
        let mut listed: Vec<ThemeInfo> = self
            .embedded_deck_only_versions()
            .into_iter()
            .filter(|(id, _)| !self.theme_list.iter().any(|t| t.id == *id))
            .filter_map(|(id, versions)| self.chosen_version(id, versions))
            .map(|(version, manifest)| ThemeInfo {
                id: manifest.id,
                version,
                name: manifest.name,
                builtin: false,
                default: false,
                deck_only: true,
            })
            .collect();
        listed.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        listed
    }

    /// La version reprise du thème du deck seul `id`.
    ///
    /// `None` s'il n'est pas embarqué.
    pub fn deck_only_theme_ref(&self, id: Uuid) -> Option<ThemeRef> {
        let versions = self.embedded_deck_only_versions().remove(&id)?;
        self.chosen_version(id, versions).map(|(version, _)| ThemeRef { id, version })
    }

    /// La version que prend le thème `id` choisi.
    ///
    /// La courante de la bibliothèque, sinon celle du deck seul
    /// (`deck_only_theme_ref`) ; sinon `UnknownTheme`.
    pub(crate) fn picked_theme(&self, id: Uuid) -> Result<ThemeRef> {
        if let Some(version) = self.themes.current(id) {
            return Ok(ThemeRef { id, version });
        }
        Ok(self.deck_only_theme_ref(id).context(UnknownThemeSnafu { id })?)
    }

    /// Les versions embarquées des thèmes du deck seul, par id.
    ///
    /// Document montré puis deck mis de côté ; seule la portée des autres versions est lue.
    fn embedded_deck_only_versions(&self) -> BTreeMap<Uuid, BTreeMap<PackVersion, ThemeManifest>> {
        let mut by_id: BTreeMap<Uuid, BTreeMap<PackVersion, ThemeManifest>> = BTreeMap::new();
        for (version, files) in self.embedded_themes().flatten() {
            // Illisible : déjà signalé à la lecture des sources ; il ne s'offre pas.
            if !matches!(files.scope(), Ok(scope) if !scope.is_library()) {
                continue;
            }
            let Ok(manifest) = ThemeManifest::from_files(files) else { continue };
            by_id.entry(manifest.id).or_default().insert(version.clone(), manifest);
        }
        by_id
    }

    /// La version offerte parmi `versions` du thème `id`.
    ///
    /// Celle du deck s'il la porte, sinon la feuille la plus récente de la
    /// chaîne des `parent` ; sans feuille (une boucle), la plus récente.
    fn chosen_version(
        &self,
        id: Uuid,
        mut versions: BTreeMap<PackVersion, ThemeManifest>,
    ) -> Option<(PackVersion, ThemeManifest)> {
        if let Some(r) = self.deck_theme().filter(|r| r.id == id)
            && let Some(manifest) = versions.remove(&r.version)
        {
            return Some((r.version.clone(), manifest));
        }
        let parents: BTreeSet<PackVersion> = versions.values().filter_map(|m| m.parent.clone()).collect();
        let newest = |(va, a): &(PackVersion, ThemeManifest), (vb, b): &(PackVersion, ThemeManifest)| {
            a.created.cmp(&b.created).then(va.cmp(vb))
        };
        let (leaves, inner): (Vec<_>, Vec<_>) = versions.into_iter().partition(|(v, _)| !parents.contains(v));
        if leaves.is_empty() {
            tracing::warn!(theme = %id, "versions du thème du deck seul en boucle : la plus récente est offerte");
            return inner.into_iter().max_by(newest);
        }
        leaves.into_iter().max_by(newest)
    }
}

#[cfg(test)]
#[path = "deck_themes_tests.rs"]
mod deck_themes_tests;
