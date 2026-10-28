//! t-1211: behind a corporate TLS-inspection proxy, HTTPS failed with `UnknownIssuer` when a
//! client trusted only bundled Mozilla roots. reqwest 0.13 checks certificates with the system
//! verifier (rustls-platform-verifier) under both of its rustls features; these guards keep the
//! dictation API's client on that path (t-1233).

use speculoos::prelude::*;

const WORKSPACE: &str = include_str!("../../../../Cargo.toml");
const CLIENT: &str = include_str!("../../src/api.rs");

#[test]
fn issue_1211_reqwest_uses_its_rustls_feature_with_the_system_verifier() {
    let reqwest = WORKSPACE.lines().find(|l| l.starts_with("reqwest = ")).unwrap_or_default();
    assert_that!((reqwest.contains(r#"features = ["rustls"]"#), reqwest.contains("webpki"))).is_equal_to((true, false));
}

#[test]
fn issue_1211_dictation_client_keeps_the_system_roots() {
    // Any `tls_*` builder call (built-in roots off, certs only…) would bypass the system store.
    assert_that!(CLIENT.contains(".tls_")).is_false();
}
