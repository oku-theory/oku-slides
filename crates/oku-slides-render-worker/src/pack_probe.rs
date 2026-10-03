//! Sonde d'un pack fichier par fichier, côté hôte : chaque fichier de
//! widget est compilé seul dans le travailleur, puis le registre des seuls
//! valides est assemblé ici.
//!
//! Règles :
//! - Un échec borné (délai, mémoire, arrêt) pendant la sonde d'un fichier
//!   donne un diagnostic pour ce fichier ; aucun pack n'est accusé.
//! - Les sondes sont gardées par `probe_key` (nom, source, `_commun`
//!   importé), échecs bornés de la sonde compris : un fichier qui boucle ne
//!   se recompile qu'une fois modifié. Un autre échec du lien (dont un échec
//!   de la préparation : file différée, rejeu) n'est pas gardé.
//! - Au-delà de `max_widgets` fichiers (dans l'ordre des noms), ou pour un
//!   fichier de plus de `max_widget_kib`, rien n'est exécuté : un diagnostic.
//!   Un `_commun` hors limites n'est pas non plus fourni aux widgets.
//! - Budget : les sondes d'un appel durent au plus `timeout_ms` en tout. Une
//!   fois épuisé, les fichiers restants (hors cache) ne sont pas exécutés :
//!   un diagnostic, non gardé (sondés au prochain appel). Un pack reçu dont
//!   chaque fichier boucle ne coûte ainsi qu'un délai.
//! - `pack` ne garde que `_commun` et les widgets valides : le module
//!   d'assemblage importe chaque fichier, un fichier cassé casserait le rendu.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, PoisonError};
use std::time::Instant;

use oku_slides_core::{WidgetFile, WidgetRegistry};
use oku_slides_render::{FileProbe, Fonts, PackWidgets, RegistryError, assemble, base_probe, probe_key};

use crate::client::{RenderWorker, unexpected};
use crate::protocol::{Request, Response};

/// La sonde d'un pack, fichier par fichier.
#[derive(Debug, Clone)]
pub struct PackProbe {
    /// Par fichier (`_commun` compris) : schéma ou diagnostics.
    pub files: BTreeMap<WidgetFile, FileProbe>,
    /// `_commun` et les widgets valides ; `None` sans widget valide.
    pub pack: Option<PackWidgets>,
    /// La base surchargée par les widgets valides.
    pub registry: Arc<WidgetRegistry>,
}

impl RenderWorker {
    /// Sonde chaque fichier de `pack`, puis assemble le registre des valides.
    ///
    /// Seule une base illisible, ou un registre refusé sans widget à
    /// écarter, est une erreur.
    pub fn probe_pack(&self, pack: &PackWidgets) -> Result<PackProbe, RegistryError> {
        let (max_widgets, max_bytes) = (self.1.max_widgets, self.1.max_widget_bytes);
        let loaded = |i: usize, source: &str| i < max_widgets && source.len() <= max_bytes;
        // `_commun` vient en tête (ordre des noms) : rang 0.
        let common = pack.common().filter(|c| loaded(0, c));
        let deadline = Instant::now() + self.1.probe_budget;
        let mut files = BTreeMap::new();
        for (i, (name, source)) in pack.files().iter().enumerate() {
            let probe = if i >= max_widgets {
                FileProbe::broken(name, format!("more than {max_widgets} widget files in the pack: not loaded"))
            } else if source.len() > max_bytes {
                FileProbe::broken(name, format!("larger than {} KiB: not loaded", max_bytes / 1024))
            } else {
                self.probe_file(name, source, common.filter(|_| !name.is_common()), deadline)
            };
            files.insert(name.clone(), probe);
        }
        // Les schémas ne dépendent pas des polices : celles de Typst suffisent.
        let base = base_probe(&Fonts::default())?;
        let registry = Arc::new(assemble(&base, &mut files)?);
        let valid: BTreeSet<WidgetFile> =
            files.iter().filter(|(n, p)| !n.is_common() && p.is_valid()).map(|(n, _)| n.clone()).collect();
        let pack = (!valid.is_empty()).then(|| pack.only(&valid));
        Ok(PackProbe { files, pack, registry })
    }

    /// La sonde d'un fichier dans le travailleur, gardée par `probe_key`.
    ///
    /// Hors cache, après `deadline` : non exécutée, un diagnostic non gardé.
    fn probe_file(&self, name: &WidgetFile, source: &str, common: Option<&str>, deadline: Instant) -> FileProbe {
        let key = probe_key(name, source, common);
        if let Some(p) = self.1.file_probes.lock().unwrap_or_else(PoisonError::into_inner).get(&key) {
            return p.clone();
        }
        if Instant::now() >= deadline {
            return FileProbe::broken(name, "not probed: the pack's time budget is exhausted");
        }
        let req =
            Request::ProbeFile { name: name.to_string(), source: source.to_owned(), common: common.map(str::to_owned) };
        let mut link = self.link();
        // Un échec de la préparation n'est pas celui de ce fichier : non gardé.
        if let Err(e) = self.prepare(&mut link) {
            return FileProbe::broken(name, e.to_string());
        }
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
            self.1.file_probes.lock().unwrap_or_else(PoisonError::into_inner).insert(key, probe.clone());
        }
        probe
    }
}
