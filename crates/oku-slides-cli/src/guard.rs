//! Provenance des requêtes de `oku-slides serve` : seule l'interface locale parle au serveur.
//!
//! Une page tierce ouverte dans le navigateur peut viser `127.0.0.1` :
//! - DNS rebinding : un nom qui résout vers `127.0.0.1` se trahit dans
//!   l'en-tête `Host`, qui doit nommer la boucle locale ou l'adresse d'écoute
//!   (`[serve] host`) ; une adresse « partout » (`0.0.0.0`, `::`) n'ajoute
//!   aucun nom : le réseau local n'est pas servi ;
//! - requête d'une autre page (`<img>`, `<script>`, `fetch` sans CORS, y
//!   compris depuis un autre port de `localhost`) : `Sec-Fetch-Site` doit
//!   valoir `same-origin` ou `none` ; sinon elle prendrait par exemple une
//!   place du flux d'évènements ;
//! - `Origin`, quand il est là, doit être l'origine même de la requête
//!   (schéma http(s), même nom et même port que `Host`) : c'est le cas de
//!   l'interface servie directement comme à travers le proxy de Vite, qui
//!   garde l'en-tête `Host` du navigateur ;
//! - un POST doit être `application/json` : d'une autre origine, le
//!   navigateur ne l'envoie qu'après une pré-vérification CORS à laquelle le
//!   serveur ne donne jamais son accord (aucun en-tête `Access-Control-*`).
//!
//! Ce n'est pas une authentification : un client hors navigateur qui joint
//! le port peut tout demander. `[serve] host` reste donc la boucle locale.

use axum::http::{HeaderMap, Method};

const LOOPBACK: [&str; 3] = ["localhost", "127.0.0.1", "::1"];

/// La règle qu'une requête refusée n'a pas respectée (pour le journal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// `Host` absent, en double, mal formé, ou qui ne nomme pas cette machine.
    Host,
    /// Le navigateur dit que la requête vient d'une autre page.
    CrossSite,
    /// `Origin` en double ou différent de l'origine de la requête.
    Origin,
    /// Un POST qui n'est pas du JSON.
    ContentType,
}

/// `Ok` si la requête vient de l'interface locale ; sinon la règle enfreinte.
pub(crate) fn check(method: &Method, headers: &HeaderMap, bound: &str) -> Result<(), Refusal> {
    let host = single(headers, "Host").ok().flatten().and_then(authority).ok_or(Refusal::Host)?;
    let bound = unbracket(bound);
    let wildcard = bound == "0.0.0.0" || bound == "::";
    let local = LOOPBACK.iter().any(|l| l.eq_ignore_ascii_case(host.name))
        || (!wildcard && bound.eq_ignore_ascii_case(host.name));
    if !local {
        return Err(Refusal::Host);
    }
    // Absent : un client hors navigateur, ou un navigateur trop ancien pour l'envoyer.
    let site = single(headers, "Sec-Fetch-Site").map_err(|()| Refusal::CrossSite)?;
    if site.is_some_and(|s| !s.eq_ignore_ascii_case("same-origin") && !s.eq_ignore_ascii_case("none")) {
        return Err(Refusal::CrossSite);
    }
    let origin = single(headers, "Origin").map_err(|()| Refusal::Origin)?;
    if origin.is_some_and(|o| origin_authority(o).is_none_or(|o| !o.same_as(&host))) {
        return Err(Refusal::Origin);
    }
    if method == Method::POST {
        let json = single(headers, "Content-Type")
            .ok()
            .flatten()
            .and_then(|t| t.split(';').next())
            .is_some_and(|t| t.trim().eq_ignore_ascii_case("application/json"));
        if !json {
            return Err(Refusal::ContentType);
        }
    }
    Ok(())
}

/// La valeur d'un en-tête ; `Err` s'il apparaît plus d'une fois ou n'est pas
/// de l'ASCII visible.
fn single<'a>(headers: &'a HeaderMap, name: &'static str) -> Result<Option<&'a str>, ()> {
    let mut values = headers.get_all(name).iter();
    let first = values.next().map(|v| v.to_str().map(str::trim).map_err(|_| ())).transpose()?;
    if values.next().is_some() { Err(()) } else { Ok(first) }
}

/// Nom et port d'une autorité (`nom`, `nom:port`, `[::1]:port`).
#[derive(Debug, Clone, Copy)]
struct Authority<'a> {
    name: &'a str,
    port: Option<&'a str>,
}

impl Authority<'_> {
    fn same_as(&self, other: &Authority<'_>) -> bool {
        self.name.eq_ignore_ascii_case(other.name) && self.port == other.port
    }
}

/// Lit une autorité ; `None` si elle est mal formée (IPv6 sans crochets, reste après `]`…).
fn authority(value: &str) -> Option<Authority<'_>> {
    let (name, port) = match value.strip_prefix('[') {
        Some(rest) => {
            let (name, after) = rest.split_once(']')?;
            let port = if after.is_empty() { None } else { Some(after.strip_prefix(':')?) };
            (name, port)
        }
        None => match value.split_once(':') {
            Some((name, port)) => (name, Some(port)),
            None => (value, None),
        },
    };
    let port_ok = port.is_none_or(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    (!name.is_empty() && port_ok).then_some(Authority { name, port })
}

/// L'autorité d'un `Origin` http(s) ; `None` pour `null`, `file://` ou un autre schéma.
fn origin_authority(origin: &str) -> Option<Authority<'_>> {
    let rest = origin.strip_prefix("http://").or_else(|| origin.strip_prefix("https://"))?;
    authority(rest)
}

/// Une adresse sans crochets (`[::1]` → `::1`).
fn unbracket(address: &str) -> &str {
    address.strip_prefix('[').and_then(|a| a.strip_suffix(']')).unwrap_or(address)
}

#[cfg(test)]
#[path = "guard_tests.rs"]
mod guard_tests;
