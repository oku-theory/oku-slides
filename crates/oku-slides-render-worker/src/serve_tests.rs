use std::io::{PipeReader, PipeWriter};
use std::path::PathBuf;
use std::thread::JoinHandle;

use oku_slides_core::PackKind;
use speculoos::prelude::*;

use super::*;
use crate::frame::{FrameLimits, read_frame, write_frame};
use crate::protocol::{Canary, Request, Response};

const MAX: usize = 64 * 1024 * 1024;

struct Harness {
    tx: PipeWriter,
    rx: PipeReader,
    _worker: JoinHandle<()>,
}

impl Harness {
    fn new() -> Self {
        Self::with_work(None)
    }

    fn with_work(work: Option<PathBuf>) -> Self {
        let (req_r, req_w) = std::io::pipe().unwrap();
        let (resp_r, resp_w) = std::io::pipe().unwrap();
        let worker = std::thread::spawn(move || {
            let _ = serve_watched(req_r, resp_w, MAX, &Busy::default(), work);
        });
        Self { tx: req_w, rx: resp_r, _worker: worker }
    }

    fn call(&mut self, req: &Request, payload: &[u8]) -> (Response, Vec<u8>) {
        write_frame(&mut self.tx, req, payload, FrameLimits::uniform(usize::MAX)).unwrap();
        let f = read_frame(&mut self.rx, FrameLimits::uniform(MAX)).unwrap().unwrap();
        (serde_json::from_slice(&f.header).unwrap(), f.payload)
    }

    fn roadmap(&mut self) -> u64 {
        assert!(matches!(self.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]).0, Response::Ok));
        match self
            .call(
                &Request::Render {
                    slot: 1,
                    deck: Box::new(fixture_deck()),
                    assets: vec![],
                    data: vec![],
                    results: vec![],
                },
                &[],
            )
            .0
        {
            Response::Compiled(h) => h.generation,
            other => panic!("{other:?}"),
        }
    }
}

fn fixture_deck() -> oku_slides_core::Deck {
    let json = include_str!("../../../fixtures/roadmap.json");
    oku_slides_core::Deck::from_json(json).unwrap()
}

#[test]
fn serve_render_then_svg_of_same_generation_returns_svg() {
    let mut h = Harness::new();
    let generation = h.roadmap();
    let (resp, svg) = h.call(&Request::Svg { slot: 1, generation, page: 0 }, &[]);
    assert_that!(matches!(resp, Response::Bytes)).is_true();
    assert_that!(String::from_utf8(svg).unwrap()).starts_with("<svg");
}

#[test]
fn serve_svg_of_released_generation_returns_stale() {
    let mut h = Harness::new();
    let generation = h.roadmap();
    h.call(&Request::Release { slot: 1, generations: vec![generation] }, &[]);
    let (resp, _) = h.call(&Request::Svg { slot: 1, generation, page: 0 }, &[]);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Stale })).is_true();
}

#[test]
fn serve_keeps_an_older_generation_until_released() {
    let mut h = Harness::new();
    let first = h.roadmap();
    h.call(
        &Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets: vec![], data: vec![], results: vec![] },
        &[],
    );
    let (resp, _) = h.call(&Request::Svg { slot: 1, generation: first, page: 0 }, &[]);
    assert_that!(matches!(resp, Response::Bytes)).is_true();
}

#[test]
fn serve_generations_grow_across_slots() {
    let mut h = Harness::new();
    let first = h.roadmap();
    h.call(&Request::OpenSlot { slot: 2, fonts: vec![] }, &[]);
    let second = match h
        .call(
            &Request::Render { slot: 2, deck: Box::new(fixture_deck()), assets: vec![], data: vec![], results: vec![] },
            &[],
        )
        .0
    {
        Response::Compiled(h) => h.generation,
        other => panic!("{other:?}"),
    };
    assert_that!(second).is_greater_than(first);
}

