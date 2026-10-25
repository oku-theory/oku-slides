//! Les notes d'un import hi-fi : chaque slide Oku comparée à sa page de référence.
//!
//! Chaque slide Oku, rognée à la page source ([`super::geometry`]), est
//! comparée à sa page de référence ([`metrics::score`], régions des items
//! source) ; les slides se répondent par leur numéro source. Une note d'un
//! outil ([`ImportState::observe`]) coûte ce que dit [`super::Tuner`] et
//! garde l'empreinte Typst de la page notée (un filtre : une page à la même
//! empreinte a le même raster ; les essais, eux, suivent l'empreinte du
//! raster noté). Pour la CLI, une slide qu'on ne peut noter prive tout le
//! deck de scores ([`ImportState::score_deck`]) : le rapport dit pourquoi, le
//! journal dit le détail. Hi-fi seulement : le retrofit ne se compare pas au
//! témoin.

use oku_slides_core::Frame;
use oku_slides_import::metrics::{self, SlideScore};
use oku_slides_import::report::{ReferenceCause, ReferenceReport};
use oku_slides_reference::Raster;
use oku_slides_render_worker::RemoteCompiled;

use super::ImportState;
use super::geometry::{Geometry, crop_box, image, px};
use super::reference::{Failure, render_failure};
use super::tune::{Charge, Seen, SlideState, fingerprint};

impl ImportState {
    /// Note la slide `slide` sur `oku` et garde le score au rapport.
    ///
    /// L'échec revient à l'appelant.
    fn rescore(&mut self, slide: &str, oku: &Raster) -> Result<SlideScore, Failure> {
        let score = self.scored(slide, oku)?;
        let report = self.plan.report_mut().slides.iter_mut().find(|r| r.slide == slide);
        let report = report.ok_or_else(|| (ReferenceCause::Failed, format!("slide {slide} absente du rapport")))?;
        report.score = Some(score.clone());
        Ok(score)
    }

    /// Note la slide `slide` sur `oku`, sa page rendue maintenant.
    ///
    /// `(deck, model)` : les empreintes de l'état du deck et du modèle de la slide.
    ///
    /// La note coûte ce que dit [`super::Tuner::charge`] : refusée, rien ne
    /// change et le score manque ; un raster inchangé garde le score. `page` :
    /// l'empreinte Typst de la page, le filtre du rapport suivant. Un échec ne
    /// change rien : ni score, ni essai, ni empreinte.
    pub(crate) fn observe(
        &mut self,
        slide: &str,
        oku: &Raster,
        page: Option<u128>,
        (deck, model): (u128, u128),
    ) -> Result<(Charge, Option<SlideScore>), Failure> {
        let raster = fingerprint([&oku.width().to_le_bytes()[..], &oku.height().to_le_bytes(), oku.rgba()]);
        let now = Seen { raster, deck, slide: model };
        let current = self.plan.report.slides.iter().find(|r| r.slide == slide).and_then(|r| r.score.clone());
        let charge = self.tuner.charge(slide, now, self.slide_state(slide));
        if charge == Charge::Refused {
            return Ok((charge, None));
        }
        let score = match (charge, current) {
            (Charge::Unchanged, Some(score)) => score,
            _ => self.rescore(slide, oku)?,
        };
        self.tuner.record(slide, now, charge, &score, self.threshold);
        match page {
            Some(page) => self.scored.insert(slide.to_owned(), page),
            None => self.scored.remove(slide),
        };
        Ok((charge, Some(score)))
    }

    /// La page de `slide`, d'empreinte Typst inchangée, n'a pas à être notée.
    ///
    /// L'état du deck `deck` et le modèle `model` de la slide passent.
    pub(crate) fn touch(&mut self, slide: &str, deck: u128, model: u128) {
        self.tuner.touch(slide, deck, model);
    }

    /// L'état de `slide` dans la file, d'après sa note et ses essais.
    #[must_use]
    pub fn slide_state(&self, slide: &str) -> SlideState {
        let score = self.plan.report.slides.iter().find(|r| r.slide == slide).and_then(|r| r.score.as_ref());
        SlideState::of(score, self.tuner.tuning(slide), self.threshold, self.tuner.limits())
    }

    /// L'empreinte de la page Oku à la dernière note de `slide`.
    pub(crate) fn scored_page(&self, slide: &str) -> Option<u128> {
        self.scored.get(slide).copied()
    }

    /// Note chaque slide du rendu `compiled` contre la référence (la CLI).
    ///
    /// Sans effet tant que la référence n'est pas faite ([`Self::reference_job`]).
    /// Pose au rapport le témoin et le seuil ; une référence en échec, ou une
    /// slide qu'on ne peut noter, laisse toutes les slides sans score et dit pourquoi.
    pub fn score_deck(&mut self, compiled: &RemoteCompiled) {
        let Some(reference) = self.reference.get() else { return };
        let (name, made) = (reference.witness.clone(), reference.pages.as_ref().map(|_| ()).map_err(Clone::clone));
        self.settle();
        let scored = match made {
            Ok(()) => match self.score_all(compiled) {
                Ok(()) => Ok(name),
                Err(failure) => {
                    self.warn(&failure, None);
                    Err((name, failure.0))
                }
            },
            // L'échec du témoin est journalisé une fois, quand il survient.
            Err(cause) => Err((name, cause)),
        };
        self.set_scored(scored);
    }

