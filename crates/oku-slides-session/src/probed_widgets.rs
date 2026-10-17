//! Les widgets d'un deck hors session.
//!
//! Son pack embarqué, sondé par le processus de rendu ([`ProbedWidgets::probe`]). Chaque fichier
//! est sondé à part, les widgets cassés écartés (diagnostics au journal) ; le registre sondé valide
//! le deck (t-355).
//!
//! Un import ou une commande n'a pas de « rendu suivant » : les fichiers
//! laissés non sondés (budget épuisé) sont sondés de nouveau, tant qu'un appel
//! en sonde au moins un, en [`PROBE_ROUNDS`] appels et [`PROBE_BUDGETS`]
//! budgets de sonde au plus. Le code d'un pack reçu est isolé et borné dans
//! ce processus (délai, mémoire : spec t-327).
//!
//! L'import et la CLI s'en servent ; leurs erreurs sont celles de l'import
//! ([`ImportError`](crate::import::ImportError)).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Instant;

use snafu::ResultExt;

use oku_slides_core::{Deck, PackFiles, PackVersion, SourceBundle, WidgetFile, WidgetRegistry};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_platform::errors::one_line;
use oku_slides_render::{Fonts, PackWidgets, RenderError};
use oku_slides_render_worker::{RenderSlot, RenderWorker};

use crate::import::error::{LibrarySnafu, Result, SourcesSnafu};
use crate::{PackLibrary, adopt_default_pack};

/// Appels de sonde d'un pack au plus.
///
/// Les sondes faites sont gardées par le processus de rendu : un appel qui ne
/// fait pas baisser le nombre de fichiers non sondés (processus indisponible)
/// arrête la relance avant cette borne.
const PROBE_ROUNDS: usize = 8;

/// Durée totale de la relance, en budgets de sonde (`probe_budget_ms`).
///
/// Chaque appel peut épuiser le sien, la relance n'en épuise pas plus de
/// trois (un import ou une commande `open` ou `render` attend au plus ce temps-là).
const PROBE_BUDGETS: u32 = 3;

/// Appelle `probe` tant qu'il progresse sur des fichiers non sondés, dans le temps donné.
///
/// `unprobed` compte les fichiers non sondés d'un résultat ; `expired`, lu
/// avant chaque relance, dit le temps écoulé. [`PROBE_ROUNDS`] appels au
/// plus ; la première erreur arrête tout.
fn probe_rounds<T, E>(
    mut probe: impl FnMut() -> Result<T, E>,
    unprobed: impl Fn(&T) -> usize,
    expired: impl Fn() -> bool,
) -> Result<T, E> {
    let mut last = probe()?;
    for _ in 1..PROBE_ROUNDS {
        let left = unprobed(&last);
        if left == 0 || expired() {
            break;
        }
        let next = probe()?;
        let progressed = unprobed(&next) < left;
        last = next;
        if !progressed {
            break;
        }
    }
    Ok(last)
}

/// Les widgets sondés d'un deck, et le processus de rendu qui les rend.
///
/// Le registre est la base surchargée par les widgets valides de son pack ;
/// sans pack à widgets valides, la base seule.
#[derive(Debug)]
pub struct ProbedWidgets {
    /// Le registre qui valide le deck et ses sources.
    pub registry: Arc<WidgetRegistry>,
    /// Fichiers du pack toujours non sondés après la relance (budget épuisé, lien indisponible).
    pub unprobed: BTreeSet<WidgetFile>,
    /// L'erreur localisée d'une sonde du pack tout entière échouée.
    ///
    /// Le registre est alors la base.
    pub probe_failed: Option<String>,
    worker: RenderWorker,
    pack: Option<PackWidgets>,
}

