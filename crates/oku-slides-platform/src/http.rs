//! Settings shared by the app's outgoing HTTP clients (feature `download`).

/// TLS that trusts the system certificate store, not a bundled root list.
///
/// A company proxy that inspects TLS signs with its own CA, installed in the system
/// store (Keychain, Windows store, `/etc/ssl` and `SSL_CERT_FILE` on Linux): ureq's
/// default Mozilla roots would reject it with `UnknownIssuer` (t-1211).
#[must_use]
pub fn system_tls() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build()
}