#[test]
fn serve_set_widgets_of_broken_pack_returns_registry_compile_error() {
    let mut h = Harness::new();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let files = std::collections::BTreeMap::from([("casse".into(), "#let widget = (".into())]);
    let pack = WirePack { id: "casse".into(), namespace: "casse".into(), kind: PackKind::Widgets, files };
    let (resp, _) = h.call(&Request::SetWidgets { slot: 1, packs: vec![pack] }, &[]);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::RegistryCompile { .. } }))
        .is_true();
}

#[test]
fn serve_whole_pack_probe_request_is_not_understood() {
    let mut h = Harness::new();
    let req = serde_json::json!({ "op": "probe", "pack": { "id": "p", "files": {} } });
    write_frame(&mut h.tx, &req, &[], FrameLimits::uniform(usize::MAX)).unwrap();
    let f = read_frame(&mut h.rx, FrameLimits::uniform(MAX)).unwrap().unwrap();
    let resp: Response = serde_json::from_slice(&f.header).unwrap();
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Frame { .. } })).is_true();
}

#[test]
fn serve_open_slot_with_unknown_font_returns_setup_error() {
    let mut h = Harness::new();
    let (resp, _) = h.call(&Request::OpenSlot { slot: 1, fonts: vec!["inconnue".into()] }, &[]);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
}

#[test]
fn serve_render_on_unknown_slot_returns_error_and_keeps_serving() {
    let mut h = Harness::new();
    let (resp, _) = h.call(
        &Request::Render { slot: 9, deck: Box::new(fixture_deck()), assets: vec![], data: vec![], results: vec![] },
        &[],
    );
    assert_that!(matches!(resp, Response::Error { .. })).is_true();
    let (resp, _) = h.call(&Request::SetLanguage { locale: "fr".into() }, &[]);
    assert_that!(matches!(resp, Response::Ok)).is_true();
}

#[test]
fn serve_unreadable_request_header_returns_frame_error() {
    let mut h = Harness::new();
    write_frame(&mut h.tx, &serde_json::json!({ "op": "inconnue" }), &[], FrameLimits::uniform(usize::MAX)).unwrap();
    let f = read_frame(&mut h.rx, FrameLimits::uniform(MAX)).unwrap().unwrap();
    let resp: Response = serde_json::from_slice(&f.header).unwrap();
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Frame { .. } })).is_true();
}

#[test]
fn serve_truncated_frame_returns_frame_error() {
    let mut out = Vec::new();
    let result = serve(&[0u8, 0, 0][..], &mut out, MAX);
    assert_that!(result.is_err()).is_true();
}

/// t-1295: a render listing an asset the worker never received answers its id, renders
/// nothing and keeps the others, instead of drawing a missing-image frame.
#[test]
fn worker_render_without_a_listed_asset_answers_missing_assets() {
    let mut w = Worker::default();
    w.handle(Request::PutAsset { id: "a".into() }, vec![1]);
    w.handle(Request::OpenSlot { slot: 1, fonts: vec![] }, Vec::new());
    let assets = vec!["a".into(), "b".into()];
    let (resp, _) = w.handle(
        Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets, data: vec![], results: vec![] },
        Vec::new(),
    );
    let Response::MissingAssets { ids } = resp else { panic!("{resp:?}") };
    assert_that!(ids).is_equal_to(vec!["b".to_string()]);
    assert_that!(w.next_generation).is_equal_to(0);
    assert_that!(w.assets.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec!["a".to_string()]);
}

#[test]
fn worker_render_evicts_assets_outside_the_union_of_last_assets() {
    let mut w = Worker::default();
    for id in ["a", "b", "c"] {
        w.handle(Request::PutAsset { id: id.into() }, vec![1]);
    }
    w.handle(Request::OpenSlot { slot: 1, fonts: vec![] }, Vec::new());
    w.handle(Request::OpenSlot { slot: 2, fonts: vec![] }, Vec::new());
    w.handle(
        Request::Render {
            slot: 1,
            deck: Box::new(fixture_deck()),
            assets: vec!["a".into()],
            data: vec![],
            results: vec![],
        },
        Vec::new(),
    );
    // Le second emplacement n'a pas encore rendu : `b` n'est dans aucun `last_assets` mais `a` reste.
    assert_that!(w.assets.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec!["a".to_string()]);
    w.handle(Request::PutAsset { id: "b".into() }, vec![2]);
    w.handle(
        Request::Render {
            slot: 2,
            deck: Box::new(fixture_deck()),
            assets: vec!["b".into()],
            data: vec![],
            results: vec![],
        },
        Vec::new(),
    );
    assert_that!(w.assets.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec!["a".to_string(), "b".to_string()]);
    w.handle(Request::CloseSlot { slot: 1 }, Vec::new());
    assert_that!(w.assets.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec!["b".to_string()]);
}

