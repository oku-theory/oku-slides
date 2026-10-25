//! Les bornes de l'ajustement d'un import hi-fi par l'agent.
//!
//! Les outils les tiennent ; chaque slide a un état dans la file.
//!
//! Une slide est notée par `import_diff` (slide entière) ou `import_report`
//! (chaque slide dont la page a changé). Chaque note porte une empreinte
//! ([`Seen`]) : celle du raster noté (la page Oku rendue à la résolution de
//! la note, pas l'empreinte Typst de la page, qui suit aussi l'introspection
//! des autres pages), celle de l'état du deck entier (le contenu de son thème
//! et de son pack, pas leurs versions, qui changent à chaque enregistrement ;
//! ses jetons, paramètres et variables) et celle du modèle propre de la slide
//! (son widget et ses paramètres, éléments compris). Ce que coûte une note
//! ([`Charge`]) :
//!
//! - la première note d'une slide est sa base, gratuite ;
//! - un raster inchangé ne coûte rien, ni un retour au raster du meilleur
//!   excès de la slide (`revert` : celui de l'import tant qu'aucun essai n'a
//!   gagné), même sans essai restant ;
//! - un raster changé par le seul deck (l'état du deck a changé depuis la
//!   note précédente de la slide, son modèle propre non : thème, widget du
//!   pack…) puise dans le budget global ([`TuneLimits::global`]) : un essai
//!   par état du deck jamais noté, quel que soit le nombre de slides qu'il
//!   change ; revenir à un état déjà noté (celui de l'import, ou un déjà
//!   payé) est gratuit ; aucun essai de la slide n'est compté ;
//! - sinon, ou une fois le budget global épuisé, c'est un essai de la slide :
//!   une slide retouchée paie son essai même si l'état du deck a aussi changé
//!   (aller et retour entre deux états payés ne rend pas ses retouches gratuites) ;
//!   sans essai restant, la note est refusée (le diff entier et le zoom sont
//!   refusés, le rapport laisse la slide à sa note d'avant, `stale`).
//!
//! Le gain se mesure sur l'excès total de la slide ([`excess`]) : la somme,
//! sur ses régions non suspectes, de ce qui dépasse le seuil. Un essai qui le
//! baisse sous le meilleur atteint est un gain ; sinon il compte « sans
//! gain ». Une slide s'arrête après [`TuneLimits::tries`] essais (`exhausted`)
//! ou [`TuneLimits::stall`] essais de suite sans gain (`stalled`). Seule une
//! slide `pending` a des essais ; `done`, `witness` et `unscored` n'en ont
//! aucun. Une slide n'est `witness` (seules des régions suspectes au-delà du
//! seuil) que telle qu'importée : son dernier raster noté est celui de sa
//! base ; changée depuis, la même note la dit `done`. La moyenne de départ
//! (`mean_before`, au centième) est celle des premières notes des slides, le
//! deck tel qu'importé : les budgets comme elle valent pour
//! toute la vie de l'import dans l'onglet (la session ne voit pas les tours
//! de l'agent ; le tour d'ajustement suit l'import). Une slide est `worse`
//! quand son excès courant dépasse le meilleur qu'elle a atteint.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use sha2::{Digest, Sha256};

use oku_slides_import::metrics::SlideScore;
use oku_slides_platform::config::ImportSettings;

/// Les bornes de l'ajustement, lues de `[import]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuneLimits {
    /// Essais par slide (`tune_iterations`).
    pub tries: u32,
    /// Essais de suite sans gain avant d'arrêter la slide (`tune_stall`).
    pub stall: u32,
    /// Essais du deck entier, pour les changements de thème ou de pack (`tune_global`).
    pub global: u32,
}

impl TuneLimits {
    /// Les bornes des réglages `settings`.
    #[must_use]
    pub fn of(settings: &ImportSettings) -> Self {
        Self {
            tries: settings.tune_iterations.get(),
            stall: settings.tune_stall.get(),
            global: settings.tune_global.get(),
        }
    }
}

/// L'excès de `score`.
///
/// La somme, sur ses régions non suspectes, de ce qui dépasse `threshold`.
///
/// `None` sans région non suspecte : la slide n'a rien à corriger.
pub(crate) fn excess(score: &SlideScore, threshold: f64) -> Option<f64> {
    let mut regions = score.regions.iter().filter(|r| r.suspect.is_none()).peekable();
    regions.peek()?;
    Some(regions.map(|r| (r.score - threshold).max(0.0)).sum())
}

/// `v` arrondi au centième, comme [`oku_slides_import::ImportReport::mean_score`].
pub(crate) fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Les 128 premiers bits du SHA-256 des morceaux `parts`, à la suite.
pub(crate) fn fingerprint<'a>(parts: impl IntoIterator<Item = &'a [u8]>) -> u128 {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    let digest = hasher.finalize();
    let mut first = [0u8; 16];
    first.copy_from_slice(&digest[..16]);
    u128::from_be_bytes(first)
}

