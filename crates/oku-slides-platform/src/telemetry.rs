//! Structured logs. `info`: what matters to the user (deck opened, saved,
//! model downloaded); `debug`: how things run (rendering, dictation);
//! `trace`: what repeats (loops, polling).

use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::ParseError;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Default level without `RUST_LOG`: `info` for us, `warn` for Loro
/// (`loro`, `loro_internal`), whose `info` logs (import, commit) do not
/// interest the user and drown ours.
pub(crate) const DEFAULT_FILTER: &str = "info,loro=warn,loro_internal=warn";

/// Environment variable that turns on HTTP and TLS debug logs.
///
/// Any value but empty, `0`, `false`, `no` or `off` (any case) enables them.
pub const NETWORK_DEBUG_ENV: &str = "OKU_SLIDES_DEBUG_NETWORK";

/// Outgoing HTTP clients and TLS: ureq (dictation API, model downloads),
/// reqwest and hyper-util (updater) and rustls with the system verifier. At
/// `debug`, ureq redacts sensitive headers and URL paths.
pub(crate) const NETWORK_FILTER: &str = "ureq=debug,ureq_proto=debug,reqwest=debug,hyper_util=debug,\
                                         tauri_plugin_updater=debug,rustls=debug,rustls_platform_verifier=debug";

/// Installs the log subscriber, on standard error.
///
/// In development, `RUST_LOG` picks the level (default: `DEFAULT_FILTER`);
/// a production build always uses `DEFAULT_FILTER`. In both, setting
/// [`NETWORK_DEBUG_ENV`] adds the HTTP and TLS directives. No effect if a
/// subscriber is already installed (tests, second call).
pub fn init() {
    let rust_log = std::env::var(EnvFilter::DEFAULT_ENV).ok();
    let network = network_debug_requested(std::env::var(NETWORK_DEBUG_ENV).ok());
    let (filter, invalid) = filter(rust_log, network, cfg!(debug_assertions));
    // `try_init`: an already installed subscriber is not an error.
    let _ = tracing_subscriber::registry().with(fmt::layer().with_writer(std::io::stderr)).with(filter).try_init();
    if let Some(error) = invalid {
        tracing::warn!(%error, "invalid RUST_LOG, using the default filter");
    }
}

/// Whether the value of [`NETWORK_DEBUG_ENV`] asks for network logs.
fn network_debug_requested(value: Option<String>) -> bool {
    value.is_some_and(|v| {
        let v = v.trim();
        !(v.is_empty() || ["0", "false", "no", "off"].iter().any(|off| v.eq_ignore_ascii_case(off)))
    })
}

/// Filter directives: the network ones, then `RUST_LOG` (debug builds) or the default.
///
/// `RUST_LOG` comes last so that it wins for a target both name (`ureq=trace`).
fn directives(rust_log: Option<String>, network: bool, debug_build: bool) -> String {
    let base = rust_log.filter(|_| debug_build).unwrap_or_else(|| DEFAULT_FILTER.to_owned());
    if network { format!("{NETWORK_FILTER},{base}") } else { base }
}

/// The filter to install, and the `RUST_LOG` parse error when it was replaced by the default.
fn filter(rust_log: Option<String>, network: bool, debug_build: bool) -> (EnvFilter, Option<ParseError>) {
    match EnvFilter::try_new(directives(rust_log, network, debug_build)) {
        Ok(filter) => (filter, None),
        Err(error) => (EnvFilter::new(directives(None, network, false)), Some(error)),
    }
}

#[cfg(test)]
#[path = "telemetry_tests.rs"]
mod telemetry_tests;
