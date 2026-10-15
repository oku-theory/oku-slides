//! Provenance des requêtes de `oku-slides serve` : seule l'interface locale parle au serveur.
//!
//! Règle commune à tout serveur HTTP local du projet : voir
//! `docs/adr/0003-regle-d-origine-des-serveurs-locaux.md`.
//!
//! Une page tierce ouverte dans le navigateur peut viser `127.0.0.1` :
//! - DNS rebinding : un nom qui résout vers `127.0.0.1` se trahit dans
//!   l'en-tête `Host`, qui doit nommer la boucle locale ou l'adresse d'écoute
//!   (`[serve] host`) ; une adresse « partout » (`0.0.0.0`, `::`) n'ajoute
//!   aucun nom accepté (ce qui n'arrête pas un client qui forge `Host`) ;
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
//! Ces règles ne sont pas une authentification : un client hors navigateur
//! qui joint le port peut forger ses en-têtes. Hors de la boucle locale,
//! `serve` tire donc un jeton à chaque lancement ([`Token`]) que `check`
//! exige aussi, après les règles d'origine : en-tête `Authorization: Bearer`,
//! ou cookie posé par un premier `GET` qui porte `?token=` (t-193).

use axum::http::{HeaderMap, Method, Uri};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest as _, Sha256};

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
    /// Serveur exposé : jeton absent ou faux (401).
    Token,
}

/// Le jeton d'un lancement exposé au réseau : 32 octets aléatoires.
///
/// Seul `serve` le connaît ; il l'affiche dans l'URL à ouvrir.
pub(crate) struct Token {
    /// Le jeton en base64 d'URL, sans remplissage.
    secret: String,
    /// Son condensat, comparé en temps constant à celui du jeton reçu.
    digest: [u8; 32],
    /// Le nom du cookie, propre au port : deux serveurs d'une même machine
    /// ne s'écrasent pas leurs cookies (le port n'isole pas les cookies).
    cookie: String,
}

impl std::fmt::Debug for Token {
    /// Le jeton n'apparaît jamais.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token").field("cookie", &self.cookie).finish_non_exhaustive()
    }
}

impl Token {
    /// Tire un jeton neuf du générateur du système, pour le serveur du port `port`.
    pub(crate) fn generate(port: u16) -> Result<Self, getrandom::Error> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes)?;
        Ok(Self::from_secret(URL_SAFE_NO_PAD.encode(bytes), port))
    }

    fn from_secret(secret: String, port: u16) -> Self {
        let digest = digest(&secret);
        Self { secret, digest, cookie: format!("oku-slides-token-{port}") }
    }

    /// Le jeton, à placer dans l'URL affichée au lancement.
    pub(crate) fn secret(&self) -> &str {
        &self.secret
    }

    /// Vrai si `candidate` est le jeton ; temps constant sur les condensats.
    fn matches(&self, candidate: &str) -> bool {
        digest(candidate).iter().zip(self.digest).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
    }

    /// Le jeton est dans `Authorization: Bearer` ou dans un cookie.
    fn presented(&self, headers: &HeaderMap) -> bool {
        let bearer = single(headers, "Authorization")
            .ok()
            .flatten()
            .and_then(|v| v.split_once(' '))
            .is_some_and(|(scheme, t)| scheme.eq_ignore_ascii_case("Bearer") && self.matches(t.trim()));
        bearer
            || headers.get_all("Cookie").iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).any(|c| {
                c.trim().split_once('=').is_some_and(|(name, value)| name == self.cookie && self.matches(value))
            })
    }

    /// L'en-tête `Set-Cookie` qui retient le jeton dans le navigateur.
    fn set_cookie(&self) -> String {
        format!("{}={}; HttpOnly; SameSite=Strict; Path=/", self.cookie, self.secret)
    }
}

fn digest(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

/// Une requête admise par [`check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Access {
    /// À router normalement.
    Granted,
    /// Un `GET ?token=` valide : poser le cookie et rediriger vers l'URL sans le jeton.
    Exchange {
        /// Valeur de l'en-tête `Set-Cookie`.
        set_cookie: String,
        /// Valeur de l'en-tête `Location` : même chemin, sans le paramètre `token`.
        location: String,
    },
}

/// Admet la requête si elle vient de l'interface locale et, quand le serveur
/// est exposé (`token`), si elle présente le jeton ; sinon la règle enfreinte.
pub(crate) fn check(
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    bound: &str,
    token: Option<&Token>,
) -> Result<Access, Refusal> {
    origin(method, headers, bound)?;
    let Some(token) = token else { return Ok(Access::Granted) };
    let (query_token, rest) = split_token(uri.query());
    match query_token {
        Some(t) if method == Method::GET && token.matches(t) => {
            // `//hôte` ou `/\hôte` serait une redirection vers un autre site.
            let path = uri.path().trim_start_matches(['/', '\\']);
            let location = if rest.is_empty() { format!("/{path}") } else { format!("/{path}?{rest}") };
            Ok(Access::Exchange { set_cookie: token.set_cookie(), location })
        }
        Some(_) => Err(Refusal::Token),
        None if token.presented(headers) => Ok(Access::Granted),
        None => Err(Refusal::Token),
    }
}

/// Le paramètre `token` d'une requête d'URL, et les autres paramètres.
fn split_token(query: Option<&str>) -> (Option<&str>, String) {
    let mut token = None;
    let mut rest = Vec::new();
    for pair in query.unwrap_or("").split('&').filter(|p| !p.is_empty()) {
        match pair.strip_prefix("token=") {
            Some(t) => token = Some(t),
            None => rest.push(pair),
        }
    }
    (token, rest.join("&"))
}

/// Les règles d'origine de l'ADR-0003.
fn origin(method: &Method, headers: &HeaderMap, bound: &str) -> Result<(), Refusal> {
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
