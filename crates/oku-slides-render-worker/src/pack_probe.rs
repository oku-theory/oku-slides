//! Sonde d'un pack fichier par fichier, côté hôte : chaque fichier de
//! widget est compilé seul dans le travailleur (dans le dossier de la sorte du
//! pack), puis le registre des seuls valides est assemblé ici : à côté du
//! `base` intégré pour un pack d'un autre espace de noms (ses slots peuvent
//! accepter `base/*`), sur une sonde vide pour un pack `base`. Un slot qui
//! accepte un kind d'un autre pack (hors `base`) y reste refusé, comme un
//! widget dont la catégorie contredit la sorte du pack.
//!
//! Règles :
//! - Un échec borné (délai, mémoire, arrêt) pendant la sonde d'un fichier
//!   donne un diagnostic pour ce fichier ; aucun pack n'est accusé.
//! - Les sondes sont gardées par `probe_key` (sorte, nom, source, `_commun`),
//!   échecs bornés de la sonde compris : un fichier qui boucle ne se
//!   recompile qu'une fois modifié, ou `_commun` modifié. Quelques sondes
//!   par fichier sont gardées (`FileProbeCache`) : le cache ne grandit pas à
//!   l'édition, et alterner deux packs aux mêmes noms ne recompile rien.
//!   Un autre échec du lien n'est pas gardé.
//! - Un échec de la préparation du lien (file différée, lancement, rejeu)
//!   n'est pas celui du fichier : non exécuté, un diagnostic non gardé, et
//!   nommé dans `PackProbe::unprobed` (repris au prochain appel).
//! - Au-delà de `max_widgets` fichiers (dans l'ordre des noms), ou pour un
//!   fichier de plus de `max_widget_kib`, rien n'est exécuté : un diagnostic.
//! - Les diagnostics de la sonde elle-même (limites, non sondé) sont dits
//!   dans la langue des emplacements du travailleur (Fluent `probe-*`),
//!   l'anglais avant le premier emplacement.
//!   Un `_commun` hors limites n'est pas non plus fourni aux widgets.
//! - Budget : les sondes d'un appel durent au plus `probe_budget_ms` en tout, le
//!   premier fichier exécuté passant toujours (chaque appel avance). Une
//!   fois épuisé, les fichiers restants (hors cache) ne sont pas exécutés :
//!   un diagnostic, non gardé (sondés au prochain appel), et nommés dans
//!   `PackProbe::unprobed`. Un pack reçu dont chaque fichier boucle ne coûte
//!   ainsi qu'un délai par appel ; un délai gardé n'est plus « non sondé ».
//! - Les registres assemblés sont gardés (les [`MAX_ASSEMBLED`] derniers) par
//!   empreinte : celle du pack réduit aux valides pour `probe_pack`, celle de
//!   l'ensemble pour `registry_of`. Le même appel rend le même `Arc`, et la
//!   session ne réinstalle pas des widgets inchangés.
//! - `pack` ne garde que `_commun` et les widgets valides : le module
//!   d'assemblage importe chaque fichier, un fichier cassé casserait le rendu.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, PoisonError};
use std::time::Instant;

use oku_slides_core::{PackKind, WidgetFile, WidgetRegistry, kinds};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_render::{
    FileProbe, Fonts, PackSet, PackWidgets, RegistryError, RenderError, assemble, assemble_set, base_probe, probe_key,
};
use serde_json::{Value as Json, json};

use crate::client::{RenderWorker, unexpected};
use crate::protocol::{Request, Response};

/// Registres assemblés gardés, au plus : les plus récemment rendus.
pub(crate) const MAX_ASSEMBLED: usize = 8;

/// La sonde d'un pack, fichier par fichier.
#[derive(Debug, Clone)]
pub struct PackProbe {
    /// Par fichier (`_commun` compris) : schéma ou diagnostics.
    pub files: BTreeMap<WidgetFile, FileProbe>,
    /// `_commun` et les widgets valides ; `None` sans widget valide.
    pub pack: Option<PackWidgets>,
    /// Le registre des widgets valides, à côté de `base` (sauf pour `base` lui-même).
    pub registry: Arc<WidgetRegistry>,
    /// Fichiers non exécutés (budget épuisé, lien indisponible) : à sonder au prochain appel.
    pub unprobed: BTreeSet<WidgetFile>,
}

