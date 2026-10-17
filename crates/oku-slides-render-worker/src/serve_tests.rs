use std::io::{PipeReader, PipeWriter};
use std::path::PathBuf;
use std::thread::JoinHandle;

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
        match self.call(&Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets: vec![] }, &[]).0 {
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
    h.call(&Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets: vec![] }, &[]);
    let (resp, _) = h.call(&Request::Svg { slot: 1, generation: first, page: 0 }, &[]);
    assert_that!(matches!(resp, Response::Bytes)).is_true();
}

#[test]
fn serve_generations_grow_across_slots() {
    let mut h = Harness::new();
    let first = h.roadmap();
    h.call(&Request::OpenSlot { slot: 2, fonts: vec![] }, &[]);
    let second = match h.call(&Request::Render { slot: 2, deck: Box::new(fixture_deck()), assets: vec![] }, &[]).0 {
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
    let pack = WirePack { id: "casse".into(), files };
    let (resp, _) = h.call(&Request::SetWidgets { slot: 1, pack: Some(pack) }, &[]);
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
    let (resp, _) = h.call(&Request::Render { slot: 9, deck: Box::new(fixture_deck()), assets: vec![] }, &[]);
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

#[test]
fn worker_render_evicts_assets_outside_the_union_of_last_assets() {
    let mut w = Worker::default();
    for id in ["a", "b", "c"] {
        w.handle(Request::PutAsset { id: id.into() }, vec![1]);
    }
    w.handle(Request::OpenSlot { slot: 1, fonts: vec![] }, Vec::new());
    w.handle(Request::OpenSlot { slot: 2, fonts: vec![] }, Vec::new());
    w.handle(Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets: vec!["a".into()] }, Vec::new());
    // Le second emplacement n'a pas encore rendu : `b` n'est dans aucun `last_assets` mais `a` reste.
    assert_that!(w.assets.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec!["a".to_string()]);
    w.handle(Request::PutAsset { id: "b".into() }, vec![2]);
    w.handle(Request::Render { slot: 2, deck: Box::new(fixture_deck()), assets: vec!["b".into()] }, Vec::new());
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
    let req = Request::ProbeFile { name: "casse".into(), source: "#let widget = (".into(), common: None };
    let (reply, _) = worker.handle(req, Vec::new());
    let Response::FileProbe { probe } = reply else { panic!("réponse inattendue : {reply:?}") };
    assert_that!(probe.is_valid()).is_false();
}

#[test]
fn set_widgets_with_a_bad_widget_file_name_returns_setup_error() {
    let mut worker = Worker::default();
    worker.handle(Request::OpenSlot { slot: 1, fonts: vec![] }, Vec::new());
    let files = std::collections::BTreeMap::from([("Casse".into(), "#let widget = (".into())]);
    let pack = Some(WirePack { id: "p".into(), files });
    let (reply, _) = worker.handle(Request::SetWidgets { slot: 1, pack }, Vec::new());
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
    let (resp, _) = h.call(&Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets: vec![] }, &[]);
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
    let req = Request::ProbeFile { name: "w".into(), source, common: None };
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
    let (resp, _) = h.call(&Request::Hello { version: crate::protocol::PROTOCOL_VERSION + 1, canary: None }, &[]);
    assert_that!(matches!(resp, Response::Hello { version, .. } if version == crate::protocol::PROTOCOL_VERSION))
        .is_true();
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
    let (resp, _) = h.call(&Request::Render { slot: 1, deck: Box::new(fixture_deck()), assets: vec![] }, &[]);
    assert_that!(matches!(resp, Response::Compiled(_))).is_true();
}

#[test]
fn hello_without_canary_answers_none() {
    let mut h = Harness::new();
    let (resp, _) = h.call(&Request::Hello { version: PROTOCOL_VERSION, canary: None }, &[]);
    assert_that!(matches!(resp, Response::Hello { canary: None, .. })).is_true();
}

#[test]
fn hello_with_a_writable_canary_answers_written() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    let canary = Some(dir.path().join("c"));
    let (resp, _) = h.call(&Request::Hello { version: PROTOCOL_VERSION, canary }, &[]);
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