    /// Pose au rapport le témoin et le seuil.
    ///
    /// Un échec (témoin, cause) laisse toutes les slides sans score.
    fn set_scored(&mut self, scored: Result<String, (String, ReferenceCause)>) {
        let threshold = self.threshold;
        let report = self.plan.report_mut();
        let (witness, unavailable) = match scored {
            Ok(witness) => (witness, None),
            Err((witness, cause)) => {
                for slide in &mut report.slides {
                    slide.score = None;
                }
                self.scored.clear();
                (witness, Some(cause))
            }
        };
        report.reference = Some(ReferenceReport { witness, threshold, unavailable });
    }

    /// Note chaque slide du rapport sur sa page de `compiled`.
    ///
    /// Le premier échec arrête tout, rien n'est gardé.
    fn score_all(&mut self, compiled: &RemoteCompiled) -> Result<(), Failure> {
        let ppp = oku_slides_core::num::narrow_f32(Geometry::of(&self.source).ppp);
        let mut scores = Vec::with_capacity(self.plan.report.slides.len());
        for slide in &self.plan.report.slides {
            let failed = |what: &str| (ReferenceCause::Failed, format!("slide {} : {what}", slide.source));
            let page = compiled.layout().page_of(&slide.slide).and_then(|p| p.checked_sub(1));
            let page = page.ok_or_else(|| failed("page Oku absente du rendu"))?;
            let oku = compiled.png(page, ppp).map_err(|e| render_failure(&e, "page Oku"))?;
            let oku = Raster::from_png(&oku).map_err(|e| (ReferenceCause::Failed, format!("page Oku : {e}")))?;
            scores.push((self.scored(&slide.slide, &oku)?, compiled.page_hash(page)));
        }
        let mut hashes = Vec::with_capacity(scores.len());
        for (slide, (score, page)) in self.plan.report_mut().slides.iter_mut().zip(scores) {
            slide.score = Some(score);
            hashes.push((slide.slide.clone(), page));
        }
        for (slide, page) in hashes {
            match page {
                Some(page) => self.scored.insert(slide, page),
                None => self.scored.remove(&slide),
            };
        }
        Ok(())
    }

    /// La page de référence de `slide` et sa page Oku rognée à la source.
    ///
    /// Les deux à la même taille, celle que [`metrics::score`] compare.
    pub(crate) fn pair(&self, slide: &str, oku: &Raster) -> Result<(Raster, Raster), Failure> {
        let failed = |what: &str| (ReferenceCause::Failed, format!("slide {slide} : {what}"));
        let page = self.reference_page(slide).ok_or_else(|| failed("page du témoin absente"))?;
        let refr = Raster::from_png(page).map_err(|e| (ReferenceCause::Failed, format!("page du témoin : {e}")))?;
        let geo = Geometry::of(&self.source);
        let (x, y) = (px(geo.rect.x, geo.ppp), px(geo.rect.y, geo.ppp));
        // Rognées à leur taille commune : un arrondi d'un pixel ne prive pas la slide de son score.
        let w = geo.width.min(refr.width()).min(oku.width().saturating_sub(x));
        let h = px(geo.rect.h, geo.ppp).min(refr.height()).min(oku.height().saturating_sub(y));
        Ok((refr.crop(0, 0, w, h), oku.crop(x, y, w, h)))
    }

    /// La rognure `[x, y, w, h]` du cadre `frame`, à `margin` pixels autour.
    ///
    /// `frame` est en pt de la page Oku ; la page source rognée fait `width` × `height` pixels.
    ///
    /// Des bords signés : un cadre qui commence avant la page n'élargit pas
    /// la rognure. `None` pour une page vide.
    pub(crate) fn crop(&self, frame: Frame, margin: u32, (width, height): (u32, u32)) -> Option<[u32; 4]> {
        crop_box(Geometry::of(&self.source).edges(frame), margin, width, height)
    }

    /// L'écart de la slide Oku `slide`, rendue en `oku`, à sa page de référence.
    fn scored(&self, slide: &str, oku: &Raster) -> Result<SlideScore, Failure> {
        let failed = |what: &str| (ReferenceCause::Failed, format!("slide {slide} : {what}"));
        let src = self.source_slide(slide).ok_or_else(|| failed("source"))?;
        let (refr, oku) = self.pair(slide, oku)?;
        let regions = metrics::regions(src, Geometry::of(&self.source).scale, self.witness_limits());
        metrics::score(image(&refr), image(&oku), &regions).ok_or_else(|| failed("images vides"))
    }
}
