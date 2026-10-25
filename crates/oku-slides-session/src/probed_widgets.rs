//! Les widgets d'un deck hors session.
//!
//! Ses packs embarqués, sondés par le processus de rendu ([`ProbedWidgets::probe`]). Chaque fichier
//! est sondé à part, les widgets cassés écartés (diagnostics au journal) ; le registre sondé valide
//! le deck (t-355). Comme dans la session, un pack est écarté seul (version absente, épingle qui
//! ne dit pas son manifeste, sonde échouée), avec ceux qui le requièrent ; un assemblage raté du
//! registre n'écarte que les packs dont l'ajout le fait échouer ([`assemble_each`]), sauf avec un
//! pack de confiance (`base`, `essentiel` livrés) : l'erreur remonte alors, rien n'est réduit.
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

use oku_slides_core::{Deck, Namespace, PackFiles, PackPin, PackVersion, SourceBundle, WidgetFile, WidgetRegistry};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_platform::errors::one_line;
use oku_slides_render::{Fonts, PackSet, PackWidgets, RegistryError, RenderError, is_trusted_pack};
use oku_slides_render_worker::{RenderSlot, RenderWorker};

use crate::deck_closure::{combined_pins, dependents};
use crate::deck_pack::{check_pack_id, in_process_pack, only_builtins};
use crate::import::error::{LibrarySnafu, RegistrySnafu, Result, SourcesSnafu};
use crate::{PackLibrary, adopt_default_packs};

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

/// `packs` rangés besoins d'abord (`requires` de `pins`) ; une boucle garde l'ordre donné.
fn requirements_first(pins: &BTreeMap<Namespace, PackPin>, mut packs: Vec<PackWidgets>) -> Vec<PackWidgets> {
    let mut out = Vec::new();
    while !packs.is_empty() {
        let waiting: BTreeSet<Namespace> = packs.iter().map(|p| p.namespace().clone()).collect();
        let ready = |p: &PackWidgets| {
            pins.get(p.namespace())
                .is_none_or(|pin| pin.requires.iter().all(|r| r == p.namespace() || !waiting.contains(r)))
        };
        let (now, rest): (Vec<_>, Vec<_>) = packs.into_iter().partition(|p| ready(p));
        if now.is_empty() {
            out.extend(rest);
            break;
        }
        out.extend(now);
        packs = rest;
    }
    out
}

/// L'ensemble de `packs` que `assemble` sait assembler, son registre et la première erreur.
///
/// Tous ensemble d'abord ; si l'assemblage échoue, pack par pack, besoins
/// d'abord : un pack dont l'ajout le fait échouer est écarté (journalisé),
/// avec ceux qui le requièrent. Les autres restent : jamais tous ensemble
/// pour un seul. Registre `None` : aucun pack gardé.
fn assemble_each<R>(
    pins: &BTreeMap<Namespace, PackPin>,
    packs: Vec<PackWidgets>,
    mut assemble: impl FnMut(&PackSet) -> Result<R, RegistryError>,
) -> (PackSet, Option<R>, Option<RegistryError>) {
    let first = match PackSet::new(packs.clone()).and_then(|set| assemble(&set).map(|r| (set, r))) {
        Ok((set, registry)) => return (set, Some(registry), None),
        Err(e) => e,
    };
    tracing::warn!(error = %one_line(&first), "registre des packs non assemblé : pack par pack");
    let (mut kept, mut last, mut dropped) = (Vec::new(), None, BTreeSet::new());
    for pack in requirements_first(pins, packs) {
        let ns = pack.namespace().clone();
        let needs_dropped = pins.get(&ns).is_some_and(|pin| pin.requires.iter().any(|r| dropped.contains(r)));
        let mut trial = kept.clone();
        trial.push(pack.clone());
        let assembled = if needs_dropped {
            None
        } else {
            match PackSet::new(trial).and_then(|set| assemble(&set).map(|r| (set, r))) {
                Ok(ok) => Some(ok),
                Err(e) => {
                    tracing::warn!(pack = %ns, error = %one_line(&e), "pack non assemblé : écarté");
                    None
                }
            }
        };
        match assembled {
            Some(ok) => {
                kept.push(pack);
                last = Some(ok);
            }
            None => {
                dropped.insert(ns);
            }
        }
    }
    match last {
        Some((set, registry)) => (set, Some(registry), Some(first)),
        None => (PackSet::default(), None, Some(first)),
    }
}

