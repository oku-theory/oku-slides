//! t-1191 (lot 3 review, bug 2): an open that lands Here after claiming gives the claim back.

use oku_slides_session::PAPIER_THEME;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::other_tabs::{OtherTabs, start_other_tabs};
use crate::common::{call, server};

#[tokio::test(flavor = "multi_thread")]
async fn issue_1191_open_pack_landing_here_leaves_the_turn_free_to_open_a_theme() {
    let (h, tabs) = start_other_tabs().await;
    let c = server(&h).await;
    let _turn = h.hub.begin_turn();
    let here = call(&c, "open_pack", json!({ "pack": OtherTabs::HERE_PACK.to_string() })).await;
    assert_that!(here.is_error).is_not_equal_to(Some(true));
    let r = call(&c, "open_theme", json!({ "theme": "papier" })).await;
    assert_that!((r.is_error, tabs.opened_themes())).is_equal_to((None, vec![PAPIER_THEME]));
}
