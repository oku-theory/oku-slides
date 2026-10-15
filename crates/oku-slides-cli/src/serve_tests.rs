use speculoos::prelude::*;

use std::net::IpAddr;

use super::{reachable_from_network, stays_inside, token_url};
use crate::guard::Token;

#[test]
fn stays_inside_accepts_plain_relative_paths() {
    for rel in ["index.html", "assets/app.js", "assets/fonts/a.woff2", "dir/"] {
        assert_that!(stays_inside(rel)).named(rel).is_true();
    }
}

#[test]
fn stays_inside_refuses_paths_that_climb_or_jump() {
    for rel in ["..", "../secret.txt", "a/../../secret.txt", "./index.html", "/etc/passwd"] {
        assert_that!(stays_inside(rel)).named(rel).is_false();
    }
}

#[cfg(windows)]
#[test]
fn stays_inside_refuses_windows_climbs_and_prefixes() {
    for rel in [r"..\secret.txt", r"a\..\..\secret.txt", "C:/Windows/win.ini", r"C:\Windows", r"\\server\share\x"] {
        assert_that!(stays_inside(rel)).named(rel).is_false();
    }
}

fn ip(text: &str) -> IpAddr {
    text.parse().unwrap_or_else(|e| panic!("{text} : {e}"))
}

#[test]
fn loopback_addresses_are_not_reachable_from_the_network() {
    for addr in ["127.0.0.1", "127.0.0.2", "::1", "::ffff:127.0.0.1"] {
        assert_that!(reachable_from_network(ip(addr))).named(addr).is_false();
    }
}

#[test]
fn wildcard_and_network_addresses_are_reachable_from_the_network() {
    for addr in ["0.0.0.0", "::", "192.168.1.10", "10.0.0.2", "fe80::1", "::ffff:192.168.1.10"] {
        assert_that!(reachable_from_network(ip(addr))).named(addr).is_true();
    }
}

#[rstest::rstest]
#[case::network_address("192.168.1.20", "http://192.168.1.20:4870/?token=")]
#[case::wildcard_v4("0.0.0.0", "http://localhost:4870/?token=")]
#[case::wildcard_v6("::", "http://localhost:4870/?token=")]
#[case::ipv6_address("fe80::1", "http://[fe80::1]:4870/?token=")]
fn token_url_names_a_host_the_guard_accepts(#[case] host: &str, #[case] prefix: &str) {
    let token = Token::generate(4870).unwrap_or_else(|e| panic!("jeton : {e}"));
    assert_that!(token_url(host, 4870, &token)).is_equal_to(format!("{prefix}{}", token.secret()));
}