/// Les widgets sondés d'un deck, et le processus de rendu qui les rend.
///
/// Le registre est fait des widgets valides de chacun de ses packs ;
/// sans pack à widgets valides, le registre de base donné.
#[derive(Debug)]
pub struct ProbedWidgets {
    /// Le registre qui valide le deck et ses sources.
    pub registry: Arc<WidgetRegistry>,
    /// Par pack, les fichiers toujours non sondés après la relance (budget épuisé, lien indisponible).
    pub unprobed: BTreeMap<Namespace, BTreeSet<WidgetFile>>,
    /// L'erreur localisée du premier pack écarté pour une panne : épingle qui
    /// ne dit pas son manifeste, sonde tout entière échouée, assemblage raté.
    ///
    /// Ce pack est alors écarté du registre, avec ceux qui le requièrent.
    pub probe_failed: Option<String>,
    worker: RenderWorker,
    set: PackSet,
}

impl ProbedWidgets {
    /// Probes each pack `deck` pins (widgets and layouts), taken from `packs`.
    ///
    /// Without a pack, or without widgets: the base. A version missing from
    /// `packs`, an inconsistent pin or a broken pack: logged, and the pack
    /// dropped along with those that require it, as in the app; the first
    /// failure keeps its error ([`Self::probe_failed`]). A failed assembly
    /// drops only the packs that make it fail ([`assemble_each`]). The retry
    /// delay is shared by the packs.
    ///
    /// # Errors
    ///
    /// A shipped (trusted) built-in pack is never dropped: if unreadable or
    /// its probe failed, its error is returned, like the failed assembly of a set that holds one.
    pub fn probe(
        platform: &Platform,
        base: &Arc<WidgetRegistry>,
        deck: &Deck,
        packs: &BTreeMap<PackVersion, PackFiles>,
    ) -> Result<Self, RegistryError> {
        let l: &Localizer = &platform.localizer;
        let worker = RenderWorker::new(platform.render_launch.clone(), platform.settings.render.clone());
        let (start, total) = (Instant::now(), platform.settings.render.probe_budget().saturating_mul(PROBE_BUDGETS));
        let (mut usable, mut unprobed, mut probe_failed, mut off) =
            (Vec::new(), BTreeMap::new(), None, BTreeSet::new());
        let pins = combined_pins(deck);
        for (ns, pin) in &pins {
            let Some(files) = packs.get(&pin.version) else {
                tracing::warn!(pack = %pin.id, version = %pin.version, "version du pack absente des sources : écarté");
                off.insert(ns.clone());
                continue;
            };
            if let Err(e) = check_pack_id(ns, pin, files) {
                let message = e.localize(l);
                tracing::warn!(pack = %pin.id, error = %e.one_line(), "{message}");
                probe_failed.get_or_insert(message);
                off.insert(ns.clone());
                continue;
            }
            if files.widgets().is_empty() {
                continue;
            }
            // Un pack intégré courant se résout ici, sans sonde, comme dans la session.
            // De confiance, il n'est jamais écarté : illisible, l'erreur remonte (arbitrage 94).
            if let Some(builtin) = in_process_pack(&pin.version) {
                usable.push(builtin?);
                continue;
            }
            let pack = PackWidgets::from_files(ns.clone(), files);
            let trusted = pack.as_ref().is_ok_and(is_trusted_pack);
            let probed = pack.and_then(|pack| {
                probe_rounds(|| worker.probe_pack(&pack), |p| p.unprobed.len(), || start.elapsed() >= total)
            });
            let probe = match probed {
                Ok(probe) => probe,
                Err(e) => {
                    let e = disable_unless_trusted(trusted, e)?;
                    let message = e.localize(l);
                    tracing::warn!(pack = %pin.id, error = %one_line(&e), "{message}");
                    probe_failed.get_or_insert(message);
                    off.insert(ns.clone());
                    continue;
                }
            };
            for (name, file) in probe.files.iter().filter(|(_, p)| !p.is_valid()) {
                for d in &file.diagnostics {
                    tracing::warn!(pack = %pin.id, widget = %name, "{}", d.localize(l));
                }
            }
            if !probe.unprobed.is_empty() {
                unprobed.insert(ns.clone(), probe.unprobed);
            }
            usable.extend(probe.pack);
        }
        let gone = dependents(&pins, &off);
        for ns in &gone {
            tracing::warn!(pack = %ns, "pack qui requiert un pack écarté : écarté");
        }
        usable.retain(|p| !gone.contains(p.namespace()));
        if usable.is_empty() {
            return Ok(Self { registry: base.clone(), unprobed, probe_failed, worker, set: PackSet::default() });
        }
        // Les seuls packs intégrés : leur registre est celui de l'app, sans processus de rendu. Les
        // schémas ne dépendent pas des polices : le registre intégré garde celles de son premier appel.
        let registry_of = |set: &PackSet| {
            if only_builtins(set) {
                oku_slides_render::builtin_registry(&Fonts::default())
            } else {
                worker.registry_of(set)
            }
        };
        // Avec un pack de confiance, un assemblage raté remonte : l'ensemble n'est jamais réduit (t-1057).
        if usable.iter().any(is_trusted_pack) {
            let set = PackSet::new(usable)?;
            let registry = registry_of(&set)?;
            return Ok(Self { registry, unprobed, probe_failed, worker, set });
        }
        let (set, registry, error) = assemble_each(&pins, usable, registry_of);
        if let Some(e) = error {
            let message = e.localize(l);
            tracing::warn!(error = %one_line(&e), "{message}");
            probe_failed.get_or_insert(message);
        }
        let registry = registry.unwrap_or_else(|| base.clone());
        Ok(Self { registry, unprobed, probe_failed, worker, set })
    }

