use oku_slides_session::WorkspaceRequest;
use speculoos::prelude::*;

use super::*;

fn req(v: serde_json::Value) -> WorkspaceRequest {
    serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"))
}

#[test]
fn classify_view_and_messages_never_touch_windows() {
    assert_that!(classify(&req(serde_json::json!({"cmd": "view"})))).is_equal_to((false, false));
    assert_that!(classify(&req(serde_json::json!({"cmd": "messages"})))).is_equal_to((false, false));
}

#[test]
fn classify_same_deck_returns_its_own_reply() {
    let same = req(serde_json::json!({"cmd": "same_deck", "path": "/d/x.oku-slides.pdf"}));
    assert_that!(classify(&same)).is_equal_to((false, false));
}

#[test]
fn classify_carry_assets_returns_its_own_reply() {
    let element = serde_json::json!({"id": "e", "kind": "base/image", "params": {"asset": "sha256:ab"}});
    let carry = req(serde_json::json!({"cmd": "carry_assets", "from": "t-1", "to": "t-2", "element": element}));
    assert_that!((classify(&carry), broadcast(&carry))).is_equal_to(((false, false), None));
}

#[test]
fn classify_structural_requests_resync_without_forcing_focus() {
    for v in [
        serde_json::json!({"cmd": "new_tab", "window": "w-1"}),
        serde_json::json!({"cmd": "activate", "tab": "t-1"}),
        serde_json::json!({"cmd": "close", "tab": "t-1"}),
        serde_json::json!({"cmd": "reorder", "tab": "t-1", "index": 0}),
        serde_json::json!({"cmd": "close_window", "window": "w-1"}),
        // Le choix d'import oublié : chaque fenêtre reçoit la vue sans lui.
        serde_json::json!({"cmd": "import_picked", "id": 1}),
    ] {
        assert_that!(classify(&req(v))).is_equal_to((true, false));
    }
}

#[test]
fn classify_requests_that_move_a_tab_also_force_focus() {
    for v in [
        serde_json::json!({"cmd": "open_tab", "window": "w-1", "path": "/a"}),
        serde_json::json!({"cmd": "new_window"}),
        serde_json::json!({"cmd": "move_tab", "tab": "t-1"}),
    ] {
        assert_that!(classify(&req(v))).is_equal_to((true, true));
    }
}

#[test]
fn is_newer_only_when_strictly_greater_than_the_last_applied() {
    assert_that!(is_newer(2, 1)).is_true();
    assert_that!(is_newer(1, 1)).is_false();
    assert_that!(is_newer(0, 1)).is_false();
    // Un focus arrivé après, mais dont la tâche termine avant, reste refusé
    // une fois qu'un focus plus récent a déjà été appliqué.
    assert_that!(is_newer(5, 10)).is_false();
}

#[test]
fn broadcast_set_scheme_reaches_every_window() {
    assert_that!(broadcast(&req(serde_json::json!({"cmd": "set_scheme", "scheme": "dark"}))))
        .is_equal_to(Some("scheme"));
}

#[test]
fn broadcast_set_locale_reaches_every_window_as_messages() {
    assert_that!(broadcast(&req(serde_json::json!({"cmd": "set_locale", "locale": "de"}))))
        .is_equal_to(Some("messages"));
}

#[test]
fn broadcast_only_requests_that_leave_windows_unsynced() {
    // `workspace` ne diffuse que dans la branche sans resynchronisation des fenêtres.
    for json in [
        serde_json::json!({"cmd": "set_locale", "locale": "de"}),
        serde_json::json!({"cmd": "set_scheme", "scheme": "dark"}),
    ] {
        let r = req(json);
        assert_that!(classify(&r).0).is_false();
    }
}

#[test]
fn broadcast_messages_read_stays_with_its_window() {
    assert_that!(broadcast(&req(serde_json::json!({"cmd": "messages"})))).is_none();
}

#[test]
fn broadcast_scheme_read_stays_with_its_window() {
    assert_that!(broadcast(&req(serde_json::json!({"cmd": "scheme"})))).is_none();
}
