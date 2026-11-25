use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{KnowledgeFile, WidgetRegistry};
use oku_slides_data::limits::{MAX_DATA, MAX_DECK_OVERLAY, MAX_DECK_ROW_IDS, MAX_OVERLAY, MAX_ROWS};
use oku_slides_data::test_support::{based_table, set_cell, sized};
use oku_slides_data::{Data, DataCell, DataId, DataKind, Table};
use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::bases::BaseStore;
use oku_slides_platform::config::RenderSettings;
use oku_slides_platform::storage::Storage;
use oku_slides_render::Fonts;
use speculoos::prelude::*;

use super::*;
use crate::client::RenderWorker;
use crate::test_packs::tiny_deck;

fn worker() -> RenderWorker {
    RenderWorker::new(
        RenderLaunch::InProcess,
        RenderSettings {
            timeout_ms: 120_000,
            probe_budget_ms: 30_000,
            memory_mib: 4096,
            max_frame_mib: 512,
            max_widget_kib: 256,
            max_widgets: 64,
            max_guide_chars: 4000,
            data_timeout_ms: 120_000,
            data_memory_mib: 4096,
        },
    )
}

fn data(name: &str, table: Table) -> Arc<Data> {
    Arc::new(Data { name: name.into(), label: None, inline: false, kind: DataKind::Manual, table })
}

fn id(name: &str) -> DataId {
    DataId::try_from(name.to_owned()).unwrap()
}

/// A deck at the data bounds: `MAX_DATA` data holding `MAX_DECK_ROW_IDS` row ids and
/// `MAX_DECK_OVERLAY` overlay cells, the first at the per-data bounds (`MAX_ROWS`, `MAX_OVERLAY`).
fn worst_case_deck() -> oku_slides_core::Deck {
    let mut deck = tiny_deck();
    let mut big = sized(MAX_ROWS, 4);
    for n in 0..MAX_OVERLAY {
        #[expect(clippy::cast_precision_loss, reason = "test values stay far below 2^53")]
        set_cell(&mut big, &format!("r{}", n / 4), &format!("c{}", n % 4), DataCell::Number(n as f64));
    }
    deck.data.insert(id("big"), data("big", big));
    let others = MAX_DATA - 1;
    let (rows, cells) = ((MAX_DECK_ROW_IDS - MAX_ROWS) / others, (MAX_DECK_OVERLAY - MAX_OVERLAY) / others);
    for i in 0..others {
        let mut table = sized(rows, 2);
        for n in 0..cells {
            set_cell(&mut table, &format!("r{}", n / 2), &format!("c{}", n % 2), DataCell::Text("texte".into()));
        }
        deck.data.insert(id(&format!("d{i}")), data(&format!("d{i}"), table));
    }
    deck
}

/// → after the first render sent every data whole, a one-cell edit of the largest one sends
/// only frames under 64 KiB (the worker caches the data; the edit goes as a `CellPatch`).
#[test]
fn one_cell_edit_sends_frame_under_64_kib() {
    let w = worker();
    let mut slot = w.slot(&Fonts::default(), &Localizer::new(Some("fr")), Arc::new(WidgetRegistry::default())).unwrap();
    let mut deck = worst_case_deck();
    slot.render(&deck, &BTreeMap::new()).unwrap();
    let first = std::mem::take(&mut w.link().written);
    let big = deck.data.get_mut("big").unwrap();
    set_cell(&mut Arc::make_mut(big).table, "r7", "c1", DataCell::Number(-1.0));

    slot.render(&deck, &BTreeMap::new()).unwrap();

    let second = std::mem::take(&mut w.link().written);
    assert_that!(first.iter().max().copied()).is_some().is_greater_than(1 << 20);
    assert_that!(second).is_not_empty();
    assert_that!(second.iter().max().copied()).is_some().is_less_than(64 << 10);
}

