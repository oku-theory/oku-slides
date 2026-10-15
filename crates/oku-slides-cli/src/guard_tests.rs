use axum::http::request::Builder;
use axum::http::{Method, Request};
use speculoos::prelude::*;

use super::{Access, Refusal, Token, check as check_parts};

/// Une requête de test, sans corps.
type Req = Request<()>;

fn check(req: &Req, bound: &str) -> Result<(), Refusal> {
    check_parts(req.method(), req.uri(), req.headers(), bound, None).map(|access| {
        assert_that!(access).is_equal_to(Access::Granted);
    })
}

fn allowed(req: &Req, bound: &str) -> bool {
    check(req, bound).is_ok()
}

/// Construit la requête (méthode GET et chemin `/` par défaut).
fn build(builder: Builder) -> Req {
    builder.body(()).unwrap_or_else(|e| panic!("requête : {e}"))
}

/// Un POST JSON de l'interface servie par Vite (Origin sur le port 5173).
fn post(host: &str, origin: Option<&str>, content_type: &str) -> Req {
    let mut req = Request::builder()
        .method(Method::POST)
        .uri("/api/agent")
        .header("Host", host)
        .header("Content-Type", content_type);
    if let Some(origin) = origin {
        req = req.header("Origin", origin);
    }
    build(req)
}

fn get(host: Option<&str>) -> Req {
    let mut req = Request::builder().uri("/api/events");
    if let Some(host) = host {
        req = req.header("Host", host);
    }
    build(req)
}

#[test]
fn allowed_post_from_the_ui_through_the_vite_proxy_is_accepted() {
    // Le proxy de Vite garde l'en-tête Host du navigateur (`changeOrigin` absent).
    let req = post("localhost:5173", Some("http://localhost:5173"), "application/json");
    assert_that!(allowed(&req, "127.0.0.1")).is_true();
}

#[test]
fn allowed_post_from_the_ui_served_directly_is_accepted() {
    let req = post("127.0.0.1:4870", Some("http://127.0.0.1:4870"), "Application/JSON");
    assert_that!(allowed(&req, "127.0.0.1")).is_true();
}

#[test]
fn check_post_from_another_local_port_is_refused() {
    // Un autre serveur local (Jupyter, un autre projet) n'est pas l'interface.
    let req = post("127.0.0.1:4870", Some("http://localhost:3000"), "application/json");
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::Origin);
}

#[test]
fn allowed_post_without_origin_is_accepted() {
    let req = post("localhost:4870", None, "application/json; charset=utf-8");
    assert_that!(allowed(&req, "127.0.0.1")).is_true();
}

#[test]
fn allowed_post_from_a_foreign_origin_is_refused() {
    let req = post("127.0.0.1:4870", Some("https://evil.example"), "application/json");
    assert_that!(allowed(&req, "127.0.0.1")).is_false();
}

#[test]
fn allowed_post_from_a_look_alike_origin_is_refused() {
    let req = post("127.0.0.1:4870", Some("http://localhost.evil.example"), "application/json");
    assert_that!(allowed(&req, "127.0.0.1")).is_false();
}

#[test]
fn allowed_post_with_an_opaque_origin_is_refused() {
    let req = post("127.0.0.1:4870", Some("null"), "application/json");
    assert_that!(allowed(&req, "127.0.0.1")).is_false();
}

#[test]
fn allowed_post_as_plain_text_is_refused() {
    // Un POST text/plain part sans pré-vérification CORS : c'est la voie d'une page tierce.
    let req = post("127.0.0.1:4870", None, "text/plain");
    assert_that!(allowed(&req, "127.0.0.1")).is_false();
}

#[test]
fn allowed_request_for_a_rebound_host_name_is_refused() {
    // DNS rebinding : evil.example résout vers 127.0.0.1, l'en-tête Host le trahit.
    assert_that!(allowed(&get(Some("evil.example:4870")), "127.0.0.1")).is_false();
}

#[test]
fn allowed_request_without_host_is_refused() {
    assert_that!(allowed(&get(None), "127.0.0.1")).is_false();
}

#[test]
fn allowed_request_for_loopback_names_is_accepted() {
    for host in ["localhost", "localhost:4870", "127.0.0.1:4870", "[::1]:4870"] {
        assert_that!(allowed(&get(Some(host)), "127.0.0.1")).named(host).is_true();
    }
}

#[test]
fn allowed_request_for_the_configured_host_is_accepted() {
    assert_that!(allowed(&get(Some("192.168.1.20:4870")), "192.168.1.20")).is_true();
}

#[test]
fn check_post_without_content_type_is_refused() {
    let req = build(Request::builder().method(Method::POST).header("Host", "127.0.0.1:4870"));
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::ContentType);
}

#[test]
fn check_post_from_an_ipv6_loopback_origin_is_accepted() {
    let req = post("[::1]:4870", Some("http://[::1]:4870"), "application/json");
    assert_that!(check(&req, "127.0.0.1")).is_ok();
}

