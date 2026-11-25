use std::collections::BTreeMap;

use oku_slides_core::KnowledgeFile;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::Fonts;
use speculoos::prelude::*;

use super::*;
use crate::client::RenderWorker;
use crate::test_packs::tiny_deck;

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
        },
    );
    let mut slot = worker.slot(&Fonts::default(), &Localizer::new(Some("en")), Arc::default()).unwrap();
    let hashes = |c: &RemoteCompiled| (0..c.page_count()).map(|i| c.page_hash(i)).collect::<Vec<_>>();
    let with = slot.render(&with_knowledge(), &BTreeMap::new()).unwrap();
    let without = slot.render(&tiny_deck(), &BTreeMap::new()).unwrap();
    assert_that!(with.page_count()).is_greater_than(0);
    assert_that!(hashes(&with)).is_equal_to(hashes(&without));
}
