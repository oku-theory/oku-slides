//! Un emplacement du travailleur : pose des widgets, langue et rendu.
//!
//! Règles (voir aussi `client`) : un échec borné pendant une opération
//! d'un emplacement accuse le pack de l'emplacement, sauf pour un
//! emplacement d'essai ; le `Drop` met `CloseSlot` en file.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Deck, WidgetRegistry};
use oku_slides_data::{CellPatch, Table};
use oku_slides_i18n::Localizer;
use oku_slides_render::{PackSet, RenderError, Results};
use serde_json::{Map, Value};

use crate::client::{
    Deferred, RenderWorker, Shared, blame, evict_assets, expect_ok, faulty_pack, set_locale, unexpected, wire,
};
use crate::compiled::RemoteCompiled;
use crate::link::{Link, SentData, SentResult, fingerprint_hex};
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
    /// The query results the next renders read ([`RenderSlot::set_results`]).
    results: Results,
}

impl std::fmt::Debug for RenderSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderSlot").field("id", &self.id).finish_non_exhaustive()
    }
}

impl RenderSlot {
    /// L'emplacement `id`, ouvert dans le travailleur ; `accuse` : `false` pour un essai.
    pub(crate) fn opened(worker: RenderWorker, id: SlotId, registry: Arc<WidgetRegistry>, accuse: bool) -> Self {
        Self { worker, id, registry, chart_palette: Vec::new(), accuse, pack_epoch: 0, results: Results::new() }
    }

    /// Rendered widgets: those of the packs of `set` (none, if empty).
    ///
    /// # Errors
    ///
    /// Fails (`pack_disabled`) if a pack of `set` is already at fault, or if
    /// the worker does not start, dies or exceeds its bounds;
    /// the pack is then blamed, except for a trial.
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

    /// Theme tokens for the next renders (`None`: default tokens); no effect if already set.
    ///
    /// # Errors
    ///
    /// Fails if the worker does not start, dies or exceeds its bounds, or if it rejects the tokens.
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
    #[must_use]
    pub fn registry(&self) -> &Arc<WidgetRegistry> {
        &self.registry
    }

    /// Language of the fallback frames and of the chart labels.
    ///
    /// # Errors
    ///
    /// Fails if the worker does not start, dies or exceeds its bounds, or if it rejects the language.
    pub fn set_language(&mut self, l: &Localizer) -> Result<(), RenderError> {
        let mut link = self.worker.link();
        self.worker.prepare(&mut link)?;
        set_locale(&mut link, l)
    }

    /// The query results the next renders read: a query data absent or `None` here draws its
    /// views' failure frame.
    pub fn set_results(&mut self, results: Results) {
        self.results = results;
    }

