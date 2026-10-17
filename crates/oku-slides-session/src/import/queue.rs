//! L'ajustement d'un import par l'agent.
//!
//! Le rapport et la file des slides (`import_report`), servis par la session de l'onglet importé.
//!
//! Hi-fi : chaque slide dont la page Oku a changé depuis sa dernière note
//! (toutes au premier appel) est notée de nouveau sur le rendu courant, à ce
//! que coûte sa note ([`super::tune`] : un essai de la slide, ou un essai
//! global pour un changement du seul deck, la slide intacte) ; une slide
//! sans essai restant garde sa note d'avant, est nommée dans `stale` et
//! marquée `stale` dans la file. Une modification du thème, d'un widget du
//! pack ou d'une slide se voit donc au rapport suivant, qui donne la moyenne
//! (`mean`) et la moyenne de départ (`mean_before`, les mêmes slides telles
//! qu'importées), toutes deux au centième et sans les slides `stale` ou
//! `unscored` de ce rapport, et nomme les autres slides dont l'excès dépasse
//! le meilleur atteint (`worse`). Une
//! slide supprimée depuis l'import (ou un import annulé) n'a plus de page :
//! elle sort de la file, de la moyenne et du compte des `done`. Une slide
//! qu'on ne peut noter (rendu ou comparaison en échec) est journalisée et
//! dite `unscored` dans ce rapport, sans arrêter les autres ; les notes ne
//! sont gardées qu'après le rendu de toutes les pages changées. Le JSON reste
//! court : une file des slides pas encore `done`, au plus [`LISTED`] éléments
//! par liste annexe, le reste compté.
//!
//! La référence se fait hors du verrou ([`Session::import_reference_job`]) :
//! le rapport n'appelle jamais le témoin ; tant qu'elle manque, il n'a pas de score.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};
use snafu::IntoError;

use oku_slides_core::PackManifest;
use oku_slides_core::tokens::effective_tokens;
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_import::plan::ItemTarget;
use oku_slides_import::report::{LossKind, ReferenceCause};
use oku_slides_reference::Raster;

use super::reference::ReferenceJob;
use super::sample::sample;
use super::tune::{Charge, SlideState, excess, fingerprint, round};
use super::tune_error::{CompareSnafu, NoImportSnafu, TuneError, TuneInner};
use super::{ImportMode, ImportState};
use crate::error::{EditingTemplatesSnafu, InHistorySnafu, ReadOnlySnafu, TuneSnafu};
use crate::theme_schema::default_tokens;
use crate::{Mode, Result, Session, SessionError};

/// Éléments d'une liste annexe du rapport (pertes figées, groupes, régions), au plus.
pub const LISTED: usize = 8;

/// Le refus `inner` d'un outil d'ajustement, en erreur de la session.
pub(crate) fn refused(inner: TuneInner) -> SessionError {
    TuneSnafu.into_error(TuneError::from(inner)).into()
}

/// L'échec d'une comparaison : son détail au journal, un refus `Compare` à l'agent.
pub(crate) fn compare_failed((cause, detail): (ReferenceCause, String)) -> SessionError {
    tracing::warn!(%cause, error = %detail, "comparaison à la référence impossible");
    refused(CompareSnafu { detail }.build())
}

/// Les ids d'éléments de `target`, vides pour un paramètre de slide.
pub(crate) fn element_ids(target: Option<&ItemTarget>) -> &[String] {
    match target {
        Some(ItemTarget::Elements(ids)) => ids,
        _ => &[],
    }
}

impl Session {
    /// L'import de l'onglet, sur le deck montré ; sinon le refus qui convient.
    pub(crate) fn tuned_import(&mut self) -> Result<&mut ImportState> {
        match self.mode {
            Mode::Deck => {}
            Mode::Templates { .. } => return EditingTemplatesSnafu.fail()?,
            Mode::History { .. } => return InHistorySnafu.fail()?,
            Mode::Help { .. } => return ReadOnlySnafu.fail()?,
        }
        self.import.as_mut().ok_or_else(|| refused(NoImportSnafu.build()))
    }