/// Ce qu'une note a vu.
///
/// Les empreintes du raster noté, de l'état du deck et du modèle de la slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    /// L'empreinte du raster de la page Oku notée ([`fingerprint`] de ses dimensions et pixels).
    pub raster: u128,
    /// L'empreinte de l'état du deck entier.
    ///
    /// Contenus du thème et du pack, jetons, paramètres, variables.
    pub deck: u128,
    /// L'empreinte du modèle propre de la slide (son widget et ses paramètres, éléments compris).
    pub slide: u128,
}

/// Ce que coûte une note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Charge {
    /// La première note de la slide : gratuite.
    Baseline,
    /// Le raster n'a pas changé : gratuite.
    Unchanged,
    /// Un changement du seul deck, la slide intacte.
    ///
    /// Le budget global (un essai par état du deck jamais noté).
    Global,
    /// Un essai de la slide.
    Slide,
    /// Retour au raster du meilleur excès de la slide : gratuite.
    Revert,
    /// Plus d'essai : la note est refusée, rien ne change.
    Refused,
}

/// Les essais d'ajustement d'une slide par l'agent.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tuning {
    /// Essais faits.
    pub tries: u32,
    /// Le plus petit excès vu ([`excess`]).
    ///
    /// `None` sans région non suspecte, ou avant la première note.
    pub best: Option<f64>,
    /// Essais de suite sans gain sur `best`.
    pub without_gain: u32,
    /// Ce qu'a vu la dernière note ; `None` avant la première.
    pub seen: Option<Seen>,
    /// Le score de la première note : la slide telle qu'importée.
    pub first: Option<f64>,
    /// L'empreinte du raster de la première note : la slide telle qu'importée.
    pub imported: Option<u128>,
    /// L'empreinte du raster de la note qui a atteint `best`.
    pub best_raster: Option<u128>,
}

impl Tuning {
    /// Vrai si le dernier raster noté n'est plus celui de l'import.
    #[must_use]
    pub fn changed(&self) -> bool {
        matches!((self.imported, self.seen), (Some(imported), Some(seen)) if seen.raster != imported)
    }
}

/// Les essais de toutes les slides et le budget global d'un import.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuner {
    limits: TuneLimits,
    slides: BTreeMap<String, Tuning>,
    /// Essais globaux faits.
    global_used: u32,
    /// Les états du deck notés sans frais.
    ///
    /// Celui de la première note, puis chacun payé.
    paid: BTreeSet<u128>,
}

impl Tuner {
    /// Aucun essai fait, aux bornes `limits`.
    #[must_use]
    pub fn new(limits: TuneLimits) -> Self {
        Self { limits, slides: BTreeMap::new(), global_used: 0, paid: BTreeSet::new() }
    }

    /// Les bornes de l'ajustement.
    #[must_use]
    pub fn limits(&self) -> TuneLimits {
        self.limits
    }

    /// Les essais de la slide `slide`, si elle a été notée.
    #[must_use]
    pub fn tuning(&self, slide: &str) -> Option<&Tuning> {
        self.slides.get(slide)
    }

    /// Les essais globaux qui restent.
    #[must_use]
    pub fn global_tries_left(&self) -> u32 {
        self.limits.global.saturating_sub(self.global_used)
    }

    /// Ce que coûterait une note de `slide`, dans l'état `state`, qui voit `now`.
    ///
    /// [`Charge::Global`] seulement si le modèle de la slide n'a pas changé depuis sa note précédente.
    #[must_use]
    pub fn charge(&self, slide: &str, now: Seen, state: SlideState) -> Charge {
        let tuning = self.slides.get(slide);
        let Some(seen) = tuning.and_then(|t| t.seen) else { return Charge::Baseline };
        if seen.raster == now.raster {
            Charge::Unchanged
        } else if tuning.and_then(|t| t.best_raster) == Some(now.raster) {
            Charge::Revert
        } else if seen.deck != now.deck
            && seen.slide == now.slide
            && (self.paid.contains(&now.deck) || self.global_tries_left() > 0)
        {
            Charge::Global
        } else if state.tries_left(tuning, self.limits) > 0 {
            Charge::Slide
        } else {
            Charge::Refused
        }
    }