impl ProbedWidgets {
    /// Sonde le pack que `deck` référence, pris dans `packs`.
    ///
    /// Sans référence, ou sans widgets : la base. Version absente de `packs`
    /// ou pack cassé : journalisés, la base, comme dans l'app ; une sonde
    /// échouée garde son erreur ([`Self::probe_failed`]).
    pub fn probe(
        platform: &Platform,
        base: &Arc<WidgetRegistry>,
        deck: &Deck,
        packs: &BTreeMap<PackVersion, PackFiles>,
    ) -> Self {
        let l: &Localizer = &platform.localizer;
        let worker = RenderWorker::new(platform.render_launch.clone(), platform.settings.render.clone());
        let base_only =
            |worker| Self { registry: base.clone(), unprobed: BTreeSet::new(), probe_failed: None, worker, pack: None };
        let Some(r) = &deck.templates else { return base_only(worker) };
        let Some(files) = packs.get(&r.version) else {
            tracing::warn!(pack = %r.id, version = %r.version, "{}", l.tr("session-pack-missing"));
            return base_only(worker);
        };
        if files.widgets().is_empty() {
            return base_only(worker);
        }
        let (start, total) = (Instant::now(), platform.settings.render.probe_budget().saturating_mul(PROBE_BUDGETS));
        let probed = PackWidgets::new(r.version.hex(), files.widgets().clone()).and_then(|pack| {
            probe_rounds(|| worker.probe_pack(&pack), |p| p.unprobed.len(), || start.elapsed() >= total)
        });
        let probe = match probed {
            Ok(probe) => probe,
            Err(e) => {
                let message = e.localize(l);
                tracing::warn!(pack = %r.id, error = %one_line(&e), "{message}");
                return Self { probe_failed: Some(message), ..base_only(worker) };
            }
        };
        for (name, file) in probe.files.iter().filter(|(_, p)| !p.is_valid()) {
            for d in &file.diagnostics {
                tracing::warn!(pack = %r.id, widget = %name, "{}", d.localize(l));
            }
        }
        match probe.pack {
            Some(pack) => Self {
                registry: probe.registry,
                unprobed: probe.unprobed,
                probe_failed: None,
                worker,
                pack: Some(pack),
            },
            None => Self { unprobed: probe.unprobed, ..base_only(worker) },
        }
    }

    /// Le pack a-t-il des widgets valides, que seul le processus de rendu exécute ?
    pub fn has_pack(&self) -> bool {
        self.pack.is_some()
    }

    /// Un emplacement du processus de rendu, avec ces widgets et les polices `fonts`.
    pub fn slot(&self, fonts: &Fonts, l: &Localizer) -> Result<RenderSlot, RenderError> {
        let mut slot = self.worker.slot(fonts, l, self.registry.clone())?;
        slot.set_widgets(self.pack.as_ref(), self.registry.clone())?;
        Ok(slot)
    }
}

/// Les sources de `deck` avec son pack `packs`, et ses widgets sondés.
///
/// Sans pack référencé, le deck adopte la version courante du pack par défaut,
/// modifiable : elle peut avoir des widgets. Le pack est sondé avant toute
/// écriture ; les sources sont recréées avec le registre sondé, pour que le
/// replica et la validation aient ses widgets.
pub fn sources_with_pack(
    platform: &Platform,
    library: &PackLibrary,
    base: &Arc<WidgetRegistry>,
    deck: Deck,
    packs: BTreeMap<PackVersion, PackFiles>,
) -> Result<(SourceBundle, ProbedWidgets)> {
    let mut bundle = SourceBundle::new(deck, base.clone()).context(SourcesSnafu)?;
    for files in packs.into_values() {
        bundle.add_pack(files);
    }
    adopt_default_pack(library, &mut bundle).context(LibrarySnafu)?;
    let widgets = ProbedWidgets::probe(platform, base, bundle.deck(), &bundle.packs);
    if !Arc::ptr_eq(&widgets.registry, base) {
        let packs = std::mem::take(&mut bundle.packs);
        bundle = SourceBundle::new(bundle.deck().clone(), widgets.registry.clone()).context(SourcesSnafu)?;
        bundle.packs = packs;
    }
    Ok((bundle, widgets))
}

#[cfg(test)]
#[path = "probed_widgets_tests.rs"]
mod probed_widgets_tests;
