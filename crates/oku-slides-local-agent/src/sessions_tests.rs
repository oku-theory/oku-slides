use speculoos::prelude::*;

use super::{McpAccess, Sessions};

fn access() -> McpAccess {
    McpAccess { url: "http://127.0.0.1:1/mcp".to_owned(), authorization: "Bearer t".to_owned() }
}

#[test]
fn sessions_open_numbers_from_one() {
    let s = Sessions::default();
    assert_that!(s.open(access())).is_equal_to("local-1".to_owned());
    assert_that!(s.open(access())).is_equal_to("local-2".to_owned());
}

#[test]
fn sessions_get_unknown_is_none() {
    assert_that!(Sessions::default().get("local-9")).is_none();
}

#[test]
fn sessions_get_returns_the_opened_session() {
    let s = Sessions::default();
    let id = s.open(access());
    assert_that!(s.get(&id).map(|st| st.mcp.url.clone())).is_equal_to(Some("http://127.0.0.1:1/mcp".to_owned()));
}

#[test]
fn begin_turn_cancels_the_previous_token_and_installs_a_fresh_one() {
    let s = Sessions::default();
    let state = s.get(&s.open(access())).unwrap();
    let (first, old) = state.begin_turn();
    let (second, new) = state.begin_turn();
    assert_that!(old.is_cancelled()).is_true();
    assert_that!(new.is_cancelled()).is_false();
    assert_that!((first, second)).is_equal_to((1, 2));
}

#[test]
fn push_summary_keeps_the_last_three() {
    let s = Sessions::default();
    let state = s.get(&s.open(access())).unwrap();
    for i in 1..=4 {
        state.push_summary(format!("tour {i}"));
    }
    assert_that!(state.summaries()).is_equal_to(vec!["tour 2".to_owned(), "tour 3".to_owned(), "tour 4".to_owned()]);
}

#[test]
fn mcp_access_debug_hides_the_token() {
    let shown =
        format!("{:?}", McpAccess { url: "http://x/mcp".to_owned(), authorization: "Bearer secret".to_owned() });
    assert_that!(shown.contains("secret")).is_false();
    assert_that!(shown.contains("http://x/mcp")).is_true();
}