    /// Renders `deck` under a new generation.
    ///
    /// # Errors
    ///
    /// Fails (`pack_disabled`) if a pack of the slot is at fault, if the
    /// compilation fails, or if the worker does not start, dies or exceeds its bounds.
    /// Fails (`frame`) if the worker still lacks assets after resending them once.
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
        let ids: Vec<String> = assets.keys().cloned().collect();
        let req = render_request(self.id, deck, ids.clone(), &self.results);
        let mut resent = false;
        let mut data_resent = false;
        let header = loop {
            send_assets(&mut link, assets)?;
            send_results(&mut link, self.id, deck, &self.results, &self.worker.1)?;
            send_data(&mut link, self.id, deck, &self.worker.1)?;
            match link.call(&req, &[]) {
                Ok((Response::Compiled(header), _)) => break header,
                // The worker lacks assets of this render: sent to a worker that restarted
                // since (inside the `PutAsset` loop or `call`), or wrongly listed as sent. They
                // go again, once; a second refusal is an error, never a fallback frame (t-1295).
                Ok((Response::MissingAssets { ids: lost }, _)) => {
                    if lost.is_empty() || !lost.iter().all(|id| assets.contains_key(id)) {
                        return Err(RenderError::frame("missing assets outside the render"));
                    }
                    if resent {
                        return Err(RenderError::frame(format!("{} assets still missing after a resend", lost.len())));
                    }
                    tracing::warn!(count = lost.len(), "processus de rendu : assets manquants renvoyés");
                    for id in &lost {
                        link.sent_assets.remove(id);
                    }
                    resent = true;
                }
                // Data sent to a worker that restarted since: sent again, once.
                Ok((Response::MissingData { ids: lost }, _)) => {
                    if data_resent {
                        return Err(RenderError::frame(format!("{} data still missing after a resend", lost.len())));
                    }
                    if let Some(state) = link.state.slots.get_mut(&self.id) {
                        state.data.clear();
                        state.results.clear();
                    }
                    data_resent = true;
                }
                Ok((Response::Error { error }, _)) => return Err(error.into_render()),
                Ok((other, _)) => return Err(unexpected(&other)),
                Err(e) => {
                    blame(&mut link, &packs, &e);
                    return Err(e);
                }
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
    #[must_use]
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

/// Sends the worker each data of `deck` whose `Arc` or base path changed since it was last sent
/// to `slot` (only its changed cells after a cell edit), and drops those `deck` no longer has.
fn send_data(link: &mut Link, slot: SlotId, deck: &Deck, shared: &Shared) -> Result<(), RenderError> {
    let sent = link.state.slots.get(&slot).map(|s| s.data.clone()).unwrap_or_default();
    let mut now = BTreeMap::new();
    for (id, data) in &deck.data {
        let base = data.table().and_then(|t| t.base.as_ref()).and_then(|b| shared.base_path(&b.id));
        let id = id.as_str().to_owned();
        let last = sent.get(&id).filter(|s| s.base == base);
        if !last.is_some_and(|s| Arc::ptr_eq(&s.data, data)) {
            // A cell edit goes as its changed cells, the rest of the data being cached.
            if let Some(patch) = last.and_then(|s| CellPatch::between(&s.data, data)) {
                let json = serde_json::to_vec(&patch).map_err(|e| RenderError::frame(format!("data {id}: {e}")))?;
                expect_ok(link, &Request::PatchData { slot, id: id.clone() }, &json)?;
            } else {
                let json = serde_json::to_vec(&**data).map_err(|e| RenderError::frame(format!("data {id}: {e}")))?;
                expect_ok(link, &Request::PutData { slot, id: id.clone(), base: base.clone() }, &json)?;
            }
        }
        now.insert(id, SentData { data: data.clone(), base });
    }
    let gone: Vec<String> = sent.keys().filter(|id| !now.contains_key(*id)).cloned().collect();
    if !gone.is_empty() {
        expect_ok(link, &Request::DropData { slot, ids: gone }, &[])?;
    }
    if let Some(state) = link.state.slots.get_mut(&slot) {
        state.data = now;
    }
    Ok(())
}

/// The ready results of `results` that `deck` reads: those of its query data.
fn ready_results<'a>(deck: &'a Deck, results: &'a Results) -> impl Iterator<Item = (&'a str, &'a Table)> {
    results.iter().filter_map(|(id, table)| {
        let table = table.as_ref()?;
        deck.data.get(id).filter(|d| d.sql().is_some()).map(|_| (id.as_str(), table))
    })
}

/// Sends the worker each ready result `deck` reads whose generation or base path changed since
/// it was last sent to `slot`, and drops those no longer ready. Dropping one forgets its data in
/// the worker too (`DropData`): `send_data`, after it, sends that data again.
fn send_results(
    link: &mut Link,
    slot: SlotId,
    deck: &Deck,
    results: &Results,
    shared: &Shared,
) -> Result<(), RenderError> {
    let sent = link.state.slots.get(&slot).map(|s| s.results.clone()).unwrap_or_default();
    let mut now = BTreeMap::new();
    for (id, table) in ready_results(deck, results) {
        let base = table.base.as_ref().and_then(|b| shared.base_path(&b.id));
        let next = SentResult { generation: table.generation.clone(), base: base.clone() };
        if sent.get(id) != Some(&next) {
            let json = serde_json::to_vec(table).map_err(|e| RenderError::frame(format!("result {id}: {e}")))?;
            expect_ok(link, &Request::PutResult { slot, id: id.to_owned(), base }, &json)?;
        }
        now.insert(id.to_owned(), next);
    }
    let gone: Vec<String> = sent.keys().filter(|id| !now.contains_key(*id)).cloned().collect();
    if !gone.is_empty() {
        expect_ok(link, &Request::DropData { slot, ids: gone.clone() }, &[])?;
    }
    if let Some(state) = link.state.slots.get_mut(&slot) {
        state.results = now;
        for id in &gone {
            state.data.remove(id);
        }
    }
    Ok(())
}

/// The render request of `deck` for `slot`, without its data (sent apart, see `send_data`) nor
/// its knowledge bundle (up to 16 MiB), which rendering never reads (t-1731); the query data
/// read the ready results of `results` (sent apart, see `send_results`).
fn render_request(slot: SlotId, deck: &Deck, assets: Vec<String>, results: &Results) -> Request {
    let data = deck.data.keys().map(|id| id.as_str().to_owned()).collect();
    let results = ready_results(deck, results).map(|(id, _)| id.to_owned()).collect();
    let mut bare = deck.without_knowledge();
    bare.data.clear();
    Request::Render { slot, deck: Box::new(bare), assets, data, results }
}

/// Sends the worker every asset of `assets` it is not known to have.
fn send_assets(link: &mut Link, assets: &BTreeMap<String, Vec<u8>>) -> Result<(), RenderError> {
    for (id, bytes) in assets {
        if !link.sent_assets.contains(id) {
            expect_ok(link, &Request::PutAsset { id: id.clone() }, bytes)?;
            link.sent_assets.insert(id.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "slot_tests.rs"]
mod slot_tests;