    /// Garde la note `score` de `slide`, qui a vu `now` et coûté `charge`.
    ///
    /// Voir [`Self::charge`].
    pub fn record(&mut self, slide: &str, now: Seen, charge: Charge, score: &SlideScore, threshold: f64) {
        let excess = excess(score, threshold);
        let first = self.paid.is_empty();
        if charge == Charge::Global && self.paid.insert(now.deck) {
            self.global_used += 1;
        }
        if charge == Charge::Baseline && first {
            self.paid.insert(now.deck);
        }
        let t = self.slides.entry(slide.to_owned()).or_default();
        // Un excès sous le meilleur atteint (ou le premier) : le raster noté devient celui du meilleur.
        let gain = match (t.best, excess) {
            (Some(best), Some(e)) => e < best,
            (None, e) => e.is_some(),
            (Some(_), None) => false,
        };
        match charge {
            Charge::Refused => return,
            Charge::Baseline => {
                *t = Tuning {
                    best: excess,
                    first: Some(score.score),
                    imported: Some(now.raster),
                    best_raster: Some(now.raster),
                    ..Tuning::default()
                };
            }
            Charge::Unchanged | Charge::Global | Charge::Revert => {}
            Charge::Slide => {
                t.tries += 1;
                match (t.best, excess) {
                    (Some(best), Some(e)) if e >= best => t.without_gain += 1,
                    (Some(_), Some(_)) => t.without_gain = 0,
                    _ => {}
                }
            }
        }
        if gain && charge != Charge::Baseline {
            t.best = excess;
            t.best_raster = Some(now.raster);
        }
        t.seen = Some(now);
    }

    /// La page de `slide` n'a pas changé.
    ///
    /// Sa dernière note voit l'état du deck `deck` et le modèle `model`.
    pub fn touch(&mut self, slide: &str, deck: u128, model: u128) {
        if let Some(seen) = self.slides.get_mut(slide).and_then(|t| t.seen.as_mut()) {
            seen.deck = deck;
            seen.slide = model;
        }
    }

    /// Vrai si l'excès de `score` dépasse le meilleur que `slide` a atteint.
    #[must_use]
    pub fn worse(&self, slide: &str, score: &SlideScore, threshold: f64) -> bool {
        let best = self.slides.get(slide).and_then(|t| t.best);
        matches!((best, excess(score, threshold)), (Some(best), Some(e)) if e > best)
    }

    /// La moyenne des premières notes de `slides`, au centième ; `None` sans aucune note.
    pub fn mean_before<'a>(&self, slides: impl IntoIterator<Item = &'a str>) -> Option<f64> {
        let firsts: Vec<f64> = slides.into_iter().filter_map(|s| self.slides.get(s)?.first).collect();
        (!firsts.is_empty()).then(|| round(firsts.iter().sum::<f64>() / oku_slides_core::num::count_f64(firsts.len())))
    }
}

/// L'état d'une slide dans la file de l'ajustement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlideState {
    /// Une région non suspecte dépasse le seuil ; il reste des essais.
    Pending,
    /// Aucune région non suspecte ne dépasse le seuil.
    ///
    /// Une région suspecte le peut si la slide a changé depuis l'import (voir `Witness`).
    Done,
    /// Toujours au-delà du seuil, tous ses essais faits.
    Exhausted,
    /// Toujours au-delà du seuil, sans gain depuis [`TuneLimits::stall`] essais.
    Stalled,
    /// Telle qu'importée, seules des régions suspectes (erreurs du témoin) dépassent le seuil.
    ///
    /// Rien à corriger. Une slide changée depuis l'import, dans ce cas, est `Done`.
    Witness,
    /// Pas de score : le rendu de référence manque.
    Unscored,
}

impl SlideState {
    /// L'état d'une slide notée `score`, aux essais `tuning`, contre le seuil `threshold`.
    pub fn of(score: Option<&SlideScore>, tuning: Option<&Tuning>, threshold: f64, limits: TuneLimits) -> Self {
        let Some(score) = score else { return Self::Unscored };
        let (tries, without_gain) = tuning.map_or((0, 0), |t| (t.tries, t.without_gain));
        if excess(score, threshold).is_some_and(|e| e > 0.0) {
            if without_gain >= limits.stall {
                Self::Stalled
            } else if tries >= limits.tries {
                Self::Exhausted
            } else {
                Self::Pending
            }
        } else if !tuning.is_some_and(Tuning::changed)
            && score.regions.iter().any(|r| r.suspect.is_some() && r.score > threshold)
        {
            Self::Witness
        } else {
            Self::Done
        }
    }

    /// Les essais qui restent dans cet état : seule une slide `pending` en a.
    #[must_use]
    pub fn tries_left(self, tuning: Option<&Tuning>, limits: TuneLimits) -> u32 {
        match self {
            Self::Pending => limits.tries.saturating_sub(tuning.map_or(0, |t| t.tries)),
            _ => 0,
        }
    }

    /// Le nom de l'état, tel que l'écrit le JSON des outils.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Done => "done",
            Self::Exhausted => "exhausted",
            Self::Stalled => "stalled",
            Self::Witness => "witness",
            Self::Unscored => "unscored",
        }
    }
}

#[cfg(test)]
#[path = "tune_tests.rs"]
mod tune_tests;