impl RenderWorker {
    /// Probes each file of `pack`, then assembles the registry of the valid ones.
    ///
    /// Only an unreadable base, or a registry rejected with no widget to
    /// set aside, is an error.
    ///
    /// # Errors
    ///
    /// Fails if the pack base is unreadable, or if the registry is rejected with no
    /// widget to set aside.
    pub fn probe_pack(&self, pack: &PackWidgets) -> Result<PackProbe, RegistryError> {
        let (max_widgets, max_bytes) = (self.1.max_widgets, self.1.max_widget_bytes);
        let loaded = |i: usize, source: &str| i < max_widgets && source.len() <= max_bytes;
        // `_commun` vient en tête (ordre des noms) : rang 0.
        let common = pack.common().filter(|c| loaded(0, c));
        let mut budget = Budget::new(self.1.probe_budget);
        let mut files = BTreeMap::new();
        let mut unprobed = BTreeSet::new();
        // Langue des emplacements, lue seulement si un diagnostic est à dire.
        let localizer = OnceCell::new();
        let l = || localizer.get_or_init(|| self.localizer());
        for (i, (name, source)) in pack.files().iter().enumerate() {
            let probe = if i >= max_widgets {
                FileProbe::broken(name, l().tr_with("probe-too-many-files", &[("max", max_widgets.into())]))
            } else if source.len() > max_bytes {
                FileProbe::broken(name, l().tr_with("probe-too-large", &[("max", (max_bytes / 1024).into())]))
            } else {
                let common = common.filter(|_| !name.is_common());
                self.probe_file(pack.kind(), name, source, common, &mut budget).unwrap_or_else(|skipped| {
                    unprobed.insert(name.clone());
                    let message = match skipped {
                        Skipped::Budget => l().tr("probe-not-probed"),
                        Skipped::Link(e) => e.localize(l()),
                    };
                    FileProbe::broken(name, message)
                })
            };
            files.insert(name.clone(), probe);
        }
        // Les schémas ne dépendent pas des polices : celles de Typst suffisent. Un pack autre que
        // `base` s'assemble à côté du `base` intégré : ses slots peuvent accepter des kinds `base/*`.
        let fixed = if pack.namespace().as_str() == kinds::base::NAMESPACE {
            empty_probe()
        } else {
            (*base_probe(&Fonts::default())?).clone()
        };
        let registry = assemble(&fixed, pack.namespace(), pack.kind(), &mut files)?;
        let valid: BTreeSet<WidgetFile> =
            files.iter().filter(|(n, p)| !n.is_common() && p.is_valid()).map(|(n, _)| n.clone()).collect();
        let kept = pack.only(&valid);
        let registry = self.kept_registry(kept.fingerprint(), || Ok((registry, true)))?;
        let pack = (!valid.is_empty()).then_some(kept);
        Ok(PackProbe { files, pack, registry, unprobed })
    }

    /// The registry of the packs of `set`, already probed, cached by the set's digest.
    ///
    /// Invalid files are set aside; two calls for the same set return the same `Arc`.
    ///
    /// # Errors
    ///
    /// Fails if a pack of `set` cannot be probed (see [`Self::probe_pack`]) or if
    /// the set's registry is rejected.
    pub fn registry_of(&self, set: &PackSet) -> Result<Arc<WidgetRegistry>, RegistryError> {
        self.kept_registry(set.fingerprint(), || {
            // Sondes gardées par `probe_key` : un pack déjà sondé ne recompile rien.
            let mut probes = set.packs().iter().map(|p| self.probe_pack(p)).collect::<Result<Vec<_>, _>>()?;
            // Un fichier non sondé manque au registre : il ne doit pas être gardé sous l'ensemble complet.
            let complete = probes.iter().all(|p| p.unprobed.is_empty());
            let mut packs: Vec<_> = set
                .packs()
                .iter()
                .zip(probes.iter_mut())
                .map(|(pack, probe)| (pack.namespace(), pack.kind(), &mut probe.files))
                .collect();
            Ok((assemble_set(&empty_probe(), &mut packs)?, complete))
        })
    }

