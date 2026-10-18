//! Point d'entrée : reconnaît `--local-agent` et sert ACP à la place de l'app.

use std::path::PathBuf;
use std::sync::Arc;

use oku_slides_platform::LOCAL_AGENT_FLAG;
use snafu::{OptionExt, ResultExt};

use crate::engine::Engine;
use crate::error::{LocalAgentError, NoEngineSnafu, RuntimeSnafu};

/// Moteur de langage choisi par `--engine`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EngineChoice {
    /// Moteur scripté des tests (builds de débogage seulement).
    #[cfg(debug_assertions)]
    Fake,
}

/// Arguments de l'agent local, après `--local-agent`.
#[derive(Debug)]
pub(crate) struct Args {
    pub(crate) engine: EngineChoice,
    #[cfg_attr(
        not(debug_assertions),
        expect(dead_code, reason = "lu par le moteur factice, absent des builds de production")
    )]
    pub(crate) script: Option<PathBuf>,
    #[cfg_attr(
        not(debug_assertions),
        expect(dead_code, reason = "lu par le moteur factice, absent des builds de production")
    )]
    pub(crate) log: Option<PathBuf>,
}

/// Lit les arguments ; `None` si `--local-agent` est absent.
///
/// Purement syntaxique : le moteur doit être connu et les valeurs présentes.
pub(crate) fn parse_args(args: &[String]) -> Option<Result<Args, LocalAgentError>> {
    if !args.iter().any(|a| a == LOCAL_AGENT_FLAG) {
        return None;
    }
    let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1));
    let engine = match value("--engine").map(String::as_str) {
        #[cfg(debug_assertions)]
        Some("fake") => Some(EngineChoice::Fake),
        _ => None,
    };
    Some(engine.context(NoEngineSnafu).map_err(Into::into).map(|engine| Args {
        engine,
        script: value("--script").map(PathBuf::from),
        log: value("--log").map(PathBuf::from),
    }))
}

/// Si ce processus est l'agent local (`--local-agent`), sert ACP puis termine ; sinon ne fait rien.
///
/// À appeler en tête de `main`, juste après `exit_if_worker` : stdout est le tube ACP,
/// les journaux vont sur stderr.
pub fn exit_if_local_agent() {
    let argv: Vec<String> = std::env::args().collect();
    let Some(parsed) = parse_args(&argv) else { return };
    oku_slides_platform::telemetry::init();
    let code = match parsed.and_then(run) {
        Ok(()) => 0,
        Err(e) => {
            tracing::error!(error = %snafu::Report::from_error(&e), "échec de l'agent local");
            1
        }
    };
    std::process::exit(code);
}

/// Construit le moteur choisi par les arguments.
fn build_engine(args: Args) -> Result<Arc<dyn Engine>, LocalAgentError> {
    match args.engine {
        #[cfg(debug_assertions)]
        EngineChoice::Fake => {
            let script = args.script.as_deref().context(NoEngineSnafu)?;
            Ok(Arc::new(crate::fake::FakeEngine::from_file(script, args.log)?))
        }
    }
}

fn run(args: Args) -> Result<(), LocalAgentError> {
    let engine = build_engine(args)?;
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().context(RuntimeSnafu)?;
    rt.block_on(crate::acp::serve(engine))
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;
