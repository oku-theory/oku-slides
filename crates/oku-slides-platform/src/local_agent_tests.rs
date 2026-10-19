use std::path::PathBuf;

use rstest::rstest;
use speculoos::prelude::*;

use super::{AGENT_MODEL, AGENT_MODEL_SIZE, ReadyEngine, Target, Unavailable, resolve};
use crate::config::LocalEngine;

fn model() -> PathBuf {
    PathBuf::from("/data/models/agent/Qwen3-4B-Instruct-2507-Q4_K_M.gguf")
}

#[rstest]
#[case::auto_ready(LocalEngine::Auto, Target::AppleSilicon, true, Ok(ReadyEngine::Gguf { model: model() }))]
#[case::gguf_ready(LocalEngine::Gguf, Target::AppleSilicon, true, Ok(ReadyEngine::Gguf { model: model() }))]
#[case::needs_download(LocalEngine::Auto, Target::AppleSilicon, false, Err(Unavailable::NeedsDownload { size: AGENT_MODEL_SIZE }))]
#[case::intel(LocalEngine::Gguf, Target::IntelMac, true, Err(Unavailable::IntelMac))]
#[case::intel_auto(LocalEngine::Auto, Target::IntelMac, false, Err(Unavailable::IntelMac))]
#[case::other_os(LocalEngine::Auto, Target::Other, false, Err(Unavailable::UnsupportedOs))]
fn resolve_engine_and_target_gives_state(
    #[case] engine: LocalEngine,
    #[case] target: Target,
    #[case] installed: bool,
    #[case] expected: Result<ReadyEngine, Unavailable>,
) {
    assert_that!(resolve(engine, target, installed, model())).is_equal_to(expected);
}

#[cfg(debug_assertions)]
#[test]
fn resolve_fake_is_ready_anywhere() {
    assert_that!(resolve(LocalEngine::Fake, Target::Other, false, model())).is_equal_to(Ok(ReadyEngine::Fake));
}

#[test]
fn agent_model_pin_is_the_audited_file() {
    assert_that!(AGENT_MODEL.revision).is_equal_to("a06e946bb6b655725eafa393f4a9745d460374c9");
    assert_that!(AGENT_MODEL.sha256)
        .is_equal_to(Some("3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597"));
}

#[test]
fn target_current_on_this_machine_matches_cfg() {
    let expected = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Target::AppleSilicon
    } else if cfg!(target_os = "macos") {
        Target::IntelMac
    } else {
        Target::Other
    };
    assert_that!(Target::current()).is_equal_to(expected);
}