    /// Le rendu de référence manquant, à faire hors du verrou de la session.
    ///
    /// L'appelant le lance ([`ReferenceJob::run`]) avant `import_report` ou
    /// `import_diff` ; le travail remplit l'état de cet import, et lui seul.
    /// `None` en retrofit, sans témoin, ou s'il est déjà fait ; sans import
    /// dans l'onglet, le refus `NoImport`.
    pub fn import_reference_job(&mut self) -> Result<Option<ReferenceJob>> {
        let state = self.tuned_import()?;
        Ok(match state.mode() {
            ImportMode::Hifi => state.reference_job(),
            ImportMode::Retrofit => None,
        })
    }

    /// Le rapport de l'import et la file de l'ajustement, en JSON pour l'agent.
    ///
    /// Hi-fi : les slides dont la page a changé depuis leur dernière note sont
    /// notées de nouveau, chacune à ce que coûte sa note ([`super::tune`]),
    /// contre la référence faite par [`Self::import_reference_job`] ; tant
    /// qu'elle manque, aucune slide n'a de score. `l` : la langue de l'agent,
    /// celle d'une référence manquante.
    pub fn import_report(&mut self, l: &Localizer) -> Result<Value> {
        self.tuned_import()?;
        self.ensure_rendered()?;
        let live = self.live_slides();
        let rescored = match self.import.as_ref().map(ImportState::mode) {
            Some(ImportMode::Hifi) => Some(self.rescore_changed(&live)?),
            _ => None,
        };
        let state = self.tuned_import()?;
        let mut v = common_json(state);
        let extra = match (state.mode(), rescored) {
            (ImportMode::Hifi, Some(rescored)) => {
                let live: BTreeSet<&str> = live.iter().map(|(slide, _)| slide.as_str()).collect();
                hifi_json(state, &live, &rescored, l)
            }
            _ => retrofit_json(state),
        };
        if let (Some(o), Value::Object(extra)) = (v.as_object_mut(), extra) {
            o.extend(extra);
        }
        Ok(v)
    }

    /// Les slides de l'import qui ont encore une page, et l'indice de cette page.
    fn live_slides(&self) -> Vec<(String, usize)> {
        let Some(state) = self.import.as_ref() else { return Vec::new() };
        let slides = state.report().slides.iter();
        slides.filter_map(|r| Some((r.slide.clone(), self.page_of(&r.slide)?.checked_sub(1)?))).collect()
    }

    /// L'empreinte de l'état du deck entier.
    ///
    /// Contenus du thème et du pack, jetons, paramètres, variables.
    ///
    /// Les contenus, pas les versions : une version porte aussi son parent et
    /// sa date, si bien que rendre au thème ou à un widget du pack son contenu
    /// d'avant revient à un état déjà noté. Le thème compte par ses jetons
    /// effectifs, le pack par ses modèles, ses exemples et ses fichiers de
    /// widgets. Une page changée alors que cette empreinte a changé, et pas le
    /// modèle de la slide ([`Self::slide_key`]), puise dans le budget global
    /// ([`super::tune`]).
    pub(crate) fn deck_key(&self) -> Result<u128> {
        let failed = |what: String| compare_failed((ReferenceCause::Failed, what));
        let deck = self.deck();
        // Les jetons du thème tels que le rendu les lit (le thème par défaut pour un deck sans thème).
        let theme = self.theme_tokens(deck).map(|tokens| effective_tokens(default_tokens(), &tokens));
        let pack = match &deck.templates {
            Some(r) => {
                let files = self.doc.sources.packs.get(&r.version);
                let files = files.ok_or_else(|| failed(format!("pack {} absent", r.id)))?;
                let m = PackManifest::from_files(files).map_err(|e| failed(format!("pack {} : {e}", r.id)))?;
                Some((m.templates, m.examples, files.widgets()))
            }
            None => None,
        };
        let state = (&theme, &pack, &deck.tokens, &deck.params, &deck.vars);
        let bytes = serde_json::to_vec(&state).map_err(|e| failed(format!("état du deck : {e}")))?;
        Ok(fingerprint([bytes.as_slice()]))
    }

    /// L'empreinte du modèle propre de la slide `slide`.
    ///
    /// Son widget et ses paramètres, éléments compris.
    ///
    /// Une page changée alors que ce modèle a changé coûte un essai de la slide ([`super::tune`]).
    pub(crate) fn slide_key(&self, slide: &str) -> Result<u128> {
        let failed = |what: String| compare_failed((ReferenceCause::Failed, what));
        let model = self.deck().slide(slide).ok_or_else(|| failed(format!("slide {slide} absente du deck")))?;
        let bytes = serde_json::to_vec(&(&model.kind, &model.params))
            .map_err(|e| failed(format!("modèle de la slide {slide} : {e}")))?;
        Ok(fingerprint([bytes.as_slice()]))
    }

