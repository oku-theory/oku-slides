use speculoos::prelude::*;

use super::agent_fingerprint;

#[test]
fn agent_fingerprint_hides_the_arguments() {
    let id = agent_fingerprint("npx", &["--api-key".into(), "secret".into()]);
    assert_that!(id.contains("secret")).is_false();
    assert_that!(id.len()).is_equal_to(64);
}

#[test]
fn agent_fingerprint_separates_command_and_arguments() {
    assert_that!(agent_fingerprint("ab", &["c".into()])).is_not_equal_to(agent_fingerprint("a", &["bc".into()]));
}