    /// Un pack a-t-il des widgets valides, que seul le processus de rendu exécute ?
    #[must_use]
    pub fn has_packs(&self) -> bool {
        !self.set.is_empty()
    }

    /// A slot of the render process, with these widgets and fonts `fonts`.
    ///
    /// # Errors
    ///
    /// Fails if the render process gives no slot, or rejects these widgets.
    pub fn slot(&self, fonts: &Fonts, l: &Localizer) -> Result<RenderSlot, RenderError> {
        let mut slot = self.worker.slot(fonts, l, self.registry.clone())?;
        slot.set_widgets(&self.set, self.registry.clone())?;
        Ok(slot)
    }
}

/// La panne d'un pack, à écarter ; de confiance (pack intégré livré), elle remonte plutôt.
fn disable_unless_trusted(trusted: bool, e: RegistryError) -> Result<RegistryError, RegistryError> {
    if trusted { Err(e) } else { Ok(e) }
}

/// The sources of `deck` with its packs `packs`, and its probed widgets.
///
/// Without a pinned pack, the deck adopts the current version of `base` and `essentiel`
/// and their requirements, which can be changed: they may have widgets. The packs
/// are probed before any write; the sources are recreated with the probed
/// registry, so that the replica and the validation have their widgets.
///
/// # Errors
///
/// Fails if the default packs are missing, if a pack cannot be probed, or if the sources cannot be recreated.
pub fn sources_with_packs(
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
    adopt_default_packs(library, &mut bundle).context(LibrarySnafu)?;
    let widgets = ProbedWidgets::probe(platform, base, bundle.deck(), &bundle.packs).context(RegistrySnafu)?;
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
