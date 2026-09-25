use speculoos::prelude::*;

use super::one_line;

#[derive(Debug)]
struct Outer(std::io::Error);
impl std::fmt::Display for Outer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("could not save")
    }
}
impl std::error::Error for Outer {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[test]
fn one_line_with_cause_joins_the_chain() {
    let e = Outer(std::io::Error::other("disk full"));
    assert_that!(one_line(&e)).is_equal_to("could not save: disk full".to_owned());
}

#[test]
fn one_line_without_cause_returns_the_message() {
    assert_that!(one_line(&std::io::Error::other("alone"))).is_equal_to("alone".to_owned());
}
