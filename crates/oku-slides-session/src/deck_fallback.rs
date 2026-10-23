//! Repli par pack : un pack qui fait échouer un rendu est écarté seul.
//!
//! Les packs de confiance (versions livrées de `base`, `is_trusted_pack`) ne
//! sont jamais accusés ni écartés : un échec qu'eux seuls portent est celui
//! du deck, et l'erreur remonte.
//!
//! Un échec borné (délai, mémoire, arrêt du processus de rendu), celui du
//! deck ou d'un autre emplacement (aperçus, variantes, polices), accuse un
//! pack quand l'emplacement ne porte que lui, et pas un pack de confiance :
//! le processus de rendu le marque fautif. Sinon il n'en accuse aucun (le
//! contenu `base` du deck peut suffire à l'échec) ; la session cherche le
//! coupable par dichotomie (`isolate_faulty`) parmi les autres packs, sur le
//! deck qui a échoué : il est rendu avec les packs de confiance et une
//! moitié des autres, puis avec la moitié en cause, jusqu'à un pack seul,
//! que son échec fait marquer par la session (`mark_faulty`) ; au plus
//! ⌈log2 N⌉ + 1 rendus. Le pack fautif et ceux qui le requièrent sont alors
//! écartés pour la session (`disable_faulty`), sans changer `deck.widgets` ;
//! un avis les nomme. Les éléments de leurs widgets se rendent comme un
//! widget inconnu.
//!
//! Avant la dichotomie, un rendu témoin avec les seuls packs de confiance :
//! s'il échoue aussi, l'échec est propre au deck et aucun pack n'est accusé.
//! Un échec que la dichotomie n'attribue à aucun pack (il ne se reproduit
//! pas) n'écarte rien non plus : l'erreur d'origine remonte.

use snafu::{OptionExt, ResultExt};

use oku_slides_core::{Deck, Namespace};
use oku_slides_render::{PackSet, PackWidgets, RenderError, is_trusted_pack};
use oku_slides_render_worker::RemoteCompiled;

use crate::deck_theme::render_themed;
use crate::error::{NotRenderedSnafu, RegistrySnafu, RenderSnafu};
use crate::{Result, Session, new_renderer};

impl Session {
    /// Rend `deck` avec les widgets installés. Si un pack a fait échouer le
    /// rendu (délai, mémoire, arrêt du processus de rendu), il est écarté
    /// (voir `disable_faulty`) et le rendu refait une fois.
    pub(crate) fn render_deck(&mut self, deck: &Deck) -> Result<RemoteCompiled> {
        self.probe_remaining();
        let layer = self.theme_layer(deck)?;
        self.open_deck_renderer()?;
        let renderer = self.renderer.as_mut().context(NotRenderedSnafu)?;
        let e = match render_themed(renderer, deck, &layer, &self.doc.sources.assets) {
            Ok(compiled) => return Ok(compiled),
            Err(e) => e,
        };
        if !self.disable_or_log(&e, deck) {
            return Err(e).context(RenderSnafu)?;
        }
        self.open_deck_renderer()?;
        let renderer = self.renderer.as_mut().context(NotRenderedSnafu)?;
        Ok(render_themed(renderer, deck, &layer, &self.doc.sources.assets).context(RenderSnafu)?)
    }

    /// Un pack installé est-il fautif ?
    fn has_faulty_pack(&self) -> bool {
        self.packs.packs().iter().any(|p| self.worker.is_faulty(p))
    }

    /// Repli : si `e` est un échec borné, le pack fautif (cherché par
    /// dichotomie sur `deck` parmi ceux qui ne sont pas de confiance) et ceux
    /// qui le requièrent sont écartés (rendu du deck, session, sources), et un
    /// avis les nomme. Vrai si un pack a été écarté ; jamais un pack de confiance.
    ///
    /// Tout emplacement y passe : deck, aperçus de layouts, variantes, polices.
    pub(crate) fn disable_faulty(&mut self, e: &RenderError, deck: &Deck) -> Result<bool> {
        let (trusted, others): (Vec<_>, Vec<_>) = self.packs.packs().iter().cloned().partition(is_trusted_pack);
        if !e.kind().is_bounded() || others.is_empty() {
            // Sans autre pack que ceux de confiance, l'échec est celui du deck.
            return Ok(false);
        }
        if !self.has_faulty_pack() {
            // Rendu témoin avec les seuls packs de confiance : un échec propre au deck n'accuse aucun pack.
            if self.fails_with(&trusted, &[], deck)? {
                tracing::warn!("le deck échoue avec les seuls packs de confiance : aucun pack accusé");
                return Ok(false);
            }
            self.isolate_faulty(&trusted, others, deck)?;
        }
        if !self.has_faulty_pack() {
            return Ok(false);
        }
        tracing::warn!(error = %oku_slides_platform::errors::one_line(e), "pack de widgets fautif : écarté");
        let before = self.widgets_disabled.clone();
        let pins = self.widgets_pins.clone();
        let (widgets, _) = self.resolve_widgets(&pins, &self.doc.sources.packs)?;
        self.install_widgets(widgets)?;
        self.doc.sources.set_registry(self.registry.clone());
        let fresh = self.widgets_disabled.iter().filter(|(ns, _)| !before.contains_key(*ns));
        let notice = self.packs_disabled_notice(fresh.map(|(_, name)| name.as_str()));
        self.set_notice(notice);
        Ok(true)
    }

