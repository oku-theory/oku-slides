//! t-1211: behind a corporate TLS-inspection proxy (Linux), the dictation API failed with
//! "invalid peer certificate: UnknownIssuer": ureq trusted only its bundled Mozilla roots,
//! not the system store holding the company's CA. Outgoing TLS now uses the system verifier.

use oku_slides_platform::http::system_tls;
use speculoos::prelude::*;
use ureq::tls::RootCerts;

#[test]
fn system_tls_default_trusts_the_platform_verifier() {
    let tls = system_tls();
    assert_that!(matches!(tls.root_certs(), RootCerts::PlatformVerifier)).is_true();
}
