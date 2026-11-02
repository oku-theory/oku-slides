use speculoos::prelude::*;

use super::{McpAccess, Sessions};

fn access() -> McpAccess {
    McpAccess { url: "http://127.0.0.1:1/mcp".to_owned(), authorization: "Bearer t".to_owned() }
}

#[test]
fn sessions_open_numbers_after_a_nonce() {
    let s = Sessions::default();
    let (first, second) = (s.open(access()), s.open(access()));
    assert_that!(first.as_str()).starts_with("local-");
    assert_that!(first.as_str()).ends_with("-1");
    assert_that!(second.as_str()).ends_with("-2");
    assert_that!(first.trim_end_matches("-1")).is_equal_to(second.trim_end_matches("-2"));
}

#[test]
fn sessions_of_two_processes_never_share_an_id() {
    let (a, b) = (Sessions::default(), Sessions::default());
    assert_that!(a.open(access())).is_not_equal_to(b.open(access()));
}

#[test]
fn sessions_load_of_an_id_from_another_table_is_refused() {
    let (earlier, now) = (Sessions::default(), Sessions::default());
    let saved = earlier.open(access_at("http://127.0.0.1:1/mcp"));
    now.open(access_at("http://127.0.0.1:2/mcp"));
    assert_that!(now.load(&saved, access_at("http://127.0.0.1:1/mcp"))).is_false();
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

#[test]
fn turn_summary_of_a_multi_line_request_is_one_line() {
    let summary = super::turn_summary("rename the deck\n\nthen   save it", "Renamed");
    assert_that!(summary.as_str()).is_equal_to("rename the deck then save it → Renamed");
}

#[test]
fn turn_summary_of_a_long_request_is_cut() {
    let summary = super::turn_summary(&"é".repeat(400), "Done");
    assert_that!(summary.chars().count()).is_less_than(super::SUMMARY_CHARS + 10);
    assert_that!(summary.as_str()).ends_with("… → Done");
}

fn access_at(url: &str) -> McpAccess {
    McpAccess { url: url.to_owned(), authorization: "Bearer t".to_owned() }
}

#[test]
fn open_on_the_same_url_n_times_keeps_one_session() {
    let s = Sessions::default();
    let ids: Vec<String> = (0..5).map(|_| s.open(access())).collect();
    assert_that!(s.len()).is_equal_to(1);
    assert_that!(s.get(&ids[4])).is_some();
    assert_that!(s.get(&ids[0])).is_none();
}

#[test]
fn open_on_the_same_url_cancels_the_replaced_turn() {
    let s = Sessions::default();
    let first = s.get(&s.open(access())).unwrap();
    let (_, token) = first.begin_turn();
    s.open(access());
    assert_that!(token.is_cancelled()).is_true();
}

#[test]
fn open_on_another_url_keeps_both_sessions() {
    let s = Sessions::default();
    s.open(access_at("http://127.0.0.1:1/mcp"));
    s.open(access_at("http://127.0.0.1:2/mcp"));
    assert_that!(s.len()).is_equal_to(2);
}

#[test]
fn load_known_session_keeps_its_summaries_and_takes_the_new_token() {
    let s = Sessions::default();
    let id = s.open(access_at("http://127.0.0.1:1/mcp"));
    s.get(&id).unwrap().push_summary("tour 1".to_owned());
    let renewed = McpAccess { url: "http://127.0.0.1:1/mcp".to_owned(), authorization: "Bearer u".to_owned() };
    assert_that!(s.load(&id, renewed)).is_true();
    let loaded = s.get(&id).unwrap();
    assert_that!(loaded.summaries()).is_equal_to(vec!["tour 1".to_owned()]);
    assert_that!(loaded.mcp.authorization.as_str()).is_equal_to("Bearer u");
}

#[test]
fn load_from_another_mcp_url_is_refused_and_keeps_the_session() {
    let s = Sessions::default();
    let id = s.open(access_at("http://127.0.0.1:1/mcp"));
    let other = s.open(access_at("http://127.0.0.1:2/mcp"));
    assert_that!(s.load(&id, access_at("http://127.0.0.1:2/mcp"))).is_false();
    assert_that!(s.get(&id).map(|st| st.mcp.url.clone())).is_equal_to(Some("http://127.0.0.1:1/mcp".to_owned()));
    assert_that!(s.get(&other)).is_some();
}

#[test]
fn load_known_session_keeps_numbering_its_turns() {
    let s = Sessions::default();
    let id = s.open(access());
    s.get(&id).unwrap().begin_turn();
    s.load(&id, access());
    assert_that!(s.get(&id).unwrap().begin_turn().0).is_equal_to(2);
}

#[test]
fn load_keeps_the_sessions_of_other_urls() {
    let s = Sessions::default();
    let first = s.open(access_at("http://127.0.0.1:1/mcp"));
    s.open(access_at("http://127.0.0.1:2/mcp"));
    assert_that!(s.load(&first, access_at("http://127.0.0.1:1/mcp"))).is_true();
    assert_that!(s.len()).is_equal_to(2);
}

#[test]
fn load_unknown_session_is_refused() {
    let s = Sessions::default();
    assert_that!(s.load("local-7", access())).is_false();
    assert_that!(s.len()).is_equal_to(0);
}

#[test]
fn clear_drops_every_session_and_cancels_its_turn() {
    let s = Sessions::default();
    let state = s.get(&s.open(access())).unwrap();
    let (_, token) = state.begin_turn();
    s.clear();
    assert_that!(s.len()).is_equal_to(0);
    assert_that!(token.is_cancelled()).is_true();
}
