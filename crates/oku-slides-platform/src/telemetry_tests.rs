use super::init;

#[test]
fn init_called_twice_is_harmless() {
    init();
    init();
    tracing::info!(appel = 2, "journaux installés");
}