#[test]
fn check_post_from_a_file_origin_is_refused() {
    let req = post("127.0.0.1:4870", Some("file://"), "application/json");
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::Origin);
}

#[test]
fn check_post_for_a_rebound_host_is_refused() {
    let req = post("evil.example:4870", None, "application/json");
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::Host);
}

#[test]
fn check_look_alike_host_is_refused() {
    assert_that!(check(&get(Some("localhost.evil.example:4870")), "127.0.0.1")).is_err_containing(Refusal::Host);
}

#[test]
fn check_get_from_a_foreign_origin_is_refused() {
    // Un EventSource d'une autre page prendrait une place du flux d'évènements.
    let req = build(
        Request::builder().uri("/api/events").header("Host", "127.0.0.1:4870").header("Origin", "https://evil.example"),
    );
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::Origin);
}

#[test]
fn check_cross_site_fetch_is_refused() {
    // Une requête no-cors (<img>, <script>) ne porte pas d'Origin, mais le navigateur dit d'où elle part.
    let req = build(
        Request::builder().uri("/api/events").header("Host", "127.0.0.1:4870").header("Sec-Fetch-Site", "cross-site"),
    );
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::CrossSite);
}

#[test]
fn check_same_origin_fetch_is_accepted() {
    for site in ["same-origin", "none"] {
        let req = build(Request::builder().header("Host", "localhost:5173").header("Sec-Fetch-Site", site));
        assert_that!(check(&req, "127.0.0.1")).named(site).is_ok();
    }
}

#[test]
fn check_same_site_fetch_from_another_local_port_is_refused() {
    // localhost:3000 → localhost:4870 est « same-site » : le port ne compte pas pour le navigateur.
    let req = build(
        Request::builder().uri("/api/events").header("Host", "localhost:4870").header("Sec-Fetch-Site", "same-site"),
    );
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::CrossSite);
}

#[test]
fn check_upper_case_host_is_accepted() {
    assert_that!(check(&get(Some("LOCALHOST:4870")), "127.0.0.1")).is_ok();
}

#[test]
fn check_host_with_junk_after_the_ipv6_bracket_is_refused() {
    assert_that!(check(&get(Some("[::1]evil:4870")), "127.0.0.1")).is_err_containing(Refusal::Host);
}

#[test]
fn check_duplicate_host_is_refused() {
    let req = build(Request::builder().header("Host", "127.0.0.1:4870").header("Host", "evil.example"));
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::Host);
}

#[test]
fn check_ipv6_bound_address_is_accepted_in_both_spellings() {
    for bound in ["::1", "[::1]"] {
        assert_that!(check(&get(Some("[::1]:4870")), bound)).named(bound).is_ok();
    }
}

#[test]
fn check_wildcard_bound_address_does_not_open_the_lan() {
    // `host = "0.0.0.0"` écoute partout, mais seuls les noms locaux sont servis (voir config).
    assert_that!(check(&get(Some("192.168.1.20:4870")), "0.0.0.0")).is_err_containing(Refusal::Host);
}

#[test]
fn check_duplicate_origin_is_refused() {
    let req = build(
        Request::builder()
            .header("Host", "127.0.0.1:4870")
            .header("Origin", "http://127.0.0.1:4870")
            .header("Origin", "http://127.0.0.1:4870"),
    );
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::Origin);
}

#[test]
fn check_duplicate_sec_fetch_site_is_refused() {
    let req = build(
        Request::builder()
            .header("Host", "127.0.0.1:4870")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Site", "same-origin"),
    );
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::CrossSite);
}

#[test]
fn check_origin_with_non_visible_ascii_is_refused() {
    let origin =
        axum::http::HeaderValue::from_bytes(b"http://127.0.0.1:4870\xff").unwrap_or_else(|e| panic!("valeur : {e}"));
    let req = build(Request::builder().header("Host", "127.0.0.1:4870").header("Origin", origin));
    assert_that!(check(&req, "127.0.0.1")).is_err_containing(Refusal::Origin);
}

// ---- Jeton d'un serveur exposé (t-193) ---------------------------------------

const SECRET: &str = "c2VjcmV0LWRlLXRlc3QtcG91ci1sZS1zZXJ2ZXVyLWV4cG9zZQ";

fn token() -> Token {
    Token::from_secret(SECRET.to_owned(), 4870)
}

/// Une requête d'un client du réseau qui forge `Host` vers un serveur sur `0.0.0.0`.
fn exposed(builder: Builder) -> Result<Access, Refusal> {
    let req = build(builder.header("Host", "127.0.0.1:4870"));
    check_parts(req.method(), req.uri(), req.headers(), "0.0.0.0", Some(&token()))
}

