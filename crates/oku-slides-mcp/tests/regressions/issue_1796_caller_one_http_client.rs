//! t-1796: the rust-mcp-sdk client built an HTTP client per transport and per request; with the
//! system roots unified in (polars `cloud`), each build read the macOS keychain, ~360 ms a call
//! instead of a few. `McpCaller` shares one client: twenty sessions, each one call, stay far below
//! the seconds the SDK client took (at least 14 s).

use std::time::{Duration, Instant};

use oku_slides_mcp::McpCaller;
use serde_json::Map;
use speculoos::prelude::*;

use crate::common::start;

#[tokio::test(flavor = "multi_thread")]
async fn twenty_caller_sessions_take_well_under_a_keychain_read_each() {
    let h = start().await;
    let began = Instant::now();
    for _ in 0..20 {
        let c = McpCaller::connect(h.server.url(), Some(h.server.authorization())).await.unwrap();
        let r = c.call_tool("deck_outline", Map::new()).await.unwrap();
        assert_that!(r.is_error).is_not_equal_to(Some(true));
        c.close().await.unwrap();
    }
    assert_that!(began.elapsed()).is_less_than(Duration::from_secs(5));
}
