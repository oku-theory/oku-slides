use speculoos::prelude::*;

use super::stays_inside;

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