    /// Le registre gardé sous `key`, sinon celui de `build`, gardé s'il est complet :
    /// deux appels pour la même clé donnent le même `Arc`, que la session compare par `Arc::ptr_eq`.
    fn kept_registry(
        &self,
        key: [u8; 32],
        build: impl FnOnce() -> Result<(WidgetRegistry, bool), RegistryError>,
    ) -> Result<Arc<WidgetRegistry>, RegistryError> {
        let found = {
            let mut kept = self.1.assembled.lock().unwrap_or_else(PoisonError::into_inner);
            kept.iter().position(|(k, _)| *k == key).and_then(|i| kept.remove(i))
        };
        // Construit hors du verrou : `build` sonde des fichiers et reprend ce verrou.
        let entry = if let Some(entry) = found {
            entry
        } else {
            let (registry, complete) = build()?;
            if !complete {
                return Ok(Arc::new(registry));
            }
            (key, Arc::new(registry))
        };
        let registry = entry.1.clone();
        let mut kept = self.1.assembled.lock().unwrap_or_else(PoisonError::into_inner);
        kept.retain(|(k, _)| *k != key);
        while kept.len() >= MAX_ASSEMBLED {
            kept.pop_front();
        }
        kept.push_back(entry);
        Ok(registry)
    }

    /// La langue posée dans les emplacements du travailleur ; l'anglais avant le premier.
    ///
    /// Gardée par code de langue : la construire coûte.
    pub(crate) fn localizer(&self) -> Arc<Localizer> {
        let locale = self.link().state.locale.clone().unwrap_or_else(|| "en".to_owned());
        let mut kept = self.1.probe_localizer.lock().unwrap_or_else(PoisonError::into_inner);
        match &*kept {
            Some((code, l)) if *code == locale => l.clone(),
            _ => {
                let l = Arc::new(Localizer::new(Some(&locale)));
                *kept = Some((locale, l.clone()));
                l
            }
        }
    }

    /// La sonde d'un fichier dans le travailleur, gardée par `probe_key`.
    ///
    /// Hors cache, `budget` épuisé ou si le lien ne se prépare pas : non
    /// exécutée, [`Skipped`].
    fn probe_file(
        &self,
        kind: PackKind,
        name: &WidgetFile,
        source: &str,
        common: Option<&str>,
        budget: &mut Budget,
    ) -> Result<FileProbe, Skipped> {
        let key = probe_key(kind, name, source, common);
        if let Some(p) = self.1.file_probes.lock().unwrap_or_else(PoisonError::into_inner).get(name, &key) {
            return Ok(p);
        }
        budget.spend()?;
        let req = Request::ProbeFile {
            kind,
            name: name.to_string(),
            source: source.to_owned(),
            common: common.map(str::to_owned),
        };
        let mut link = self.link();
        // Un échec de la préparation n'est pas celui de ce fichier : non sondé.
        self.prepare(&mut link).map_err(Skipped::Link)?;
        let reply = link.call(&req, &[]);
        drop(link);
        let (probe, keep) = match reply {
            Ok((Response::FileProbe { probe }, _)) => (probe, true),
            Ok((Response::Error { error }, _)) => (FileProbe::broken(name, error.into_render().to_string()), false),
            Ok((other, _)) => (FileProbe::broken(name, unexpected(&other).to_string()), false),
            // Délai, mémoire, arrêt : ce fichier est en cause, lui seul.
            Err(e) if e.kind().is_bounded() => (FileProbe::broken(name, e.to_string()), true),
            Err(e) => (FileProbe::broken(name, e.to_string()), false),
        };
        if keep {
            self.1.file_probes.lock().unwrap_or_else(PoisonError::into_inner).insert(name, key, probe.clone());
        }
        Ok(probe)
    }
}

/// Une sonde sans pack : la base d'assemblage de `base` et d'un ensemble.
fn empty_probe() -> Json {
    json!({ "packs": [] })
}

/// Le budget de temps d'un appel : le premier fichier exécuté passe toujours.
#[derive(Debug)]
struct Budget {
    deadline: Instant,
    /// Un fichier a déjà été exécuté dans cet appel.
    spent: bool,
}

impl Budget {
    fn new(budget: std::time::Duration) -> Self {
        Self { deadline: Instant::now() + budget, spent: false }
    }

    /// Autorise l'exécution d'un fichier : le premier de l'appel, puis jusqu'à l'échéance.
    fn spend(&mut self) -> Result<(), Skipped> {
        if self.spent && Instant::now() >= self.deadline {
            return Err(Skipped::Budget);
        }
        self.spent = true;
        Ok(())
    }
}

/// Pourquoi un fichier n'a pas été sondé (repris au prochain appel).
#[derive(Debug)]
enum Skipped {
    /// Le budget de temps de l'appel est épuisé.
    Budget,
    /// Le lien n'a pas pu être préparé.
    Link(RenderError),
}

#[cfg(test)]
#[path = "pack_set_tests.rs"]
mod pack_set_tests;
