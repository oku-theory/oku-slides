use agent_client_protocol::schema::v1::{PermissionOption, PermissionOptionKind};
use oku_slides_session::agent::PermissionKind;
use speculoos::prelude::*;

use super::choice;

// `PermissionOptionKind` est `#[non_exhaustive]` : la branche `Other` n'est pas constructible ici.

#[test]
fn choice_each_acp_kind_maps_to_the_same_kind_and_keeps_the_name() {
    let cases = [
        (PermissionOptionKind::AllowOnce, PermissionKind::AllowOnce),
        (PermissionOptionKind::AllowAlways, PermissionKind::AllowAlways),
        (PermissionOptionKind::RejectOnce, PermissionKind::RejectOnce),
        (PermissionOptionKind::RejectAlways, PermissionKind::RejectAlways),
    ];
    for (acp, kind) in cases {
        let c = choice(&PermissionOption::new("id", "Nom", acp));
        assert_that!(c.name.as_str()).is_equal_to("Nom");
        assert_that!(c.kind).is_equal_to(kind);
    }
}