/// A slide with a chart and a table, both views of data `d` holding `table`.
fn chart_and_table(table: Table) -> oku_slides_core::Deck {
    let frame = |y: u32| serde_json::json!({ "x": 0, "y": y, "w": 800, "h": 200 });
    let json = serde_json::json!({
        "version": oku_slides_core::DECK_VERSION,
        "slides": [{ "id": "s", "kind": oku_slides_core::kinds::essentiel::BLANK, "params": { "elements": [
            { "id": "g", "kind": oku_slides_core::kinds::base::CHART, "frame": frame(0), "params": { "option": {
                "data": "d", "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } } },
            { "id": "t", "kind": oku_slides_core::kinds::base::TABLE, "frame": frame(250),
                "params": { "data": { "data": "d" } } }
        ] } }]
    });
    let mut deck = oku_slides_core::Deck::from_json(&json.to_string()).unwrap();
    deck.data.insert(id("d"), data("d", table));
    deck
}

/// A slot of `w` with the built-in widgets.
fn builtin_slot(w: &RenderWorker) -> crate::RenderSlot {
    let set = oku_slides_render::builtin_pack_set().unwrap();
    let registry = Arc::new(WidgetRegistry::default());
    let mut slot = w.slot(&Fonts::default(), &Localizer::new(Some("fr")), registry.clone()).unwrap();
    slot.set_widgets(&set, registry).unwrap();
    let defaults = oku_slides_render::builtin_component_defaults().unwrap();
    let layer = oku_slides_core::tokens::theme_layer(defaults.iter().map(|(ns, d)| (ns, d.clone())), []);
    slot.set_theme(Some(&layer)).unwrap();
    slot
}

fn page(slot: &mut crate::RenderSlot, deck: &oku_slides_core::Deck) -> String {
    slot.render(deck, &BTreeMap::new()).unwrap().svg(0).unwrap()
}

/// → an overlay edit over a compacted data (a base) reaches both views: the page equals the one
/// drawn from the same values held as cells, and differs from the page before the edit.
#[test]
fn cell_edit_on_compacted_data_reaches_chart_and_table() {
    let dir = tempfile::tempdir().unwrap();
    let store = BaseStore::new(Storage::local().unwrap(), dir.path(), 1 << 30).unwrap();
    let (based, file) = based_table(3, 2);
    let base = store.adopt(file.path(), 1 << 30).unwrap();
    let w = worker().with_bases(store);
    w.set_known_bases(std::iter::once(base).collect());
    let mut deck = chart_and_table(based);
    let mut slot = builtin_slot(&w);
    let before = page(&mut slot, &deck);
    let edited = deck.data.get_mut("d").unwrap();
    set_cell(&mut Arc::make_mut(edited).table, "r1", "c1", DataCell::Number(4242.0));

    let after = page(&mut slot, &deck);

    let mut cells = sized(3, 2);
    for (r, c) in [(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (2, 1)] {
        let value = if (r, c) == (1, 1) { 4242.0 } else { f64::from(r * 1000 + c) };
        set_cell(&mut cells, &format!("r{r}"), &format!("c{c}"), DataCell::Number(value));
    }
    let expected = page(&mut builtin_slot(&worker()), &chart_and_table(cells));
    assert_that!(after.contains("<image")).is_true();
    assert_that!(after == before).is_false();
    assert_that!(after == expected).is_true();
}

/// `tiny_deck` with a one-file knowledge bundle.
fn with_knowledge() -> Deck {
    let mut deck = tiny_deck();
    let file = KnowledgeFile { path: "/a.md".to_owned(), text: "---\ntype: Note\n---\nbody\n".to_owned() };
    deck.knowledge.insert(format!("{:032x}", 1), file);
    deck
}

#[test]
fn slot_render_request_has_no_knowledge() {
    let deck = with_knowledge();
    let Request::Render { deck: sent, .. } = render_request(7, &deck, Vec::new()) else {
        panic!("not a render request");
    };
    assert_that!(sent.knowledge.is_empty()).is_true();
    assert_that!(*sent).is_equal_to(deck.without_knowledge());
}

#[test]
fn render_same_pages_with_or_without_knowledge() {
    let worker = RenderWorker::new(
        RenderLaunch::InProcess,
        RenderSettings {
            timeout_ms: 10_000,
            probe_budget_ms: 10_000,
            memory_mib: 4096,
            max_frame_mib: 64,
            max_widget_kib: 256,
            max_widgets: 64,
            max_guide_chars: 4000,
            data_timeout_ms: 10_000,
            data_memory_mib: 4096,
        },
    );
    let mut slot = worker.slot(&Fonts::default(), &Localizer::new(Some("en")), Arc::default()).unwrap();
    let hashes = |c: &RemoteCompiled| (0..c.page_count()).map(|i| c.page_hash(i)).collect::<Vec<_>>();
    let with = slot.render(&with_knowledge(), &BTreeMap::new()).unwrap();
    let without = slot.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    assert_that!(with.page_count()).is_greater_than(0);
    assert_that!(hashes(&with)).is_equal_to(hashes(&without));
}
