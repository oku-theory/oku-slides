use oku_slides_i18n::{Localize, Localizer, available, english};
use snafu::IntoError;
use speculoos::prelude::*;

use super::{TaskSnafu, ToolError};

/// Les clés des erreurs des outils et des cartes (`ToolError`, `HubError`, `say_session`, `say_to_agent`).
const KEYS: &[&str] = &[
    "agent-error-arguments",
    "agent-error-unknown-tool",
    "agent-error-denied",
    "agent-error-no-turn",
    "agent-error-turn-over",
    "agent-error-no-file",
    "agent-error-path-relative",
    "agent-error-path-parent",
    "agent-error-file",
    "agent-error-asset-unknown",
    "agent-error-setting-locked",
    "agent-error-setting",
    "agent-error-rewrite-text",
    "agent-error-paragraphs",
    "agent-error-unexpected-action",
    "agent-error-unknown-surface",
    "agent-error-expired",
    "agent-error-incomplete",
    "agent-error-no-field",
    "agent-error-no-variant",
    "agent-error-no-pack",
    "agent-error-history-not-opened",
    "agent-error-history-index",
    "agent-error-present",
    "agent-error-locale",
    "agent-error-empty-interface",
    "agent-error-not-shown",
];

#[test]
fn every_error_key_is_translated_in_every_shipped_language() {
    assert_that!(available().len()).is_equal_to(20);
    for locale in available() {
        let l = Localizer::new(Some(locale));
        for key in KEYS {
            asserting(&format!("{locale} : {key}")).that(&l.has(key)).is_true();
            if locale != "en" {
                // Une vraie traduction, pas l'anglais recopié.
                asserting(&format!("{locale} : {key} recopié de l'anglais"))
                    .that(&l.tr(key))
                    .is_not_equal_to(english().tr(key));
            }
        }
    }
}

#[tokio::test]
async fn a_failed_task_is_said_in_the_user_language() {
    // La tâche ne rend jamais rien : seule l'erreur est possible.
    let Err(e) = tokio::task::spawn_blocking(|| panic!("échec simulé")).await;
    let e: ToolError = TaskSnafu.into_error(e);
    assert_that!(e.localize(&Localizer::new(Some("fr"))))
        .is_equal_to("L'outil a échoué de façon inattendue.".to_owned());
    assert_that!(e.localize(english())).is_equal_to("The tool failed unexpectedly.".to_owned());
}

#[test]
fn argument_detail_drops_the_position() {
    let Err(e) = serde_json::from_str::<super::super::tools::SlideId>("{}") else { panic!("accepté sans id") };
    assert_that!(e.to_string()).contains("line");
    assert_that!(super::argument_detail(&e)).is_equal_to("missing field `id`".to_owned());
}