fn os_args(args: &[&str]) -> Vec<std::ffi::OsString> {
    args.iter().map(std::ffi::OsString::from).collect()
}

#[test]
fn worker_args_reads_frame_size_and_host_bounds() {
    let args =
        os_args(&["--render-worker", "--max-frame-bytes", "1024", "--timeout-ms", "500", "--memory-bytes", "2048"]);
    let parsed = worker_args(&args).unwrap();
    assert_that!(parsed.max_frame).is_equal_to(1024);
    assert_that!(parsed.limits).is_equal_to(crate::watchdog::Limits::from_host(Some(500), Some(2048)));
}

#[test]
fn worker_args_without_bounds_keeps_defaults() {
    let parsed = worker_args(&os_args(&["--render-worker"])).unwrap();
    assert_that!(parsed.max_frame).is_equal_to(512 * 1024 * 1024);
    assert_that!(parsed.limits).is_equal_to(crate::watchdog::Limits::default());
}

#[test]
fn worker_args_of_an_ordinary_launch_is_none() {
    assert_that!(worker_args(&os_args(&["deck.oku-slides.pdf"]))).is_none();
}

#[test]
fn probe_file_request_returns_the_file_probe() {
    let mut worker = Worker::default();
    let req = Request::ProbeFile {
        kind: PackKind::Widgets,
        name: "casse".into(),
        source: "#let widget = (".into(),
        common: None,
    };
    let (reply, _) = worker.handle(req, Vec::new());
    let Response::FileProbe { probe } = reply else { panic!("réponse inattendue : {reply:?}") };
    assert_that!(probe.is_valid()).is_false();
}

#[test]
fn set_widgets_with_a_bad_widget_file_name_returns_setup_error() {
    let mut worker = Worker::default();
    worker.handle(Request::OpenSlot { slot: 1, fonts: vec![] }, Vec::new());
    let files = std::collections::BTreeMap::from([("Casse".into(), "#let widget = (".into())]);
    let pack = WirePack { id: "p".into(), namespace: "p".into(), kind: PackKind::Widgets, files };
    let (reply, _) = worker.handle(Request::SetWidgets { slot: 1, packs: vec![pack] }, Vec::new());
    assert_that!(matches!(reply, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
}

#[test]
fn serve_set_theme_tokens_reach_the_render() {
    let mut h = Harness::new();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let tokens = serde_json::json!({ "color": { "bg": "#000000" } });
    let tokens = tokens.as_object().cloned();
    let (resp, _) = h.call(&Request::SetTheme { slot: 1, tokens }, &[]);
    assert_that!(matches!(resp, Response::Ok)).is_true();
    let (resp, _) = h.call(
        &Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets: vec![], data: vec![], results: vec![] },
        &[],
    );
    let Response::Compiled(header) = resp else { panic!("{resp:?}") };
    let (_, svg) = h.call(&Request::Svg { slot: 1, generation: header.generation, page: 0 }, &[]);
    assert_that!(String::from_utf8(svg).unwrap()).contains("#000000");
}

#[test]
fn serve_set_theme_on_unknown_slot_returns_setup_error() {
    let mut h = Harness::new();
    let (resp, _) = h.call(&Request::SetTheme { slot: 9, tokens: None }, &[]);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
}

#[test]
fn serve_refuses_a_request_header_over_an_eighth_of_the_frame_size() {
    let source = "x".repeat(200);
    let req = Request::ProbeFile { kind: PackKind::Widgets, name: "w".into(), source, common: None };
    let mut input = Vec::new();
    write_frame(&mut input, &req, &[], FrameLimits::uniform(usize::MAX)).unwrap();
    // 1024 octets par trame : l'en-tête (plus de 200) dépasse 1024 / 8.
    assert_that!(serve(input.as_slice(), Vec::new(), 1024)).is_err();
}

fn libreoffice_pptx() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import/libreoffice.pptx")).unwrap()
}

