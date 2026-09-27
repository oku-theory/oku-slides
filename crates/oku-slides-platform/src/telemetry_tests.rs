use tracing_subscriber::EnvFilter;

use super::{DEFAULT_FILTER, init};

#[test]
fn init_called_twice_is_harmless() {
    init();
    init();
    tracing::info!(appel = 2, "journaux installés");
}

#[test]
fn default_filter_silences_loro_info_logs() {
    assert!(DEFAULT_FILTER.contains("loro=warn"));
    assert!(DEFAULT_FILTER.contains("loro_internal=warn"));
    // Le filtre doit rester analysable par `EnvFilter` (une faute de syntaxe
    // romprait silencieusement le filtrage par défaut).
    let _: EnvFilter = DEFAULT_FILTER.parse().unwrap_or_else(|e| panic!("filtre par défaut invalide : {e}"));
}
