//! Options de permission ACP, telles que la fenêtre de confirmation les montre.

use agent_client_protocol::schema::v1::{PermissionOption, PermissionOptionKind};
use oku_slides_session::agent::{PermissionChoice, PermissionKind};

/// L'option ACP, avec sa nature : c'est elle, pas l'ordre donné par l'agent,
/// qui décide du bouton principal (jamais « toujours autoriser »).
pub(crate) fn choice(o: &PermissionOption) -> PermissionChoice {
    let kind = match o.kind {
        PermissionOptionKind::AllowOnce => PermissionKind::AllowOnce,
        PermissionOptionKind::AllowAlways => PermissionKind::AllowAlways,
        PermissionOptionKind::RejectOnce => PermissionKind::RejectOnce,
        PermissionOptionKind::RejectAlways => PermissionKind::RejectAlways,
        _ => PermissionKind::Other,
    };
    PermissionChoice { name: o.name.clone(), kind }
}

#[cfg(test)]
#[path = "permission_tests.rs"]
mod permission_tests;
