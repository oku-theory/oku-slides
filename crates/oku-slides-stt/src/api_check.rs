//! Règles des réglages de l'API de dictée, sur chaque écriture et à l'usage.
//!
//! L'adresse et la variable décident quel secret part vers quel hôte : la forme normalisée
//! est la seule stockée, montrée et appelée (spec t-1144).

/// Longueur maximale d'une adresse d'API, normalisée.
const MAX_URL: usize = 512;
/// Longueur maximale d'un hôte (DNS).
const MAX_HOST: usize = 253;
/// Longueur maximale d'une étiquette d'hôte (DNS).
const MAX_LABEL: usize = 63;
/// Hôtes autorisés en `http://` : cette machine, après normalisation.
const LOOPBACK: [&str; 3] = ["localhost", "127.0.0.1", "[::1]"];

/// La forme normalisée d'une adresse d'API acceptable ; `None` si elle est refusée.
#[must_use]
pub fn normalized_url(url: &str) -> Option<String> {
    let plain =
        !url.is_empty() && url.bytes().all(|b| b.is_ascii_graphic()) && url.contains("://") && !url.contains('\\');
    if !plain {
        return None;
    }
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let scheme_ok = match parsed.scheme() {
        "https" => true,
        "http" => LOOPBACK.contains(&host),
        _ => false,
    };
    let host_ok = host.len() <= MAX_HOST
        && (host.starts_with('[')
            || host.split('.').all(|label| {
                (1..=MAX_LABEL).contains(&label.len())
                    && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            }));
    let clean = parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none();
    let normal = parsed.as_str().trim_end_matches('/').to_owned();
    (scheme_ok && host_ok && clean && normal.len() <= MAX_URL).then_some(normal)
}

/// L'hôte d'une adresse valide, montré seul dans les confirmations.
pub fn url_host(url: &str) -> Option<String> {
    let normal = normalized_url(url)?;
    url::Url::parse(&normal).ok()?.host_str().map(str::to_owned)
}

/// L'adresse valide vise cette machine (pas de proxy pour elle).
#[must_use]
pub fn is_loopback(url: &str) -> bool {
    url_host(url).is_some_and(|h| LOOPBACK.contains(&h.as_str()))
}

/// Le nom est celui d'une variable d'environnement : majuscules, chiffres et `_`.
#[must_use]
pub fn valid_key_env(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else { return false };
    name.len() <= 64
        && (first.is_ascii_uppercase() || first == b'_')
        && bytes.all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// Le nom est celui d'un modèle d'API : lettres, chiffres et `._:/-`.
#[must_use]
pub fn valid_api_model(name: &str) -> bool {
    (1..=128).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'/' | b'-'))
}

#[cfg(test)]
#[path = "api_check_tests.rs"]
mod api_check_tests;
