//! Structured logs. `info`: what matters to the user (deck opened, saved,
//! model downloaded); `debug`: how things run (rendering, dictation);
//! `trace`: what repeats (loops, polling).

use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::ParseError;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Default level without `RUST_LOG`: `info` for us, `warn` for Loro
/// (`loro`, `loro_internal`), whose `info` logs (import, commit) do not
/// interest the user and drown ours, `off` for async-openai, whose `warn` and
/// `error` copy dictation API bodies that may echo a secret (t-1233).
pub(crate) const DEFAULT_FILTER: &str = "info,loro=warn,loro_internal=warn,async_openai=off";

/// Environment variable that turns on HTTP and TLS debug logs.
///
/// Any value but empty, `0`, `false`, `no` or `off` (any case) enables them.
pub const NETWORK_DEBUG_ENV: &str = "OKU_SLIDES_DEBUG_NETWORK";

/// Outgoing HTTP clients and TLS: ureq (model downloads), reqwest and
/// hyper-util (dictation API, updater) and rustls with the system verifier.
/// At `debug`, ureq redacts sensitive headers and URL paths; reqwest logs none.
pub(crate) const NETWORK_FILTER: &str = "ureq=debug,ureq_proto=debug,reqwest=debug,hyper_util=debug,\
                                         tauri_plugin_updater=debug,rustls=debug,rustls_platform_verifier=debug";

/// Environment variable that logs every ACP message exchanged with the agent.
///
/// Read like [`NETWORK_DEBUG_ENV`]. Both ways, one `debug` event per JSON-RPC line
/// on [`ACP_TARGET`]; the MCP tools' `Authorization` header stays masked.
pub const ACP_DEBUG_ENV: &str = "OKU_SLIDES_DEBUG_ACP";

/// Log target of the raw ACP messages, written by `oku-slides-agent`.
pub const ACP_TARGET: &str = "oku_slides_acp";

/// The directive [`ACP_DEBUG_ENV`] adds: [`ACP_TARGET`] at `debug`.
pub(crate) const ACP_FILTER: &str = "oku_slides_acp=debug";

/// Installs the log subscriber, on standard error.
///
/// In development, `RUST_LOG` picks the level (default: `DEFAULT_FILTER`);
/// a production build always uses `DEFAULT_FILTER`. In both, setting
/// [`NETWORK_DEBUG_ENV`] adds the HTTP and TLS directives, and [`ACP_DEBUG_ENV`]
/// the ACP messages. No effect if a subscriber is already installed (tests, second call).
pub fn init() {
    let rust_log = std::env::var(EnvFilter::DEFAULT_ENV).ok();
    let debug: Vec<&str> = [(NETWORK_DEBUG_ENV, NETWORK_FILTER), (ACP_DEBUG_ENV, ACP_FILTER)]
        .into_iter()
        .filter(|(env, _)| debug_requested(std::env::var(env).ok()))
        .map(|(_, directive)| directive)
        .collect();
    let (filter, invalid) = filter(rust_log, &debug, cfg!(debug_assertions));
    // `try_init`: an already installed subscriber is not an error.
    let _ = tracing_subscriber::registry().with(fmt::layer().with_writer(std::io::stderr)).with(filter).try_init();
    if let Some(error) = invalid {
        tracing::warn!(%error, "invalid RUST_LOG, using the default filter");
    }
}

/// Whether the value of a debug variable ([`NETWORK_DEBUG_ENV`], [`ACP_DEBUG_ENV`]) asks for its logs.
fn debug_requested(value: Option<String>) -> bool {
    value.is_some_and(|v| {
        let v = v.trim();
        !(v.is_empty() || ["0", "false", "no", "off"].iter().any(|off| v.eq_ignore_ascii_case(off)))
    })
}

/// Filter directives: the `debug` ones, then `RUST_LOG` (debug builds) or the default.
///
/// `RUST_LOG` comes last so that it wins for a target both name (`ureq=trace`).
fn directives(rust_log: Option<String>, debug: &[&str], debug_build: bool) -> String {
    let base = rust_log.filter(|_| debug_build).unwrap_or_else(|| DEFAULT_FILTER.to_owned());
    debug.iter().copied().chain(std::iter::once(base.as_str())).collect::<Vec<_>>().join(",")
}

/// The filter to install, and the `RUST_LOG` parse error when it was replaced by the default.
fn filter(rust_log: Option<String>, debug: &[&str], debug_build: bool) -> (EnvFilter, Option<ParseError>) {
    match EnvFilter::try_new(directives(rust_log, debug, debug_build)) {
        Ok(filter) => (filter, None),
        Err(error) => (EnvFilter::new(directives(None, debug, false)), Some(error)),
    }
}

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod telemetry_tests;
