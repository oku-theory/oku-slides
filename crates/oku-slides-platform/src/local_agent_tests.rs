use std::path::PathBuf;

use rstest::rstest;
use speculoos::prelude::*;

use super::{AGENT_MODEL, AGENT_MODEL_SIZE, AppleModel, ReadyEngine, Target, Unavailable, resolve};
use crate::config::LocalEngine;

fn model() -> PathBuf {
    PathBuf::from("/data/models/agent/Qwen3-4B-Instruct-2507-Q4_K_M.gguf")
}

fn gguf_ready() -> ReadyEngine {
    ReadyEngine::Gguf { model: model() }
}

const DOWNLOAD: Result<ReadyEngine, Unavailable> = Err(Unavailable::NeedsDownload { size: AGENT_MODEL_SIZE });

#[rstest]
// auto: Apple's model first, else Gguf.
#[case::auto_apple_ready(LocalEngine::Auto, Target::AppleSilicon, AppleModel::Ready, false, Ok(ReadyEngine::Apple))]
#[case::auto_apple_ready_gguf_installed(
    LocalEngine::Auto,
    Target::AppleSilicon,
    AppleModel::Ready,
    true,
    Ok(ReadyEngine::Apple)
)]
#[case::auto_apple_off_gguf_installed(
    LocalEngine::Auto,
    Target::AppleSilicon,
    AppleModel::IntelligenceOff,
    true,
    Ok(gguf_ready())
)]
#[case::auto_apple_off_needs_download(
    LocalEngine::Auto,
    Target::AppleSilicon,
    AppleModel::IntelligenceOff,
    false,
    DOWNLOAD
)]
#[case::auto_apple_not_ready_needs_download(
    LocalEngine::Auto,
    Target::AppleSilicon,
    AppleModel::ModelNotReady,
    false,
    DOWNLOAD
)]
#[case::auto_old_macos_gguf_installed(
    LocalEngine::Auto,
    Target::AppleSilicon,
    AppleModel::MacOsTooOld,
    true,
    Ok(gguf_ready())
)]
#[case::auto_old_macos_needs_download(
    LocalEngine::Auto,
    Target::AppleSilicon,
    AppleModel::MacOsTooOld,
    false,
    DOWNLOAD
)]
#[case::auto_device_gguf_installed(
    LocalEngine::Auto,
    Target::AppleSilicon,
    AppleModel::DeviceNotEligible,
    true,
    Ok(gguf_ready())
)]
#[case::auto_unknown_gguf_installed(
    LocalEngine::Auto,
    Target::AppleSilicon,
    AppleModel::Unknown,
    true,
    Ok(gguf_ready())
)]
#[case::auto_intel_apple_ready(LocalEngine::Auto, Target::IntelMac, AppleModel::Ready, false, Ok(ReadyEngine::Apple))]
#[case::auto_intel_apple_off(
    LocalEngine::Auto,
    Target::IntelMac,
    AppleModel::IntelligenceOff,
    false,
    Err(Unavailable::AppleIntelligenceOff)
)]
#[case::auto_intel_apple_not_ready(
    LocalEngine::Auto,
    Target::IntelMac,
    AppleModel::ModelNotReady,
    false,
    Err(Unavailable::AppleModelNotReady)
)]
#[case::auto_intel_device(
    LocalEngine::Auto,
    Target::IntelMac,
    AppleModel::DeviceNotEligible,
    false,
    Err(Unavailable::IntelMac)
)]
#[case::auto_intel_old_macos(
    LocalEngine::Auto,
    Target::IntelMac,
    AppleModel::MacOsTooOld,
    true,
    Err(Unavailable::IntelMac)
)]
#[case::auto_intel_unknown(LocalEngine::Auto, Target::IntelMac, AppleModel::Unknown, false, Err(Unavailable::IntelMac))]
// apple: forced, its own reason.
#[case::apple_ready(LocalEngine::Apple, Target::AppleSilicon, AppleModel::Ready, false, Ok(ReadyEngine::Apple))]
#[case::apple_intel_ready(LocalEngine::Apple, Target::IntelMac, AppleModel::Ready, false, Ok(ReadyEngine::Apple))]
#[case::apple_off(
    LocalEngine::Apple,
    Target::AppleSilicon,
    AppleModel::IntelligenceOff,
    true,
    Err(Unavailable::AppleIntelligenceOff)
)]
#[case::apple_not_ready(
    LocalEngine::Apple,
    Target::AppleSilicon,
    AppleModel::ModelNotReady,
    true,
    Err(Unavailable::AppleModelNotReady)
)]
#[case::apple_device(
    LocalEngine::Apple,
    Target::IntelMac,
    AppleModel::DeviceNotEligible,
    false,
    Err(Unavailable::AppleUnsupported)
)]
#[case::apple_old_macos(
    LocalEngine::Apple,
    Target::AppleSilicon,
    AppleModel::MacOsTooOld,
    true,
    Err(Unavailable::AppleUnsupported)
)]
#[case::apple_unknown(
    LocalEngine::Apple,
    Target::AppleSilicon,
    AppleModel::Unknown,
    true,
    Err(Unavailable::AppleUnsupported)
)]
// gguf: forced, Apple's model ignored.
#[case::gguf_ready(LocalEngine::Gguf, Target::AppleSilicon, AppleModel::Ready, true, Ok(gguf_ready()))]
#[case::gguf_needs_download(LocalEngine::Gguf, Target::AppleSilicon, AppleModel::Ready, false, DOWNLOAD)]
#[case::gguf_intel(LocalEngine::Gguf, Target::IntelMac, AppleModel::Ready, true, Err(Unavailable::IntelMac))]
#[case::gguf_other(LocalEngine::Gguf, Target::Other, AppleModel::Ready, true, Err(Unavailable::UnsupportedOs))]
// Outside macOS nothing runs.
#[case::auto_other(LocalEngine::Auto, Target::Other, AppleModel::MacOsTooOld, false, Err(Unavailable::UnsupportedOs))]
#[case::apple_other(LocalEngine::Apple, Target::Other, AppleModel::MacOsTooOld, true, Err(Unavailable::UnsupportedOs))]
fn resolve_engine_target_and_models_give_state(
    #[case] engine: LocalEngine,
    #[case] target: Target,
    #[case] apple: AppleModel,
    #[case] installed: bool,
    #[case] expected: Result<ReadyEngine, Unavailable>,
) {
    assert_that!(resolve(engine, target, || apple, installed, model())).is_equal_to(expected);
}

#[rstest]
#[case::gguf(LocalEngine::Gguf, Target::AppleSilicon)]
#[case::auto_other(LocalEngine::Auto, Target::Other)]
#[case::apple_other(LocalEngine::Apple, Target::Other)]
fn resolve_without_apple_engine_never_probes_apple(#[case] engine: LocalEngine, #[case] target: Target) {
    let probed = std::cell::Cell::new(false);
    let _ = resolve(
        engine,
        target,
        || {
            probed.set(true);
            AppleModel::Ready
        },
        true,
        model(),
    );
    assert_that!(probed.get()).is_false();
}

#[cfg(debug_assertions)]
#[test]
fn resolve_fake_is_ready_anywhere() {
    let apple = || -> AppleModel { panic!("fake never probes Apple") };
    assert_that!(resolve(LocalEngine::Fake, Target::Other, apple, false, model())).is_equal_to(Ok(ReadyEngine::Fake));
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
