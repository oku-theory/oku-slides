//! Point d'entrée : reconnaît `--local-agent` et sert ACP à la place de l'app.

use std::sync::Arc;

use oku_slides_platform::local_agent::{ReadyEngine, local_engine};
use oku_slides_platform::{LOCAL_AGENT_FLAG, Platform};
#[cfg(debug_assertions)]
use snafu::OptionExt;
use snafu::ResultExt;

use crate::engine::Engine;
#[cfg(debug_assertions)]
use crate::error::NoScriptSnafu;
use crate::error::{LocalAgentError, PlatformSnafu, RuntimeSnafu, UnavailableSnafu};

/// Si ce processus est l'agent local (`--local-agent`), sert ACP puis termine ; sinon ne fait rien.
///
/// À appeler en tête de `main`, juste après `exit_if_worker` : stdout est le tube ACP,
/// les journaux vont sur stderr.
pub fn exit_if_local_agent() {
    if !std::env::args().any(|a| a == LOCAL_AGENT_FLAG) {
        return;
    }
    oku_slides_platform::telemetry::init();
    let code = match run() {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!(error = %snafu::Report::from_error(&e), "échec de l'agent local");
            1
        }
    };
    std::process::exit(code);
}

/// Construit le moteur que dictent les réglages ; un moteur forcé indisponible échoue avec sa raison.
pub(crate) fn build_engine(platform: &Platform) -> Result<Arc<dyn Engine>, LocalAgentError> {
    match local_engine(&platform.settings, &platform.storage) {
        #[cfg(debug_assertions)]
        Ok(ReadyEngine::Fake) => {
            let settings = &platform.settings.local_agent;
            let script = settings.fake_script.as_deref().context(NoScriptSnafu)?;
            Ok(Arc::new(crate::fake::FakeEngine::from_file(script, settings.fake_log.clone())?))
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        Ok(ReadyEngine::Gguf { model }) => {
            Ok(Arc::new(crate::gguf::GgufEngine::load(&model, &platform.settings.local_agent)?))
        }
        // `local_engine` ne rend Gguf que sur Apple silicon : ailleurs ce bras est inatteignable
        // mais il échoue avec sa raison plutôt que de paniquer.
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        Ok(ReadyEngine::Gguf { .. }) => {
            Err(UnavailableSnafu { reason: "gguf engine not built for this target" }.build().into())
        }
        Err(why) => Err(UnavailableSnafu { reason: format!("{why:?}") }.build().into()),
    }
}

fn run() -> Result<(), LocalAgentError> {
    let platform = Platform::load().context(PlatformSnafu)?;
    // Avant le runtime : le chargement du modèle bloque, et un seul moteur existe par processus.
    let engine = build_engine(&platform)?;
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().context(RuntimeSnafu)?;
    let served = rt.block_on(crate::acp::serve(engine.clone()));
    // Metal abandonne le processus à la sortie si un modèle est encore chargé :
    // le runtime (et ses clones du moteur) tombe d'abord, puis le moteur.
    drop(rt);
    drop(engine);
    served
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;
