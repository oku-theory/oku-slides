//! What a pasted element brings from the tab it was copied in (t-1178, t-1690).
//!
//! The source tab gives the bytes of the assets the pasted elements cite
//! ([`Session::element_assets`]) and the data their views name (`Session::paste_source`).
//! The target keeps the assets in memory (`Session::carry_assets`); they enter its bundle,
//! and leave memory, when an op cites them (`Session::embed_op_assets`). The data is copied
//! under fresh ids and names (a query data in another deck as its result, a manual data over
//! the result's base), every pasted view names its copy (`Session::with_pasted_data`),
//! and the paste is one undo step.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use oku_slides_core::{Element, Op, check_external};
use oku_slides_data::{BaseId, Data, DataId, Table};
use snafu::ResultExt;

use crate::data_copy::{Queries, copy_data_ops};
use crate::data_panel::data_missing;
use crate::derived::QueryState;
use crate::error::{PrepareSnafu, QueryPasteNoResultSnafu};
use crate::widget_examples::{each_added, op_data_id, rename_refs, view_refs};
use crate::{Request, Result, Session};

/// The assets brought by the last paste, by id.
pub(crate) type HeldAssets = BTreeMap<String, Vec<u8>>;

/// What a paste reads from the tab its element was copied in.
#[derive(Debug, Default)]
pub(crate) struct PasteSource {
    /// The bytes of the assets the pasted elements cite.
    pub(crate) assets: Vec<Vec<u8>>,
    /// The data their views name, by id.
    pub(crate) data: BTreeMap<DataId, Arc<Data>>,
    /// The bases of those data the source deck knows: the paste makes them known, and leased.
    pub(crate) bases: BTreeSet<BaseId>,
    /// The ready results of the query data among them, as tables over their bases.
    pub(crate) results: BTreeMap<DataId, Table>,
    /// The tab they were read in.
    pub(crate) from: String,
    /// Whether the paste lands in that same tab: its query data are then copied as queries,
    /// else as manual data holding their result.
    pub(crate) same_deck: bool,
}

impl Session {
    /// Les octets des assets que cite `element` et que le bundle tient.
    ///
    /// Un asset cité mais absent du bundle est omis : l'onglet d'arrivée le refusera
    /// comme tout asset inconnu.
    pub fn element_assets(&self, element: &Element) -> Vec<Vec<u8>> {
        let refs = oku_slides_core::element_asset_refs(element, &self.registry);
        refs.iter().filter_map(|id| self.doc.sources.assets.get(id).cloned()).collect()
    }

    /// What the paste `ops` need from this deck: the assets and data their added elements name.
    ///
    /// An asset or data this deck lacks is left out: the paste then refuses it.
    pub(crate) fn paste_source(&self, ops: &[Op]) -> PasteSource {
        let deck = self.doc.sources.deck();
        let mut source = PasteSource::default();
        for op in ops {
            if let Op::AddElement { element, .. } = op {
                source.assets.extend(self.element_assets(element));
            }
            let mut op = op.clone();
            each_added(&mut op, &mut |_, params| {
                for id in view_refs(params) {
                    if let Some(data) = deck.data.get(&id) {
                        let base = data.table().and_then(|t| t.base.as_ref()).map(|b| &b.id);
                        source.bases.extend(base.filter(|b| self.doc.sources.bases.contains(*b)).cloned());
                        if let Some(QueryState::Ready { out, .. }) = self.query_state(&id) {
                            source.results.insert(id.clone(), out.table());
                        }
                        source.data.insert(id, Arc::clone(data));
                    }
                }
            });
        }
        source
    }