    /// Note de nouveau chaque slide de `live` dont la page Oku a changé.
    ///
    /// Toutes les pages changées sont rendues d'abord, sans rien garder ; les
    /// notes ne sont gardées qu'ensuite. Une slide qu'on ne peut rendre ou
    /// noter est journalisée et laissée à sa note d'avant, sans arrêter les autres.
    fn rescore_changed(&mut self, live: &[(String, usize)]) -> Result<Rescored> {
        let _span = tracing::debug_span!("rescore_changed", slides = live.len()).entered();
        let state = self.tuned_import()?;
        state.settle();
        if !matches!(state.reference_status(), Some(Ok(_))) {
            return Ok(Rescored::default());
        }
        let ppp = state.pixel_per_pt();
        let deck = self.deck_key()?;
        let (mut unchanged, mut rendered, mut out) = (Vec::new(), Vec::new(), Rescored::default());
        for (slide, page) in live {
            let model = self.slide_key(slide)?;
            let hash = self.compiled.as_ref().and_then(|c| c.page_hash(*page));
            if hash.is_some() && hash == self.import.as_ref().and_then(|s| s.scored_page(slide)) {
                // Même empreinte Typst, même raster : rien à noter, l'état du deck passe.
                unchanged.push((slide.as_str(), model));
                continue;
            }
            match self.with_compiled(|c| Ok((c.png(*page, ppp)?, c.page_hash(*page)))) {
                Ok(png) => rendered.push((slide.as_str(), model, png)),
                Err(e) => {
                    tracing::warn!(slide = %slide, error = %e.one_line(), "page Oku non rendue : slide non notée");
                    out.unscored.insert(slide.clone());
                }
            }
        }
        let state = self.tuned_import()?;
        for (slide, model) in unchanged {
            state.touch(slide, deck, model);
        }
        for (slide, model, (png, hash)) in rendered {
            let oku = Raster::from_png(&png).map_err(|e| (ReferenceCause::Failed, format!("page Oku : {e}")));
            match oku.and_then(|oku| state.observe(slide, &oku, hash, (deck, model))) {
                Ok((Charge::Refused, _)) => out.stale.push(slide.to_owned()),
                Ok(_) => {}
                Err(failure) => {
                    state.warn(&failure, Some(slide));
                    out.unscored.insert(slide.to_owned());
                }
            }
        }
        Ok(out)
    }
}

/// Les slides que le rapport a laissées à leur note d'avant.
///
/// Soit sans essai restant (`stale`), soit sans nouvelle note (`unscored`).
#[derive(Debug, Default)]
struct Rescored {
    /// Changées, mais sans essai restant.
    stale: Vec<String>,
    /// Pas notées : rendu ou comparaison en échec (journalisé).
    unscored: BTreeSet<String>,
}

/// Les champs des deux modes.
///
/// Le mode, les pertes comptées par sorte, les items figés.
fn common_json(state: &ImportState) -> Value {
    let report = state.report();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut frozen = Vec::new();
    for slide in &report.slides {
        for loss in &slide.losses {
            *kinds.entry(loss.kind.name().to_owned()).or_default() += 1;
            if let (LossKind::Opaque(_), Some(item)) = (&loss.kind, &loss.item) {
                frozen.push(json!({ "slide": slide.slide, "item": item }));
            }
        }
    }
    let more = frozen.len().saturating_sub(LISTED);
    frozen.truncate(LISTED);
    json!({
        "mode": state.mode(),
        "losses": { "total": report.loss_count(), "by_kind": kinds },
        "frozen": frozen,
        "frozen_more": more,
    })
}

/// Le retrofit.
///
/// Les demandes de widget (motif, slides, repli employé, échantillon de sa première occurrence).
fn retrofit_json(state: &ImportState) -> Value {
    let requests: Vec<Value> = state
        .widget_requests()
        .iter()
        .map(|r| json!({ "signature": r.signature, "slides": r.slides, "fallback": r.fallback, "sample": sample(&r.sample) }))
        .collect();
    json!({ "widget_requests": requests })
}