#[test]
fn serve_reference_then_png_returns_a_png() {
    let mut h = Harness::new();
    let reference = match h
        .call(
            &Request::Reference { witness: crate::ReferenceWitness::Office2Pdf, fonts: vec![], slides: None },
            &libreoffice_pptx(),
        )
        .0
    {
        Response::Reference { reference, pages, .. } => {
            assert_that!(pages).is_equal_to(3);
            reference
        }
        other => panic!("{other:?}"),
    };
    let (resp, png) = h.call(&Request::ReferencePng { reference, page: 0, width: 480 }, &[]);
    assert_that!(matches!(resp, Response::Bytes)).is_true();
    assert_that!(png[1..4].to_vec()).is_equal_to(b"PNG".to_vec());
}

#[test]
fn serve_png_of_released_reference_returns_stale() {
    let mut h = Harness::new();
    let Response::Reference { reference, .. } = h
        .call(
            &Request::Reference { witness: crate::ReferenceWitness::Office2Pdf, fonts: vec![], slides: None },
            &libreoffice_pptx(),
        )
        .0
    else {
        panic!("pas de référence")
    };
    h.call(&Request::ReleaseReference { reference }, &[]);
    let (resp, _) = h.call(&Request::ReferencePng { reference, page: 0, width: 480 }, &[]);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Stale })).is_true();
}

#[test]
fn serve_reference_with_unknown_font_returns_setup() {
    let mut h = Harness::new();
    let (resp, _) = h.call(
        &Request::Reference {
            witness: crate::ReferenceWitness::Office2Pdf,
            fonts: vec!["inconnue".into()],
            slides: None,
        },
        &libreoffice_pptx(),
    );
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
}

#[test]
fn serve_hello_answers_the_worker_protocol_version() {
    let mut h = Harness::new();
    let (resp, _) =
        h.call(&Request::Hello { version: crate::protocol::PROTOCOL_VERSION + 1, canary: None, bases: None }, &[]);
    assert_that!(matches!(resp, Response::Hello { version, .. } if version == crate::protocol::PROTOCOL_VERSION))
        .is_true();
}

/// Version 9 merges `ExtractDocument` (t-1755) with the data requests (t-1690): `Hello` answers it.
#[test]
fn protocol_version_matches_constant() {
    let mut h = Harness::new();
    let (resp, _) = h.call(&Request::Hello { version: PROTOCOL_VERSION, canary: None, bases: None }, &[]);
    assert_that!(PROTOCOL_VERSION).is_equal_to(10);
    assert_that!(matches!(resp, Response::Hello { version, .. } if version == PROTOCOL_VERSION)).is_true();
}

#[test]
fn serve_extract_document_answers_the_extraction_in_its_payload() {
    let mut h = Harness::new();
    let req = Request::ExtractDocument {
        kind: oku_slides_core::knowledge::DocumentType::Text,
        limits: crate::test_documents::limits(),
        read: crate::test_documents::read(),
        token: crate::test_documents::TOKEN.into(),
    };
    let (resp, payload) = h.call(&req, b"plain words");
    let Response::Extracted { markdown_len, images, .. } = resp else { panic!("{resp:?}") };
    assert_that!((markdown_len, images.len())).is_equal_to((payload.len(), 0));
    assert_that!(String::from_utf8(payload).unwrap()).contains("plain words");
}