    /// Applies the paste `ops` with what `source` brings, through `handle`: the snapshot.
    ///
    /// # Errors
    ///
    /// As `handle` for an `Apply`, plus `op-data-missing` for a view naming a data `source`
    /// lacks.
    pub(crate) fn paste(&mut self, label: &str, ops: Vec<Op>, mut source: PasteSource) -> Result<String> {
        // The checks of an `Apply`; `apply_paste` checks the ops themselves (`ensure_caller_ops`).
        let prepared = self.prepare(&Request::Apply { label: label.to_owned(), ops: Vec::new(), merge: None })?;
        // Known before the copies are applied, so they render; a refused paste forgets them again.
        if !source.same_deck {
            // A result copied as values brings its base, which the source's results hold leased.
            source.bases.extend(source.results.values().filter_map(|t| t.base.as_ref().map(|b| b.id.clone())));
        }
        let valued = pasted_as_values(&source);
        let brought: BTreeSet<BaseId> =
            std::mem::take(&mut source.bases).into_iter().filter(|b| !self.doc.sources.bases.contains(b)).collect();
        self.doc.sources.bases.extend(brought.iter().cloned());
        self.sync_bases();
        if let Err(e) = self.apply_paste(label, ops, source) {
            self.forget_bases(&brought);
            return Err(e);
        }
        for name in valued {
            self.set_notice(self.localizer.tr_with("session-data-query-pasted-as-values", &[("data", name.into())]));
        }
        self.finish(prepared)
    }

    /// Garde les assets d'un collage venu d'un autre onglet, à la place des précédents.
    ///
    /// Les octets ne sont pas fiables : leur identifiant est recalculé (un bundle lu d'un
    /// zip ne vérifie pas le sien), et ils ne sont dessinés que par le rendu borné.
    /// Un asset que le bundle tient déjà n'est pas gardé : aucun collage ne le
    /// consommerait.
    pub(crate) fn carry_assets(&mut self, assets: Vec<Vec<u8>>) {
        let bundle = &self.doc.sources.assets;
        self.carried = assets
            .into_iter()
            .map(|bytes| (oku_slides_core::asset_id(&bytes), bytes))
            .filter(|(id, _)| !bundle.contains_key(id))
            .collect();
    }

    /// `ops` with one `AddData` before them per data of `source` their added views name.
    ///
    /// Every such data is copied, even into its own deck (`copy_data_ops`: a fresh id and a
    /// name unique in the deck), and every view names its copy; views on one data share one
    /// copy. A data the batch adds itself is left alone.
    ///
    /// # Errors
    ///
    /// `op-data-missing` for a view naming a data neither `source` nor the batch holds;
    /// `QueryPasteNoResult` for a query data with no ready result, pasted into another deck;
    /// `Prepare` for UI ops `check_external` refuses (sealed view data, a base, an overlay).
    pub(crate) fn with_pasted_data(&self, source: &PasteSource, mut ops: Vec<Op>) -> Result<Vec<Op>> {
        let reserved: BTreeSet<String> = ops.iter().filter_map(op_data_id).map(str::to_owned).collect();
        let mut refs = Vec::new();
        for op in &mut ops {
            each_added(op, &mut |_, params| refs.extend(view_refs(params)));
        }
        refs.retain(|id| !reserved.contains(id.as_str()));
        if let Some(missing) = refs.iter().find(|id| !source.data.contains_key(*id)) {
            return Err(data_missing(missing.as_str()));
        }
        let queries = if source.same_deck { Queries::Kept } else { Queries::Values(&source.results) };
        if !source.same_deck
            && let Some(data) = refs
                .iter()
                .filter_map(|id| source.data.get(id).map(|d| (id, d)))
                .find_map(|(id, d)| (d.sql().is_some() && !source.results.contains_key(id)).then_some(&d.name))
        {
            return Err(QueryPasteNoResultSnafu { data: data.clone() }.build().into());
        }
        let deck = self.doc.sources.deck();
        let (mut copies, map) =
            copy_data_ops(&source.data, &refs, deck, &reserved, &mut oku_slides_data::new_id, queries)
                .context(PrepareSnafu)?;
        for op in &mut ops {
            each_added(op, &mut |_, params| rename_refs(params, &|id| map.get(id).cloned()));
        }
        check_external(&ops).context(PrepareSnafu)?;
        copies.extend(ops);
        Ok(copies)
    }
}

/// The names of the query data `source` brings as values, in id order.
fn pasted_as_values(source: &PasteSource) -> Vec<String> {
    if source.same_deck {
        return Vec::new();
    }
    source.results.keys().filter_map(|id| source.data.get(id)).map(|d| d.name.clone()).collect()
}

#[cfg(test)]
#[path = "carried_assets_tests.rs"]
mod carried_assets_tests;
