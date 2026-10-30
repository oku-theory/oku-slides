//! The tab gate's refusals, as the agent reads them in French.

use rust_mcp_sdk::schema::CallToolResult;
use speculoos::prelude::*;

use super::{Harness, text_of};

/// The fr text of `agent-error-tab-no-turn`.
pub const NO_TURN_FR: &str = "N'ouvrez ou n'affichez un onglet de pack ou de thème qu'en répondant à l'utilisateur.";
/// The fr text of `agent-error-tab-limit`.
pub const LIMIT_FR: &str = "Vous avez déjà ouvert ou affiché un onglet de pack ou de thème dans ce tour ; terminez-y, ou demandez à l'utilisateur avant d'en changer.";
/// The fr text of `agent-error-continue-in-other-tab`.
pub const CONTINUE_FR: &str = "Cette demande se poursuit dans l'onglet que vous avez ouvert ou affiché : demandez à l'utilisateur d'y continuer ; cet onglet ne modifie plus de widget pour l'instant.";

/// `r` is a refusal that says `key` (and `fr` in French).
pub fn assert_refused(h: &Harness, r: &CallToolResult, key: &str, fr: &str) {
    assert_that!(r.is_error).is_equal_to(Some(true));
    assert_that!(text_of(r)).is_equal_to(h.localizer.tr(key));
    assert_that!(text_of(r)).is_equal_to(fr.to_owned());
}