#[test]
fn serve_drop_fonts_forgets_them_for_the_next_slots() {
    let mut h = Harness::new();
    let font = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts/carlito-400.ttf")).unwrap();
    h.call(&Request::PutFont { id: "carlito".into() }, &font);
    assert_that!(matches!(h.call(&Request::OpenSlot { slot: 1, fonts: vec!["carlito".into()] }, &[]).0, Response::Ok))
        .is_true();
    assert_that!(matches!(h.call(&Request::DropFonts { ids: vec!["carlito".into()] }, &[]).0, Response::Ok)).is_true();
    let (resp, _) = h.call(&Request::OpenSlot { slot: 2, fonts: vec!["carlito".into()] }, &[]);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
    // L'emplacement déjà ouvert garde sa copie : il rend encore.
    let (resp, _) = h.call(
        &Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets: vec![], data: vec![], results: vec![] },
        &[],
    );
    assert_that!(matches!(resp, Response::Compiled(_))).is_true();
}

#[test]
fn hello_without_canary_answers_none() {
    let mut h = Harness::new();
    let (resp, _) = h.call(&Request::Hello { version: PROTOCOL_VERSION, canary: None, bases: None }, &[]);
    assert_that!(matches!(resp, Response::Hello { canary: None, .. })).is_true();
}

#[test]
fn hello_with_a_writable_canary_answers_written() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    let canary = Some(dir.path().join("c"));
    let (resp, _) = h.call(&Request::Hello { version: PROTOCOL_VERSION, canary, bases: None }, &[]);
    assert_that!(matches!(resp, Response::Hello { canary: Some(Canary::Written), .. })).is_true();
}

#[test]
fn worker_args_read_the_work_dir() {
    let args: Vec<OsString> = ["--render-worker", "--work-dir", "/w"].into_iter().map(OsString::from).collect();
    assert_that!(worker_args(&args).unwrap().work).is_equal_to(Some(PathBuf::from("/w")));
}

#[test]
fn reference_empties_the_work_dir_afterwards() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("planted"), b"x").unwrap();
    let mut h = Harness::with_work(Some(work.path().to_owned()));
    let req = Request::Reference { witness: crate::ReferenceWitness::Office2Pdf, fonts: vec![], slides: None };
    // `wipe_work` suit toute réponse : une charge vide suffit, sans rendu office2pdf.
    let _ = h.call(&req, &[]);
    assert_that!(work.path().join("planted").exists()).is_false();
    assert_that!(work.path().exists()).is_true();
}

/// A one-slide deck whose chart reads data `d`, or a data `data` names.
fn bound_chart_deck(data: &str) -> oku_slides_core::Deck {
    let json = serde_json::json!({
        "version": oku_slides_core::DECK_VERSION,
        "data": { "d": { "name": "ventes", "kind": "manual", "table": {
            "generation": "g",
            "columns": [{ "id": "m", "name": "Mois", "type": "text" }, { "id": "v", "name": "Ventes", "type": "number" }],
            "rows": ["r1", "r2"],
            "cells": { "r1/m": "Jan", "r1/v": 3, "r2/m": "Fév", "r2/v": 5 }
        } } },
        "slides": [{ "id": "s", "kind": oku_slides_core::kinds::essentiel::BLANK, "params": { "elements": [{
            "id": "g", "kind": oku_slides_core::kinds::base::CHART, "frame": { "x": 0, "y": 0, "w": 800, "h": 450 },
            "params": { "option": { "data": data, "xAxis": { "type": "category" }, "series": [{ "type": "bar" }] } }
        }] } }]
    });
    oku_slides_core::Deck::from_json(&json.to_string()).unwrap()
}

/// Renders `deck` as the host does: each data with `PutData`, then the deck without them.
fn page_svg(h: &mut Harness, mut deck: oku_slides_core::Deck) -> String {
    let mut data = Vec::new();
    for (id, d) in std::mem::take(&mut deck.data) {
        let put = Request::PutData { slot: 1, id: id.as_str().to_owned(), base: None };
        assert_that!(matches!(h.call(&put, &serde_json::to_vec(&*d).unwrap()).0, Response::Ok)).is_true();
        data.push(id.as_str().to_owned());
    }
    let (resp, _) =
        h.call(&Request::Render { slot: 1, deck: Box::new(deck), assets: vec![], data, results: vec![] }, &[]);
    let Response::Compiled(header) = resp else { panic!("{resp:?}") };
    let (_, svg) = h.call(&Request::Svg { slot: 1, generation: header.generation, page: 0 }, &[]);
    String::from_utf8(svg).unwrap()
}

