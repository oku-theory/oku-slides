//! Un emplacement du travailleur : pose des widgets, langue et rendu.
//!
//! Règles (voir aussi `client`) : un échec borné pendant une opération
//! d'un emplacement accuse le pack de l'emplacement, sauf pour un
//! emplacement d'essai ; le `Drop` met `CloseSlot` en file.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Deck, WidgetRegistry};
use oku_slides_i18n::Localizer;
use oku_slides_render::{PackSet, RenderError};
use serde_json::{Map, Value};

use crate::client::{
    Deferred, RenderWorker, blame, evict_assets, expect_ok, faulty_pack, set_locale, unexpected, wire,
};
use crate::compiled::RemoteCompiled;
use crate::link::{Link, fingerprint_hex};
use crate::protocol::{Request, Response, SlotId, WirePack};

/// Un emplacement du travailleur : un rendu, ses widgets et sa langue.
///
/// La langue est celle du travailleur, commune à ses emplacements.
pub struct RenderSlot {
    worker: RenderWorker,
    pub(crate) id: SlotId,
    registry: Arc<WidgetRegistry>,
    chart_palette: Vec<String>,
    /// Un échec borné y accuse le pack ; `false` pour un emplacement d'essai.
    accuse: bool,
    /// Époque du travailleur où `set_widgets` a posé le pack.
    pack_epoch: u64,
}

impl std::fmt::Debug for RenderSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderSlot").field("id", &self.id).finish_non_exhaustive()
    }
}

impl RenderSlot {
    /// L'emplacement `id`, ouvert dans le travailleur ; `accuse` : `false` pour un essai.
    pub(crate) fn opened(worker: RenderWorker, id: SlotId, registry: Arc<WidgetRegistry>, accuse: bool) -> Self {
        Self { worker, id, registry, chart_palette: Vec::new(), accuse, pack_epoch: 0 }
    }

    /// Widgets rendus : ceux des packs de `set` (aucun, si vide).
    pub fn set_widgets(&mut self, set: &PackSet, registry: Arc<WidgetRegistry>) -> Result<(), RenderError> {
        let mut link = self.worker.link();
        let wire: Vec<_> = set.packs().iter().map(wire).collect();
        if self.accuse
            && let Some(p) = wire.iter().find(|p| link.faulty_packs.contains(&fingerprint_hex(p)))
        {
            return Err(RenderError::pack_disabled(p.id.clone()));
        }
        let blamed: &[_] = if self.accuse { &wire } else { &[] };
        self.worker.prepare_for(&mut link, blamed)?;
        let req = Request::SetWidgets { slot: self.id, packs: wire.clone() };
        if let Err(e) = expect_ok(&mut link, &req, &[]) {
            blame(&mut link, blamed, &e);
            return Err(e);
        }
        self.pack_epoch = link.restarts;
        if let Some(state) = link.state.slots.get_mut(&self.id) {
            state.packs = wire;
        }
        self.registry = registry;
        Ok(())
    }

    /// Jetons du thème des rendus suivants (`None` : jetons par défaut) ; sans effet s'ils sont déjà posés.
    pub fn set_theme(&mut self, tokens: Option<&Map<String, Value>>) -> Result<(), RenderError> {
        let mut link = self.worker.link();
        if link.state.slots.get(&self.id).is_some_and(|s| s.theme.as_ref() == tokens) {
            return Ok(());
        }
        let packs = self.blamed_packs(&link);
        self.worker.prepare_for(&mut link, &packs)?;
        expect_ok(&mut link, &Request::SetTheme { slot: self.id, tokens: tokens.cloned() }, &[])?;
        if let Some(state) = link.state.slots.get_mut(&self.id) {
            state.theme = tokens.cloned();
        }
        Ok(())
    }

    /// Les packs posés dans l'emplacement qu'un échec borné accuse (aucun pour un essai).
    fn blamed_packs(&self, link: &Link) -> Vec<WirePack> {
        if self.accuse {
            link.state.slots.get(&self.id).map(|s| s.packs.clone()).unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Registre des widgets rendus.
    pub fn registry(&self) -> &Arc<WidgetRegistry> {
        &self.registry
    }

    /// Langue des cadres de repli et des libellés des graphiques.
    pub fn set_language(&mut self, l: &Localizer) -> Result<(), RenderError> {
        let mut link = self.worker.link();
        self.worker.prepare(&mut link)?;
        set_locale(&mut link, l)
    }

    /// Rend `deck` sous une nouvelle génération.
    pub fn render(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> Result<RemoteCompiled, RenderError> {
        let mut link = self.worker.link();
        if self.accuse
            && let Some(pack) = faulty_pack(&link, self.id)
        {
            return Err(RenderError::pack_disabled(pack));
        }
        let packs = self.blamed_packs(&link);
        self.worker.prepare_for(&mut link, &packs)?;
        // Essai : un pack fautif posé avant une relance n'a pas été rejoué.
        if !self.accuse
            && link.restarts != self.pack_epoch
            && let Some(pack) = faulty_pack(&link, self.id)
        {
            return Err(RenderError::pack_disabled(pack));
        }
        for (id, bytes) in assets {
            if !link.sent_assets.contains(id) {
                expect_ok(&mut link, &Request::PutAsset { id: id.clone() }, bytes)?;
                link.sent_assets.insert(id.clone());
            }
        }
        let ids: Vec<String> = assets.keys().cloned().collect();
        let req = Request::Render { slot: self.id, deck: Box::new(deck.clone()), assets: ids.clone() };
        let header = match link.call(&req, &[]) {
            Ok((Response::Compiled(header), _)) => header,
            Ok((Response::Error { error }, _)) => return Err(error.into_render()),
            Ok((other, _)) => return Err(unexpected(&other)),
            Err(e) => {
                blame(&mut link, &packs, &e);
                return Err(e);
            }
        };
        // L'époque du travailleur qui a répondu : `call` ne relance pas une connexion ouverte.
        let epoch = link.restarts;
        if let Some(state) = link.state.slots.get_mut(&self.id) {
            state.last_assets = ids;
        }
        evict_assets(&mut link);
        drop(link);
        let mut header = *header;
        self.chart_palette = std::mem::take(&mut header.chart_palette);
        RemoteCompiled::from_header(self.worker.clone(), self.id, epoch, self.accuse, header)
    }

    /// Palette des graphiques du dernier rendu (celle du thème du deck).
    pub fn chart_palette(&self) -> &[String] {
        &self.chart_palette
    }
}

impl Drop for RenderSlot {
    /// Met `CloseSlot` en file : envoyé avant la requête suivante.
    fn drop(&mut self) {
        self.worker.1.defer(Deferred::Close { slot: self.id });
    }
}
