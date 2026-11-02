use std::sync::{Arc, Mutex, PoisonError};

use rstest::rstest;
use speculoos::prelude::*;
use tracing::Level;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use super::{
    ACP_FILTER, ACP_TARGET, DEFAULT_FILTER, NETWORK_FILTER, debug_requested, directives, filter, init, recent_problems,
};

#[test]
fn init_called_twice_is_harmless() {
    init();
    init();
    tracing::info!(appel = 2, "journaux installés");
}

#[test]
fn init_then_warn_and_info_rings_only_the_warning() {
    init();
    tracing::info!("ring-marker-info");
    tracing::warn!("ring-marker-warn");
    let messages: Vec<String> = recent_problems().into_iter().map(|l| l.message).collect();
    assert_that!(messages.iter().any(|m| m == "ring-marker-warn")).is_true();
    assert_that!(messages.iter().any(|m| m == "ring-marker-info")).is_false();
}

#[test]
fn init_then_async_openai_error_is_not_ringed() {
    init();
    tracing::error!(target: "async_openai", "ring-marker-secret");
    assert_that!(recent_problems().iter().any(|l| l.message == "ring-marker-secret")).is_false();
}

#[test]
fn default_filter_silences_loro_info_logs() {
    assert_that!(DEFAULT_FILTER).contains("loro=warn");
    assert_that!(DEFAULT_FILTER).contains("loro_internal=warn");
    // The filter must stay parseable by `EnvFilter` (a syntax error would
    // silently break the default filtering).
    assert_that!(DEFAULT_FILTER.parse::<EnvFilter>()).is_ok();
}

#[rstest]
#[case(NETWORK_FILTER)]
#[case(ACP_FILTER)]
fn debug_filter_parsed_by_envfilter_succeeds(#[case] debug: &str) {
    assert_that!(debug.parse::<EnvFilter>()).is_ok();
}

#[test]
fn acp_filter_names_the_acp_target() {
    assert_that!(ACP_FILTER).is_equal_to(format!("{ACP_TARGET}=debug").as_str());
}

#[rstest]
#[case(None, false)]
#[case(Some(""), false)]
#[case(Some("0"), false)]
#[case(Some("false"), false)]
#[case(Some("FALSE"), false)]
#[case(Some("no"), false)]
#[case(Some(" Off "), false)]
#[case(Some("1"), true)]
#[case(Some("true"), true)]
fn debug_requested_reads_the_variable(#[case] value: Option<&str>, #[case] expected: bool) {
    assert_that!(debug_requested(value.map(Into::into))).is_equal_to(expected);
}

#[test]
fn directives_without_debug_keep_the_default() {
    assert_that!(directives(None, &[], false)).is_equal_to(DEFAULT_FILTER.to_owned());
}

#[test]
fn directives_with_network_debug_add_http_clients_in_release() {
    assert_that!(directives(None, &[NETWORK_FILTER], false)).is_equal_to(format!("{NETWORK_FILTER},{DEFAULT_FILTER}"));
}

#[test]
fn directives_in_release_ignore_rust_log() {
    assert_that!(directives(Some("trace".into()), &[], false)).is_equal_to(DEFAULT_FILTER.to_owned());
}

#[test]
fn directives_in_debug_build_put_rust_log_after_network() {
    assert_that!(directives(Some("ureq=trace".into()), &[NETWORK_FILTER], true))
        .is_equal_to(format!("{NETWORK_FILTER},ureq=trace,async_openai=off"));
}

#[rstest]
#[case("debug")]
#[case("debug,async_openai=trace")]
fn filter_in_debug_build_with_rust_log_still_drops_async_openai(#[case] rust_log: &str) {
    let seen = Seen::default();
    let (installed, invalid) = filter(Some(rust_log.into()), &[], true);
    assert_that!(invalid.is_none()).is_true();
    let subscriber = tracing_subscriber::registry().with(seen.clone().with_filter(installed));
    tracing::subscriber::with_default(subscriber, || {
        tracing::error!(target: "async_openai::error", "body: {{\"key\":\"sk-x\"}}");
        tracing::debug!(target: "oku_slides_session", "kept");
    });
    let recorded = seen.0.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_that!(recorded).is_equal_to(vec![("oku_slides_session".to_owned(), Level::DEBUG)]);
}

#[test]
fn init_then_registered_secret_never_reaches_recent_problems() {
    init();
    super::register_secret("oku-e2e-secret-1144");
    tracing::error!(key = "oku-e2e-secret-1144", "ring-marker-key");
    let line = recent_problems().into_iter().find(|l| l.message.starts_with("ring-marker-key"));
    assert_that!(line.map(|l| l.message.contains("oku-e2e-secret-1144"))).is_equal_to(Some(false));
}

#[test]
fn filter_with_invalid_rust_log_falls_back_and_reports() {
    let (installed, invalid) = filter(Some("ureq=nope".into()), &[NETWORK_FILTER], true);
    assert_that!(invalid.is_some()).is_true();
    assert_that!(installed.to_string())
        .is_equal_to(EnvFilter::new(directives(None, &[NETWORK_FILTER], false)).to_string());
}

/// Records the target and level of each event that passes the filter.
#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<(String, Level)>>>);

impl<S: tracing::Subscriber> Layer<S> for Seen {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        let meta = event.metadata();
        self.0.lock().unwrap_or_else(PoisonError::into_inner).push((meta.target().to_owned(), *meta.level()));
    }
}

#[test]
fn filter_with_network_debug_records_ureq_debug_and_keeps_loro_quiet() {
    let seen = Seen::default();
    let (installed, _) = filter(None, &[NETWORK_FILTER], false);
    let subscriber = tracing_subscriber::registry().with(seen.clone().with_filter(installed));
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug!(target: "ureq::run", "GET");
        tracing::info!(target: "loro_internal::oplog", "import");
        tracing::debug!(target: "oku_slides_session", "rendu");
    });
    let recorded = seen.0.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_that!(recorded).is_equal_to(vec![("ureq::run".to_owned(), Level::DEBUG)]);
}

#[rstest]
#[case(&[])]
#[case(&[NETWORK_FILTER])]
fn filter_drops_async_openai_errors_that_quote_bodies(#[case] debug: &[&str]) {
    let seen = Seen::default();
    let (installed, _) = filter(None, debug, false);
    let subscriber = tracing_subscriber::registry().with(seen.clone().with_filter(installed));
    tracing::subscriber::with_default(subscriber, || {
        tracing::error!(target: "async_openai::error", "failed deserialization of: {{\"key\":\"sk-x\"}}");
    });
    assert_that!(seen.0.lock().unwrap_or_else(PoisonError::into_inner).clone()).is_empty();
}

#[test]
fn filter_with_acp_debug_records_acp_messages_only() {
    let seen = Seen::default();
    let (installed, _) = filter(None, &[ACP_FILTER], false);
    let subscriber = tracing_subscriber::registry().with(seen.clone().with_filter(installed));
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug!(target: ACP_TARGET, "{{}}");
        tracing::debug!(target: "ureq::run", "GET");
    });
    let recorded = seen.0.lock().unwrap_or_else(PoisonError::into_inner).clone();
    assert_that!(recorded).is_equal_to(vec![(ACP_TARGET.to_owned(), Level::DEBUG)]);
}

#[test]
fn directives_with_acp_and_network_debug_add_both() {
    assert_that!(directives(None, &[NETWORK_FILTER, ACP_FILTER], false))
        .is_equal_to(format!("{NETWORK_FILTER},{ACP_FILTER},{DEFAULT_FILTER}"));
}