#[test]
fn worker_renders_bound_chart_through_set_deck() {
    let mut h = Harness::new();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let set = oku_slides_render::builtin_pack_set().unwrap();
    let packs = set.packs().iter().map(crate::client::wire).collect();
    assert_that!(matches!(h.call(&Request::SetWidgets { slot: 1, packs }, &[]).0, Response::Ok)).is_true();
    let defaults = oku_slides_render::builtin_component_defaults().unwrap();
    let layer = oku_slides_core::tokens::theme_layer(defaults.iter().map(|(ns, d)| (ns, d.clone())), []);
    h.call(&Request::SetTheme { slot: 1, tokens: Some(layer) }, &[]);
    // The chart is drawn as an embedded image; an unresolved view draws a failure frame instead.
    assert_that!(page_svg(&mut h, bound_chart_deck("d"))).contains("<image");
    assert_that!(page_svg(&mut h, bound_chart_deck("nowhere"))).does_not_contain("<image");
}

#[test]
fn serve_render_with_unsent_data_answers_missing_data() {
    let mut h = Harness::new();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let req = Request::Render {
        slot: 1,
        deck: Box::new(fixture_deck()),
        assets: vec![],
        data: vec!["sales".into()],
        results: vec![],
    };
    let (resp, _) = h.call(&req, &[]);
    assert_that!(matches!(resp, Response::MissingData { ref ids } if ids == &["sales".to_owned()])).is_true();
}

#[test]
fn serve_put_data_with_base_outside_bases_folder_is_refused() {
    let mut h = Harness::new();
    let bases = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let hello = Request::Hello {
        version: crate::protocol::PROTOCOL_VERSION,
        canary: None,
        bases: Some(bases.path().to_owned()),
    };
    assert_that!(matches!(h.call(&hello, &[]).0, Response::Hello { .. })).is_true();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let (table, _file) = oku_slides_data::test_support::based_table(2, 2);
    let data = oku_slides_data::Data {
        name: "sales".into(),
        label: None,
        inline: false,
        source: oku_slides_data::Source::Manual(table),
    };
    let json = serde_json::to_vec(&data).unwrap();
    let base = Some(elsewhere.path().join("ab.parquet"));
    let (resp, _) = h.call(&Request::PutData { slot: 1, id: "sales".into(), base }, &json);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
}

/// → a `PatchData` for an id the worker lacks is ignored: the next Render answers `MissingData`.
#[test]
fn serve_patch_data_of_unknown_id_leaves_it_missing() {
    let mut h = Harness::new();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let patch = oku_slides_data::CellPatch { set: vec![], removed: vec![] };
    let req = Request::PatchData { slot: 1, id: "sales".into() };
    assert_that!(matches!(h.call(&req, &serde_json::to_vec(&patch).unwrap()).0, Response::Ok)).is_true();
    let req = Request::Render {
        slot: 1,
        deck: Box::new(fixture_deck()),
        assets: vec![],
        data: vec!["sales".into()],
        results: vec![],
    };
    let (resp, _) = h.call(&req, &[]);
    assert_that!(matches!(resp, Response::MissingData { ref ids } if ids == &["sales".to_owned()])).is_true();
}