/// Le hi-fi.
///
/// Le témoin, le seuil, la file des slides `live` (celles qui ont une page) et les groupes répétés.
fn hifi_json(state: &ImportState, live: &BTreeSet<&str>, rescored: &Rescored, l: &Localizer) -> Value {
    let report = state.report();
    let (threshold, limits, tuner) = (state.threshold(), state.limits(), state.tuner());
    let slides: Vec<_> = report.slides.iter().filter(|s| live.contains(s.slide.as_str())).collect();
    let mut queue: Vec<(Option<f64>, u32, Value)> = Vec::new();
    let (mut done, mut worse, mut current) = (0, Vec::new(), Vec::new());
    for slide in &slides {
        let tuning = state.tuning(&slide.slide);
        // Une note d'avant (slide `stale` ou `unscored` dans ce rapport) ne dit pas le deck rendu.
        let stale = rescored.stale.contains(&slide.slide);
        let unscored = rescored.unscored.contains(&slide.slide);
        let score = slide.score.as_ref();
        if !stale && !unscored {
            current.push(slide);
            if score.is_some_and(|s| tuner.worse(&slide.slide, s, threshold)) {
                worse.push(slide.slide.as_str());
            }
        }
        let slide_state = if unscored { SlideState::Unscored } else { state.slide_state(&slide.slide) };
        if slide_state == SlideState::Done && !stale {
            done += 1;
            continue;
        }
        let worst = score.and_then(oku_slides_import::metrics::SlideScore::worst);
        let mut entry = json!({
            "slide": slide.slide,
            "source": slide.source,
            "score": score.map(|s| s.score),
            "worst": worst,
            "excess": score.and_then(|s| excess(s, threshold)),
            "state": slide_state,
            "tries_left": slide_state.tries_left(tuning, limits),
        });
        if let (true, Some(o)) = (stale, entry.as_object_mut()) {
            o.insert("stale".into(), json!(true));
        }
        queue.push((worst, slide.source, entry));
    }
    // Les pires d'abord ; une slide sans score après, dans l'ordre source.
    queue.sort_by(|a, b| b.0.unwrap_or(f64::MIN).total_cmp(&a.0.unwrap_or(f64::MIN)).then(a.1.cmp(&b.1)));
    let groups: Vec<Value> = state
        .groups()
        .iter()
        .take(LISTED)
        .map(|g| {
            let occurrences: Vec<Value> = g
                .occurrences
                .iter()
                .map(|(slide, item)| {
                    json!({ "slide": slide, "item": item, "elements": element_ids(state.target(item)) })
                })
                .collect();
            json!({ "signature": g.signature, "occurrences": occurrences })
        })
        .collect();
    let mut v = json!({
        "threshold": threshold,
        "tries_per_slide": limits.tries,
        "global_tries_left": tuner.global_tries_left(),
        "mean": mean(current.iter().filter_map(|s| s.score.as_ref().map(|s| s.score))),
        "slides": queue.into_iter().map(|(_, _, v)| v).collect::<Vec<_>>(),
        "done": done,
        "groups": groups,
        "groups_more": state.groups().len().saturating_sub(LISTED),
    });
    if let Some(o) = v.as_object_mut() {
        match state.reference_status() {
            Some(Ok(witness)) => {
                o.insert("witness".into(), json!(witness));
            }
            Some(Err(cause)) => {
                o.insert("reference_unavailable".into(), json!(cause.localize(l)));
            }
            None => {}
        }
        // Une première note faite : la moyenne de départ, celle des slides de `mean`.
        if tuner.mean_before(slides.iter().map(|s| s.slide.as_str())).is_some() {
            let before = tuner.mean_before(current.iter().map(|s| s.slide.as_str()));
            o.insert("mean_before".into(), json!(before));
            o.insert("worse".into(), json!(worse));
            o.insert("stale".into(), json!(rescored.stale));
        }
    }
    v
}

/// La moyenne de `scores`, arrondie au centième ([`round`]) ; `None` sans score.
fn mean(scores: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, n) = scores.fold((0.0, 0u32), |(sum, n), s| (sum + s, n + 1));
    (n > 0).then(|| round(sum / f64::from(n)))
}

#[cfg(test)]
#[path = "queue_tests.rs"]
mod queue_tests;