    /// Cherche par dichotomie, parmi `candidates`, le pack qui fait échouer le rendu de `deck`.
    ///
    /// Chaque essai rend `deck` dans un emplacement neuf avec les packs de
    /// confiance `trusted` et une partie des candidats ; l'échec borné d'un
    /// candidat seul le fait marquer fautif (même s'il n'y en avait qu'un).
    /// Au plus ⌈log2 N⌉ + 1 rendus.
    fn isolate_faulty(&mut self, trusted: &[PackWidgets], mut candidates: Vec<PackWidgets>, deck: &Deck) -> Result<()> {
        while candidates.len() > 1 {
            let rest = candidates.split_off(candidates.len() / 2);
            if !self.fails_with(trusted, &candidates, deck)? {
                candidates = rest;
            }
        }
        if let [single] = candidates.as_slice()
            && !self.worker.is_faulty(single)
            && self.fails_with(trusted, &candidates, deck)?
        {
            // L'emplacement portait les packs de confiance : il n'a accusé personne.
            self.worker.mark_faulty(single);
        }
        let names: Vec<&str> = candidates.iter().map(|p| p.namespace().as_str()).collect();
        tracing::debug!(packs = ?names, "dichotomie des packs terminée");
        Ok(())
    }

    /// `deck` rendu avec les seuls packs `trusted` et `candidates` échoue-t-il d'un échec borné ?
    ///
    /// Un emplacement neuf, jeté ensuite.
    fn fails_with(&mut self, trusted: &[PackWidgets], candidates: &[PackWidgets], deck: &Deck) -> Result<bool> {
        #[cfg(test)]
        if candidates.is_empty() && std::mem::take(&mut self.failpoints.control_render) {
            return Ok(true);
        }
        let packs: Vec<PackWidgets> = trusted.iter().chain(candidates).cloned().collect();
        let set = PackSet::new(packs).context(RegistrySnafu)?;
        // Le `base` intégré courant seul : registre intégré, sans sonde.
        let registry = self.registry_for(&set)?;
        let layer = self.theme_layer(deck)?;
        let names: Vec<&Namespace> = set.packs().iter().map(PackWidgets::namespace).collect();
        let slot = new_renderer(&self.worker, self.render_fonts.clone(), &self.localizer, &set, &registry);
        let rendered = slot.and_then(|mut slot| {
            render_themed(&mut slot, deck, &layer, &self.doc.sources.assets).context(RenderSnafu)?;
            Ok(())
        });
        let failed = rendered.as_ref().err().and_then(|e| e.render_error()).is_some_and(|e| e.kind().is_bounded());
        tracing::debug!(packs = ?names, failed, "essai de rendu d'une partie des packs");
        Ok(failed)
    }

    /// [`Self::disable_faulty`] ; s'il échoue, journalisé et faux : l'appelant
    /// rend l'erreur d'origine `e`, pas celle du repli.
    fn disable_or_log(&mut self, e: &RenderError, deck: &Deck) -> bool {
        self.disable_faulty(e, deck).unwrap_or_else(|back| {
            tracing::error!(error = %back.one_line(), "pack fautif non écarté");
            false
        })
    }

    /// Les packs installés, sauf s'il en est devenu un fautif ailleurs : il est
    /// écarté d'abord.
    pub(crate) fn usable_packs(&mut self) -> Result<PackSet> {
        if let Some(pack) = self.packs.packs().iter().find(|p| self.worker.is_faulty(p)) {
            let e = RenderError::pack_disabled(pack.id());
            let deck = self.doc.sources.deck().clone();
            self.disable_faulty(&e, &deck)?;
        }
        Ok(self.packs.clone())
    }

    /// `attempt`, refait une fois après avoir écarté le pack qui l'a fait
    /// échouer (voir `disable_faulty`, dichotomie sur chacun des `decks` que
    /// rend `attempt`, jusqu'au premier qui écarte un pack).
    ///
    /// `decks` ne sert qu'après un échec ; s'il échoue, journalisé : l'erreur d'origine remonte.
    pub(crate) fn with_fallback<T>(
        &mut self,
        decks: impl FnOnce(&Self) -> Result<Vec<Deck>>,
        mut attempt: impl FnMut(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let e = match attempt(self) {
            Ok(v) => return Ok(v),
            Err(e) => e,
        };
        let Some(re) = e.render_error().filter(|re| re.kind().is_bounded()) else { return Err(e) };
        let decks = match decks(self) {
            Ok(decks) => decks,
            Err(back) => {
                tracing::error!(error = %back.one_line(), "decks du repli illisibles : pack fautif non écarté");
                return Err(e);
            }
        };
        let disabled = decks.iter().any(|deck| self.disable_or_log(re, deck));
        if disabled { attempt(self) } else { Err(e) }
    }
}