/// → a `PatchData` edits the cached data: the page equals one drawn from the edited data sent whole.
#[test]
fn serve_patch_data_edits_the_cached_data() {
    let mut h = Harness::new();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let set = oku_slides_render::builtin_pack_set().unwrap();
    let packs = set.packs().iter().map(crate::client::wire).collect();
    h.call(&Request::SetWidgets { slot: 1, packs }, &[]);
    let defaults = oku_slides_render::builtin_component_defaults().unwrap();
    let layer = oku_slides_core::tokens::theme_layer(defaults.iter().map(|(ns, d)| (ns, d.clone())), []);
    h.call(&Request::SetTheme { slot: 1, tokens: Some(layer) }, &[]);
    let older = bound_chart_deck("d");
    let mut newer = older.clone();
    let d = newer.data.get_mut("d").unwrap();
    oku_slides_data::test_support::set_cell(
        std::sync::Arc::make_mut(d).table_mut().unwrap(),
        "r2",
        "v",
        oku_slides_data::DataCell::Number(40.0),
    );
    let expected = page_svg(&mut h, newer.clone());
    page_svg(&mut h, older.clone());
    let patch = oku_slides_data::CellPatch::between(&older.data["d"], &newer.data["d"]).unwrap();
    let req = Request::PatchData { slot: 1, id: "d".into() };
    assert_that!(matches!(h.call(&req, &serde_json::to_vec(&patch).unwrap()).0, Response::Ok)).is_true();
    let mut deck = newer;
    deck.data.clear();
    let (resp, _) = h.call(
        &Request::Render { slot: 1, deck: Box::new(deck), assets: vec![], data: vec!["d".into()], results: vec![] },
        &[],
    );
    let Response::Compiled(header) = resp else { panic!("{resp:?}") };
    let (_, svg) = h.call(&Request::Svg { slot: 1, generation: header.generation, page: 0 }, &[]);
    assert_that!(String::from_utf8(svg).unwrap() == expected).is_true();
    assert_that!(expected.contains("<image")).is_true();
}

/// A worker that said `Hello` with `bases`, its work dir `work`, and a based data's JSON.
fn data_harness(bases: &std::path::Path, work: Option<PathBuf>) -> (Harness, Vec<u8>) {
    let mut h = Harness::with_work(work);
    let hello =
        Request::Hello { version: crate::protocol::PROTOCOL_VERSION, canary: None, bases: Some(bases.to_owned()) };
    assert_that!(matches!(h.call(&hello, &[]).0, Response::Hello { .. })).is_true();
    let data = oku_slides_data::Data {
        name: "sales".into(),
        label: None,
        inline: false,
        source: oku_slides_data::Source::Manual(oku_slides_data::test_support::sized(2, 2)),
    };
    (h, serde_json::to_vec(&data).unwrap())
}

/// → a compaction writing outside the bases and work folders is refused, with or without a
/// work folder, and leaves no file there (t-1690).
#[rstest::rstest]
#[case::without_work_dir(false)]
#[case::with_work_dir(true)]
fn serve_compact_data_out_outside_its_folders_is_refused(#[case] with_work: bool) {
    let bases = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let (mut h, json) = data_harness(bases.path(), with_work.then(|| work.path().to_owned()));
    let out = elsewhere.path().join("compact.parquet");
    let (resp, _) = h.call(&Request::CompactData { base: None, out: out.clone() }, &json);
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
    assert_that!(out.exists()).is_false();
}

/// → a compaction writes inside the bases folder, or inside the work folder when it has one.
#[rstest::rstest]
#[case::into_bases(false)]
#[case::into_work(true)]
fn serve_compact_data_out_inside_its_folders_is_written(#[case] into_work: bool) {
    let bases = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let (mut h, json) = data_harness(bases.path(), Some(work.path().to_owned()));
    let out = if into_work { work.path() } else { bases.path() }.join("compact.parquet");
    let (resp, _) = h.call(&Request::CompactData { base: None, out: out.clone() }, &json);
    assert_that!(matches!(resp, Response::Compacted { .. })).is_true();
    assert_that!(out.exists()).is_true();
}

/// A deck whose chart `g` reads query data `q` over data `d` of [`bound_chart_deck`].
fn query_chart_deck() -> oku_slides_core::Deck {
    let mut deck = bound_chart_deck("q");
    let q = oku_slides_data::Data {
        name: "top".into(),
        label: None,
        inline: false,
        source: oku_slides_data::Source::Query { sql: "SELECT * FROM ventes".into() },
    };
    deck.data.insert(DataId::try_from("q".to_owned()).unwrap(), std::sync::Arc::new(q));
    deck
}

