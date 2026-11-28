//! t-1690: a one-cell edit resent the whole data to the worker, past 64 KiB for a large one.
//! It now goes as a cell patch: the page after the edit must equal a fresh render of it.

use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::WidgetRegistry;
use oku_slides_data::DataCell;
use oku_slides_data::test_support::set_cell;
use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::Settings;
use oku_slides_render::Fonts;
use oku_slides_render_worker::{RenderSlot, RenderWorker};
use speculoos::prelude::*;

fn deck() -> oku_slides_core::Deck {
    let json = serde_json::json!({
        "version": oku_slides_core::DECK_VERSION,
        "data": { "d": { "name": "ventes", "kind": "manual", "table": {
            "generation": "g",
            "columns": [{ "id": "m", "name": "Mois", "type": "text" }, { "id": "v", "name": "Ventes", "type": "number" }],
            "rows": ["r1", "r2"],
            "cells": { "r1/m": "Jan", "r1/v": 3, "r2/m": "Fév", "r2/v": 5 }
        } } },
        "slides": [{ "id": "s", "kind": oku_slides_core::kinds::essentiel::BLANK, "params": { "elements": [
            { "id": "t", "kind": oku_slides_core::kinds::base::TABLE, "frame": { "x": 0, "y": 0, "w": 800, "h": 450 },
              "params": { "data": { "data": "d" } } }
        ] } }]
    });
    oku_slides_core::Deck::from_json(&json.to_string()).unwrap()
}

fn slot(w: &RenderWorker) -> RenderSlot {
    let set = oku_slides_render::builtin_pack_set().unwrap();
    let registry = Arc::new(WidgetRegistry::default());
    let mut slot = w.slot(&Fonts::default(), &Localizer::new(Some("fr")), registry.clone()).unwrap();
    slot.set_widgets(&set, registry).unwrap();
    let defaults = oku_slides_render::builtin_component_defaults().unwrap();
    let layer = oku_slides_core::tokens::theme_layer(defaults.iter().map(|(ns, d)| (ns, d.clone())), []);
    slot.set_theme(Some(&layer)).unwrap();
    slot
}

fn worker() -> RenderWorker {
    RenderWorker::new(RenderLaunch::InProcess, Settings::defaults().unwrap().render)
}

#[test]
fn one_cell_edit_after_render_reaches_the_page() {
    let w = worker();
    let mut s = slot(&w);
    let mut deck = deck();
    let before = s.render(&deck, &BTreeMap::new()).unwrap().svg(0).unwrap();
    let d = deck.data.get_mut("d").unwrap();
    set_cell(
        Arc::make_mut(d).table_mut().unwrap_or_else(|| panic!("a manual data")),
        "r2",
        "v",
        DataCell::Number(4242.0),
    );

    let after = s.render(&deck, &BTreeMap::new()).unwrap().svg(0).unwrap();

    let fresh = slot(&worker()).render(&deck, &BTreeMap::new()).unwrap().svg(0).unwrap();
    assert_that!(after == before).is_false();
    assert_that!(after == fresh).is_true();
}
