//! Le lien face à un faux travailleur (script shell) : autre protocole,
//! sortie prématurée, échec pendant l'envoi de la file différée.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{RenderErrorKind, WireError};
use speculoos::prelude::*;

use super::*;
use crate::client::{Deferred, RenderWorker};
use crate::frame::{FrameLimits, write_frame};

fn settings() -> RenderSettings {
    RenderSettings {
        timeout_ms: 30_000,
        probe_budget_ms: 30_000,
        memory_mib: 4096,
        max_frame_mib: 64,
        max_widget_kib: 256,
        max_widgets: 64,
    }
}

/// Un faux travailleur : lit la requête `Hello`, répond `reply`, puis exécute `then`.
struct Fake {
    _dir: tempfile::TempDir,
    program: PathBuf,
}

impl Fake {
    fn new(reply: &Response, then: &str) -> Self {
        let dir = tempfile::Builder::new().prefix("oku-slides-fake-worker-").tempdir().unwrap();
        let mut hello = Vec::new();
        let req = Request::Hello { version: PROTOCOL_VERSION, canary: None };
        write_frame(&mut hello, &req, &[], FrameLimits::uniform(usize::MAX)).unwrap();
        let mut answer = Vec::new();
        write_frame(&mut answer, reply, &[], FrameLimits::uniform(usize::MAX)).unwrap();
        let answer_path = dir.path().join("reply.bin");
        std::fs::write(&answer_path, answer).unwrap();
        let program = dir.path().join("worker.sh");
        let body = format!("#!/bin/sh\nhead -c {} >/dev/null\ncat '{}'\n{then}\n", hello.len(), answer_path.display());
        std::fs::write(&program, body).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        Self { _dir: dir, program }
    }

    /// Répond `Hello` dans la bonne version, puis sort en 7 à la requête suivante.
    fn exiting_at_the_next_request() -> Self {
        Self::new(&Response::Hello { version: PROTOCOL_VERSION, canary: None }, "head -c 1 >/dev/null\nexit 7")
    }

    fn launch(&self) -> RenderLaunch {
        RenderLaunch::Process { program: self.program.clone() }
    }
}

fn set_language() -> Request {
    Request::SetLanguage { locale: "fr".into() }
}

#[test]
fn link_to_a_worker_of_another_protocol_version_is_refused_and_dropped() {
    let fake = Fake::new(&Response::Hello { version: 999, canary: None }, "cat >/dev/null");
    let mut link = Link::new(fake.launch(), settings());
    link.unsandboxed = true;
    let e = link.call(&set_language(), &[]).unwrap_err();
    assert_that!(e.kind()).is_equal_to(RenderErrorKind::Protocol);
    assert_that!(e.to_string()).contains("999");
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("render-protocol"));
    assert_that!(link.connected()).is_false();
}

#[test]
fn link_to_a_worker_older_than_the_handshake_is_refused() {
    // Un travailleur sans `Hello` répond une erreur de trame à la requête inconnue.
    let reply = Response::Error { error: WireError::Frame { message: "unknown op".into() } };
    let fake = Fake::new(&reply, "cat >/dev/null");
    let mut link = Link::new(fake.launch(), settings());
    link.unsandboxed = true;
    assert_that!(link.call(&set_language(), &[]).unwrap_err().kind()).is_equal_to(RenderErrorKind::Protocol);
}

#[test]
fn link_to_a_worker_that_exits_reports_its_exit_status() {
    let fake = Fake::exiting_at_the_next_request();
    let mut link = Link::new(fake.launch(), settings());
    link.unsandboxed = true;
    let e = link.call(&set_language(), &[]).unwrap_err();
    assert_that!(e.kind()).is_equal_to(RenderErrorKind::WorkerLost);
    assert_that!(e.to_string()).contains("exit status: 7");
}

#[test]
fn bounded_failure_while_flushing_the_deferred_queue_blames_the_pack() {
    let fake = Fake::exiting_at_the_next_request();
    let w = RenderWorker::new(fake.launch(), settings());
    w.link().unsandboxed = true;
    w.link().ensure_connected().unwrap();
    w.1.defer(Deferred::Close { slot: 1 });
    let pack = WirePack { id: "p".into(), files: BTreeMap::new() };
    let mut link = w.link();
    let e = w.prepare_for(&mut link, Some(&pack)).unwrap_err();
    assert_that!(e.kind()).is_equal_to(RenderErrorKind::WorkerLost);
    assert_that!(link.faulty_packs.contains(&fingerprint_hex(&pack))).is_true();
}
