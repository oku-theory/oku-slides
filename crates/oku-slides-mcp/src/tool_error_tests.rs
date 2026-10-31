use oku_slides_i18n::{Localize, Localizer, available, english};
use snafu::IntoError;
use speculoos::prelude::*;

use super::{
    ContinueInOtherTabSnafu, EmptyDictationChangeSnafu, HubSnafu, TabLimitSnafu, TabNoTurnSnafu, TaskSnafu, ToolError,
    UsageNoDeckSnafu, VoiceSnafu,
};

/// Les clés des erreurs des outils et des cartes (`ToolError`, `HubError`, `say_session`, `say_to_agent`).
const KEYS: &[&str] = &[
    "agent-error-arguments",
    "agent-error-unknown-tool",
    "agent-error-denied",
    "agent-error-no-turn",
    "agent-error-tab-no-turn",
    "agent-error-tab-limit",
    "agent-error-continue-in-other-tab",
    "agent-error-pack-open-in-tab",
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
    "agent-error-empty-dictation",
    "agent-error-usage-no-deck",
    "agent-confirm-dictation-api",
    "agent-confirm-dictation-download",
    "agent-confirm-dictation-language",
    "agent-confirm-agent",
    "agent-confirm-agent-builtin",
    "agent-confirm-agent-model",
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

#[rstest::rstest]
#[case::empty(EmptyDictationChangeSnafu.build(), "Rien à changer : donnez au moins un réglage de la dictée.")]
#[case::voice(
    VoiceSnafu.into_error(Box::new(crate::VoiceError::new(crate::VoiceErrorKind::NothingToCancel))),
    "Aucun téléchargement n'est en cours"
)]
fn a_voice_tool_error_is_said_in_french(#[case] e: ToolError, #[case] said: &str) {
    assert_that!(e.localize(&Localizer::new(Some("fr")))).is_equal_to(said.to_owned());
}

/// The refusal a deck session gets when another session holds the pack's tab: `PackOpenInTab`.
fn refused_by_a_pack_tab() -> oku_slides_session::SessionError {
    use oku_slides_core::Namespace;
    use oku_slides_session::{BASE_PACK, Fonts, Platform, Session, Settings};
    let data = tempfile::tempdir().unwrap_or_else(|e| panic!("dossier de test : {e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.path().to_owned());
    settings.paths.decks = Some(data.path().join("decks"));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    let mut deck = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
    let mut tab = Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {e}"));
    deck.new_deck().unwrap_or_else(|e| panic!("deck : {e}"));
    tab.open_pack_tab_here(BASE_PACK, None).unwrap_or_else(|e| panic!("onglet de pack : {e}"));
    let base = Namespace::parse("base").unwrap_or_else(|| panic!("espace de noms"));
    let Err(refused) = deck.set_widget(Some(&base), "carte", "#let x = 1".into()) else {
        panic!("l'écriture d'un pack tenu par un onglet est refusée")
    };
    assert_that!(refused.kind()).is_equal_to(oku_slides_session::SessionErrorKind::PackOpenInTab);
    refused
}

#[rstest::rstest]
#[case::en("en")]
#[case::fr("fr")]
fn hub_session_error_reaches_the_agent_without_a_title(#[case] locale: &str) {
    let l = Localizer::new(Some(locale));
    let refused = refused_by_a_pack_tab();
    let with_title = refused.localize(&l);
    let e: ToolError = HubSnafu.into_error(Box::new(crate::HubError::from_session(refused)));
    assert_that!(e.localize(&l)).is_equal_to(l.tr("agent-error-pack-open-in-tab"));
    assert_that!(e.localize(&l)).is_not_equal_to(with_title);
}

#[test]
fn hub_error_of_another_family_keeps_its_text() {
    let l = Localizer::new(Some("fr"));
    let e: ToolError = HubSnafu.into_error(Box::new(crate::HubError::unknown_surface("ask-1")));
    assert_that!(e.localize(&l)).is_equal_to(l.tr("agent-error-unknown-surface"));
}

#[rstest::rstest]
#[case(TabNoTurnSnafu.build(), "agent-error-tab-no-turn")]
#[case(TabLimitSnafu.build(), "agent-error-tab-limit")]
#[case(ContinueInOtherTabSnafu.build(), "agent-error-continue-in-other-tab")]
fn tab_gate_errors_say_their_key(#[case] e: ToolError, #[case] key: &str) {
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr(key));
    assert_that!(fr.tr(key)).is_not_equal_to(Localizer::new(Some("en")).tr(key));
}

#[test]
fn usage_no_deck_localized_in_french_is_its_key() {
    let fr = Localizer::new(Some("fr"));
    assert_that!(UsageNoDeckSnafu.build().localize(&fr)).is_equal_to(fr.tr("agent-error-usage-no-deck"));
    assert_that!(fr.tr("agent-error-usage-no-deck"))
        .is_not_equal_to(Localizer::new(Some("en")).tr("agent-error-usage-no-deck"));
}