#[test]
fn check_loopback_server_without_token_needs_none() {
    let req = build(Request::builder().uri("/?x=1").header("Host", "127.0.0.1:4870"));
    let access = check_parts(req.method(), req.uri(), req.headers(), "127.0.0.1", None);
    assert_that!(access).is_ok_containing(Access::Granted);
}

#[test]
fn check_exposed_request_without_token_is_unauthorized() {
    assert_that!(exposed(Request::builder().uri("/api/events"))).is_err_containing(Refusal::Token);
}

#[rstest::rstest]
#[case::wrong_bearer("Authorization", "Bearer pas-le-jeton")]
#[case::other_scheme("Authorization", "Basic c2VjcmV0")]
#[case::wrong_cookie("Cookie", "oku-slides-token-4870=pas-le-jeton")]
#[case::cookie_of_another_port("Cookie", "oku-slides-token-4871=c2VjcmV0LWRlLXRlc3QtcG91ci1sZS1zZXJ2ZXVyLWV4cG9zZQ")]
#[case::truncated_bearer("Authorization", "Bearer c2VjcmV0LWRlLXRlc3Q")]
fn check_exposed_request_with_a_wrong_token_is_unauthorized(#[case] name: &str, #[case] value: &str) {
    assert_that!(exposed(Request::builder().uri("/api/events").header(name, value))).is_err_containing(Refusal::Token);
}

#[rstest::rstest]
#[case::bearer("Authorization", format!("Bearer {SECRET}"))]
#[case::bearer_lower_case("Authorization", format!("bearer {SECRET}"))]
#[case::cookie("Cookie", format!("oku-slides-token-4870={SECRET}"))]
#[case::cookie_among_others("Cookie", format!("theme=dark; oku-slides-token-4870={SECRET}; x=1"))]
fn check_exposed_request_with_the_token_is_granted(#[case] name: &str, #[case] value: String) {
    let builder = Request::builder().method(Method::POST).uri("/api").header("Content-Type", "application/json");
    assert_that!(exposed(builder.header(name, value))).is_ok_containing(Access::Granted);
}

#[test]
fn check_exposed_page_with_the_token_in_the_url_sets_the_cookie_and_redirects() {
    let access = exposed(Request::builder().uri(format!("/?token={SECRET}")));
    assert_that!(access).is_ok_containing(Access::Exchange {
        set_cookie: format!("oku-slides-token-4870={SECRET}; HttpOnly; SameSite=Strict; Path=/"),
        location: "/".to_owned(),
    });
}

#[test]
fn check_exposed_page_keeps_the_other_query_parameters_in_the_redirect() {
    let access = exposed(Request::builder().uri(format!("/index.html?a=1&token={SECRET}&b=2")));
    let location = match access {
        Ok(Access::Exchange { location, .. }) => location,
        other => panic!("échange attendu : {other:?}"),
    };
    assert_that!(location).is_equal_to("/index.html?a=1&b=2".to_owned());
}

#[test]
fn check_exposed_page_never_redirects_to_another_host() {
    for uri in [format!("//evil.example/?token={SECRET}"), format!("/%5Cevil.example?token={SECRET}")] {
        let location = match exposed(Request::builder().uri(uri.as_str())) {
            Ok(Access::Exchange { location, .. }) => location,
            other => panic!("échange attendu : {other:?}"),
        };
        assert_that!(location.starts_with("//") || location.starts_with("/\\")).named(&uri).is_false();
    }
}

#[test]
fn check_exposed_page_with_a_wrong_token_in_the_url_is_unauthorized() {
    assert_that!(exposed(Request::builder().uri("/?token=faux"))).is_err_containing(Refusal::Token);
}

#[test]
fn check_exposed_post_with_the_token_in_the_url_is_unauthorized() {
    // Le jeton dans l'URL ne sert qu'à poser le cookie, au chargement de la page.
    let builder = Request::builder()
        .method(Method::POST)
        .uri(format!("/api?token={SECRET}"))
        .header("Content-Type", "application/json");
    assert_that!(exposed(builder)).is_err_containing(Refusal::Token);
}

#[test]
fn check_exposed_foreign_origin_is_refused_before_the_token() {
    let builder =
        Request::builder().header("Authorization", format!("Bearer {SECRET}")).header("Origin", "https://evil.example");
    assert_that!(exposed(builder)).is_err_containing(Refusal::Origin);
}

#[test]
fn token_generate_draws_a_fresh_url_safe_token() {
    let (a, b) = (Token::generate(4870), Token::generate(4870));
    let (a, b) = (a.unwrap_or_else(|e| panic!("jeton : {e}")), b.unwrap_or_else(|e| panic!("jeton : {e}")));
    assert_that!(a.secret()).is_not_equal_to(b.secret());
    assert_that!(a.secret().len()).is_equal_to(43);
    assert_that!(a.secret().bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')).is_true();
}

#[test]
fn token_debug_hides_the_secret() {
    assert_that!(format!("{:?}", token())).does_not_contain(SECRET);
}