/// Renders `deck` as the host does, query data `q` reading `result` when there is one.
fn query_page_svg(h: &mut Harness, mut deck: oku_slides_core::Deck, result: Option<&Table>) -> String {
    let mut data = Vec::new();
    for (id, d) in std::mem::take(&mut deck.data) {
        let put = Request::PutData { slot: 1, id: id.as_str().to_owned(), base: None };
        assert_that!(matches!(h.call(&put, &serde_json::to_vec(&*d).unwrap()).0, Response::Ok)).is_true();
        data.push(id.as_str().to_owned());
    }
    let mut results = Vec::new();
    if let Some(table) = result {
        let put = Request::PutResult { slot: 1, id: "q".into(), base: None };
        assert_that!(matches!(h.call(&put, &serde_json::to_vec(table).unwrap()).0, Response::Ok)).is_true();
        results.push("q".to_owned());
    }
    let (resp, _) = h.call(&Request::Render { slot: 1, deck: Box::new(deck), assets: vec![], data, results }, &[]);
    let Response::Compiled(header) = resp else { panic!("{resp:?}") };
    let (_, svg) = h.call(&Request::Svg { slot: 1, generation: header.generation, page: 0 }, &[]);
    String::from_utf8(svg).unwrap()
}

#[test]
fn serve_query_view_draws_its_put_result_else_failure_frame() {
    let mut h = Harness::new();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let set = oku_slides_render::builtin_pack_set().unwrap();
    let packs = set.packs().iter().map(crate::client::wire).collect();
    assert_that!(matches!(h.call(&Request::SetWidgets { slot: 1, packs }, &[]).0, Response::Ok)).is_true();
    let defaults = oku_slides_render::builtin_component_defaults().unwrap();
    let layer = oku_slides_core::tokens::theme_layer(defaults.iter().map(|(ns, d)| (ns, d.clone())), []);
    h.call(&Request::SetTheme { slot: 1, tokens: Some(layer) }, &[]);
    let deck = query_chart_deck();
    let result = deck.data.get("d").and_then(|d| d.table()).cloned().unwrap();
    assert_that!(query_page_svg(&mut h, deck.clone(), Some(&result))).contains("<image");
    assert_that!(query_page_svg(&mut h, deck, None)).does_not_contain("<image");
}

#[test]
fn serve_render_after_drop_data_answers_missing_result() {
    let mut h = Harness::new();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let table = query_chart_deck().data.get("d").and_then(|d| d.table()).cloned().unwrap();
    let put = Request::PutResult { slot: 1, id: "q".into(), base: None };
    assert_that!(matches!(h.call(&put, &serde_json::to_vec(&table).unwrap()).0, Response::Ok)).is_true();
    h.call(&Request::DropData { slot: 1, ids: vec!["q".into()] }, &[]);
    let req = Request::Render {
        slot: 1,
        deck: Box::new(fixture_deck()),
        assets: vec![],
        data: vec![],
        results: vec!["q".into()],
    };
    let (resp, _) = h.call(&req, &[]);
    assert_that!(matches!(resp, Response::MissingData { ref ids } if ids == &["q".to_owned()])).is_true();
}

#[test]
fn serve_put_result_with_base_outside_bases_folder_is_refused() {
    let mut h = Harness::new();
    let bases = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let hello = Request::Hello {
        version: crate::protocol::PROTOCOL_VERSION,
        canary: None,
        bases: Some(bases.path().to_owned()),
    };
    assert_that!(matches!(h.call(&hello, &[]).0, Response::Hello { .. })).is_true();
    h.call(&Request::OpenSlot { slot: 1, fonts: vec![] }, &[]);
    let (table, _file) = oku_slides_data::test_support::based_table(2, 2);
    let base = Some(elsewhere.path().join("ab.parquet"));
    let (resp, _) = h.call(&Request::PutResult { slot: 1, id: "q".into(), base }, &serde_json::to_vec(&table).unwrap());
    assert_that!(matches!(resp, Response::Error { error: oku_slides_render::WireError::Setup { .. } })).is_true();
}
