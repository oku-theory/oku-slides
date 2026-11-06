use std::sync::Arc;
use std::time::{Duration, Instant};

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::forms::{self, Anchor, AskChoice};
use oku_slides_session::agent::{FocusSource, PermissionChoice, PermissionKind, ShowVariants, Surface, Variant};
use rstest::rstest;
use speculoos::prelude::*;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use super::{ARMING, AnswerOutcome, armed};
use crate::hub::Shown;
use crate::hub::tests::hub;
use crate::{ConfirmAction, ConfirmAsk, ConfirmVerb, HeardUtterance, Lexicon, McpEvent, SurfaceHub};

fn fr() -> Lexicon {
    Lexicon::for_locales("fr", None)
}

/// `text`, said once the question is armed.
fn said(text: &str) -> HeardUtterance {
    HeardUtterance { seq: 1, text: text.into(), started: Instant::now() + ARMING + Duration::from_millis(50) }
}

/// The next surface the hub sends.
async fn next_surface(rx: &mut broadcast::Receiver<McpEvent>) -> Surface {
    loop {
        if let Ok(McpEvent::Surface { surface, .. }) = rx.recv().await {
            return surface;
        }
    }
}

/// A guarded confirmation, shown; its task and surface.
async fn confirming(hub: &Arc<SurfaceHub>, ask: ConfirmAsk) -> (JoinHandle<bool>, Surface) {
    let mut rx = hub.subscribe();
    let task = tokio::spawn({
        let hub = hub.clone();
        async move { hub.confirm("Fusionner la copie ?".into(), ask).await }
    });
    (task, next_surface(&mut rx).await)
}

/// A permission with allow-always first, shown; its task.
async fn asking_permission(hub: &Arc<SurfaceHub>, always: &str) -> JoinHandle<Option<usize>> {
    let mut rx = hub.subscribe();
    let choices: Vec<PermissionChoice> = [
        (always, PermissionKind::AllowAlways),
        ("Autoriser une fois", PermissionKind::AllowOnce),
        ("Refuser", PermissionKind::RejectOnce),
    ]
    .into_iter()
    .map(|(name, kind)| PermissionChoice { name: name.into(), kind })
    .collect();
    let task = tokio::spawn({
        let hub = hub.clone();
        async move { hub.permission("Écrire le fichier ?", &choices).await }
    });
    next_surface(&mut rx).await;
    task
}

/// What `heard` does to the open question, and the surface it showed again, if any.
fn answer(
    hub: &Arc<SurfaceHub>,
    heard: &HeardUtterance,
    focus: Option<FocusSource>,
) -> (AnswerOutcome, Option<String>) {
    let mut rx = hub.subscribe();
    let outcome = hub.answer_voice(heard, &fr(), focus, None);
    let mut shown = None;
    while let Ok(event) = rx.try_recv() {
        if let McpEvent::Surface { surface, .. } = event {
            shown = Some(serde_json::to_string(&surface).unwrap_or_default());
        }
    }
    (outcome, shown)
}

#[rstest]
#[case::before_shown(-1, false)]
#[case::just_short(499, false)]
#[case::exactly(500, true)]
#[case::after(501, true)]
fn armed_boundaries(#[case] offset_ms: i64, #[case] want: bool) {
    let shown = Instant::now();
    let delta = Duration::from_millis(offset_ms.unsigned_abs());
    let started = if offset_ms < 0 { shown.checked_sub(delta) } else { shown.checked_add(delta) };
    assert_that!(started.is_some_and(|started| armed(shown, started))).is_equal_to(want);
}

#[tokio::test]
async fn confirm_danger_without_verb_keeps_open_with_heard() {
    let hub = hub();
    let (task, surface) = confirming(&hub, ConfirmAsk::Danger(ConfirmVerb::Merge)).await;
    let (outcome, shown) = answer(&hub, &said("oui"), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(shown.unwrap_or_default()).contains("Entendu : « oui »");
    assert_that!(hub.surfaces().iter().map(|s| s.id.clone()).collect::<Vec<_>>()).is_equal_to(vec![surface.id]);
    assert_that!(task.is_finished()).is_false();
}

#[tokio::test]
async fn confirm_danger_with_verb_from_host_confirms() {
    let hub = hub();
    let (task, _) = confirming(&hub, ConfirmAsk::Danger(ConfirmVerb::Merge)).await;
    let (outcome, _) = answer(&hub, &said("oui, fusionner"), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::Answered);
    assert_that!(task.await.unwrap()).is_true();
}

#[tokio::test]
async fn confirm_unfocused_window_keeps_open() {
    let hub = hub();
    let (task, _) = confirming(&hub, ConfirmAsk::Danger(ConfirmVerb::Merge)).await;
    let (outcome, shown) = answer(&hub, &said("oui, fusionner"), None);
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(shown.unwrap_or_default()).contains("Cliquez sur le bouton pour confirmer.");
    assert_that!(task.is_finished()).is_false();
}

/// The dev server only has the focus its pages report: never enough to confirm (Q6, R16).
#[tokio::test]
async fn plain_confirm_page_reported_focus_keeps_open() {
    let hub = hub();
    let (task, _) = confirming(&hub, ConfirmAsk::Plain(ConfirmAction::Save)).await;
    let (outcome, shown) = answer(&hub, &said("oui"), Some(FocusSource::PageReported));
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(shown.unwrap_or_default()).contains("Cliquez sur le bouton pour confirmer.");
    assert_that!(task.is_finished()).is_false();
}

#[tokio::test]
async fn confirm_page_reported_focus_keeps_open() {
    let hub = hub();
    let (task, _) = confirming(&hub, ConfirmAsk::Danger(ConfirmVerb::Merge)).await;
    let (outcome, shown) = answer(&hub, &said("oui, fusionner"), Some(FocusSource::PageReported));
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(shown.unwrap_or_default()).contains("Cliquez sur le bouton pour confirmer.");
    assert_that!(task.is_finished()).is_false();
}

#[tokio::test]
async fn confirm_no_refuses_whatever_the_focus() {
    let hub = hub();
    let (task, _) = confirming(&hub, ConfirmAsk::Danger(ConfirmVerb::Merge)).await;
    let (outcome, _) = answer(&hub, &said("non"), Some(FocusSource::PageReported));
    assert_that!(outcome).is_equal_to(AnswerOutcome::Answered);
    assert_that!(task.await.unwrap()).is_false();
}

#[tokio::test]
async fn confirm_said_before_armed_keeps_open_too_early() {
    let hub = hub();
    let (task, _) = confirming(&hub, ConfirmAsk::Plain(ConfirmAction::Save)).await;
    let early = HeardUtterance { seq: 2, text: "oui".into(), started: Instant::now() };
    let (outcome, shown) = answer(&hub, &early, Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(shown.unwrap_or_default()).contains("Dit trop tôt");
    assert_that!(task.is_finished()).is_false();
}

#[tokio::test]
async fn plain_confirm_bare_yes_confirms() {
    let hub = hub();
    let (task, _) = confirming(&hub, ConfirmAsk::Plain(ConfirmAction::Save)).await;
    let (outcome, _) = answer(&hub, &said("oui"), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::Answered);
    assert_that!(task.await.unwrap()).is_true();
}

#[tokio::test]
async fn confirm_typed_reply_keeps_it_open_with_say_or_click() {
    let hub = hub();
    let (task, surface) = confirming(&hub, ConfirmAsk::Plain(ConfirmAction::Save)).await;
    let mut rx = hub.subscribe();
    assert_that!(hub.answer(&surface.id, "oui")).is_equal_to(AnswerOutcome::KeptOpen);
    let shown = serde_json::to_string(&next_surface(&mut rx).await).unwrap_or_default();
    assert_that!(shown).contains("Dites votre réponse ou cliquez sur un bouton.");
    assert_that!(task.is_finished()).is_false();
}

#[tokio::test]
async fn answer_voice_on_closed_question_is_not_a_question() {
    let hub = hub();
    let (task, surface) = confirming(&hub, ConfirmAsk::Plain(ConfirmAction::Save)).await;
    hub.cancel_asks();
    assert_that!(task.await.unwrap()).is_false();
    let (outcome, shown) = answer(&hub, &said("oui"), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::NotAQuestion);
    assert_that!(shown).is_none();
    assert_that!(hub.answer(&surface.id, "oui")).is_equal_to(AnswerOutcome::NotAQuestion);
}

#[tokio::test]
async fn permission_yes_picks_allow_once() {
    let hub = hub();
    let task = asking_permission(&hub, "Toujours autoriser").await;
    let (outcome, _) = answer(&hub, &said("oui"), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::Answered);
    assert_that!(task.await.unwrap()).is_equal_to(Some(1));
}

#[rstest]
#[case::page_reported(Some(FocusSource::PageReported))]
#[case::unfocused(None)]
#[tokio::test]
async fn permission_yes_page_reported_focus_keeps_open(#[case] focus: Option<FocusSource>) {
    let hub = hub();
    let task = asking_permission(&hub, "Toujours autoriser").await;
    let (outcome, shown) = answer(&hub, &said("oui"), focus);
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(shown.unwrap_or_default()).contains("Cliquez sur le bouton pour confirmer.");
    assert_that!(task.is_finished()).is_false();
}

#[tokio::test]
async fn permission_said_before_armed_keeps_open_too_early() {
    let hub = hub();
    let task = asking_permission(&hub, "Toujours autoriser").await;
    let early = HeardUtterance { seq: 2, text: "oui".into(), started: Instant::now() };
    let (outcome, shown) = answer(&hub, &early, Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(shown.unwrap_or_default()).contains("Dit trop tôt");
    assert_that!(task.is_finished()).is_false();
}

#[rstest]
#[case::yes_always("oui, toujours")]
#[case::label("toujours autoriser")]
#[case::ordinal("la première")]
#[tokio::test]
async fn permission_always_never_by_voice(#[case] text: &str) {
    let hub = hub();
    let task = asking_permission(&hub, "Toujours autoriser").await;
    let (outcome, shown) = answer(&hub, &said(text), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(shown.unwrap_or_default()).contains("Entendu");
    assert_that!(task.is_finished()).is_false();
}

/// The agent's own `ask_choice` `id`, shown, a sheet when anchored on `slide`; its task.
async fn asking(hub: &Arc<SurfaceHub>, id: &str, slide: Option<&str>) -> JoinHandle<crate::Answer> {
    let mut rx = hub.subscribe();
    let option = |id: &str| oku_slides_a2ui::ChoiceOption { id: id.into(), label: id.into(), hint: None };
    let anchor = slide.map(|s| Anchor { slide: s.into(), owner: s.into(), field: None });
    let spec = AskChoice { question: "Laquelle ?".into(), options: vec![option("a"), option("b")], anchor };
    let mut surface = forms::choice(id, &spec, &Localizer::new(Some("fr")));
    surface.anchor = spec.anchor;
    let task = tokio::spawn({
        let hub = hub.clone();
        async move { hub.ask(surface).await }
    });
    next_surface(&mut rx).await;
    task
}

#[tokio::test]
async fn agent_question_takes_the_spoken_text() {
    let hub = hub();
    let task = asking(&hub, "ask-1", None).await;
    let (outcome, _) = answer(&hub, &said("la deuxième"), Some(FocusSource::PageReported));
    assert_that!(outcome).is_equal_to(AnswerOutcome::Answered);
    assert_that!(task.await.unwrap()).is_equal_to(crate::Answer::text("la deuxième"));
}

/// Speech begun before the question showed is the next instruction, not its answer.
#[tokio::test]
async fn agent_question_utterance_started_before_it_is_a_prompt() {
    let before = Instant::now();
    let hub = hub();
    let task = asking(&hub, "ask-1", None).await;
    let early = HeardUtterance { seq: 2, text: "ajoute une slide".into(), started: before };
    let (outcome, _) = answer(&hub, &early, Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::NotAQuestion);
    assert_that!(task.is_finished()).is_false();
    assert_that!(hub.has_pending_ask()).is_true();
}

/// The page stages the first sheet on the shown slide; the host answers that one (R17).
#[rstest]
#[case::staged_first(Some("s-1"), 0)]
#[case::staged_second(Some("s-2"), 1)]
#[case::none_staged_last_opened(None, 1)]
#[tokio::test]
async fn sheet_target_is_the_one_on_the_staged_slide(#[case] staged: Option<&str>, #[case] want: usize) {
    let hub = hub();
    let mut tasks = vec![asking(&hub, "ask-1", Some("s-1")).await, asking(&hub, "ask-2", Some("s-2")).await];
    let outcome = hub.answer_voice(&said("la deuxième"), &fr(), Some(FocusSource::Host), staged);
    assert_that!(outcome).is_equal_to(AnswerOutcome::Answered);
    let answered = tasks.remove(want);
    assert_that!(answered.await.unwrap()).is_equal_to(crate::Answer::text("la deuxième"));
    assert_that!(tasks.iter().any(JoinHandle::is_finished)).is_false();
}

/// Variants "Sombre" and "Clair" on slide `s-1`.
fn spec() -> ShowVariants {
    let variant = |label: &str| Variant { label: label.into(), ops: Vec::new() };
    ShowVariants { slide: "s-1".into(), variants: vec![variant("Sombre"), variant("Clair")] }
}

/// A variant drawer `show-1` over `NoSession`: a pick reaches the session and fails there.
fn drawer(hub: &Arc<SurfaceHub>) {
    let surface = forms::frozen_choice("show-1", "Variantes", None, &Localizer::new(Some("fr")));
    hub.show_variants(surface, spec());
}

/// A prompt that only mentions a label stays a prompt: the drawer takes the label alone.
#[tokio::test]
async fn variant_prompt_naming_a_label_is_a_prompt() {
    let hub = hub();
    drawer(&hub);
    let (outcome, _) = answer(&hub, &said("mets le titre en sombre"), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::NotAQuestion);
    assert_that!(hub.has_variants()).is_true();
}

/// A pick that fails puts the drawer back, as a click would.
#[tokio::test]
async fn variant_failed_pick_keeps_the_drawer() {
    let hub = hub();
    drawer(&hub);
    let (outcome, _) = answer(&hub, &said("sombre"), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::KeptOpen);
    assert_that!(hub.has_variants()).is_true();
}

/// A failed pick whose drawer another one replaced meanwhile closes the stale one.
#[tokio::test]
async fn variant_failed_pick_after_a_new_drawer_closes_the_stale_one() {
    let hub = hub();
    drawer(&hub);
    let taken: Vec<String> = {
        let mut state = hub.state();
        let ids: Vec<String> =
            state.shows.iter().filter(|(_, s)| matches!(s, Shown::Variants(_))).map(|(id, _)| id.clone()).collect();
        for id in &ids {
            state.shows.remove(id);
        }
        ids
    };
    let surface = forms::frozen_choice("show-2", "Variantes", None, &Localizer::new(Some("fr")));
    hub.show_variants(surface, spec());
    let mut rx = hub.subscribe();
    assert_that!(hub.pick_variant("show-1".into(), spec(), 0, 1)).is_equal_to(AnswerOutcome::KeptOpen);
    let closed: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|e| match e {
            McpEvent::CloseSurface { id } => Some(id),
            _ => None,
        })
        .collect();
    assert_that!(taken).is_equal_to(vec!["show-1".to_owned()]);
    assert_that!(closed).is_equal_to(vec!["show-1".to_owned()]);
    assert_that!(hub.state().shows.keys().cloned().collect::<Vec<_>>()).is_equal_to(vec!["show-2".to_owned()]);
}

#[tokio::test]
async fn variant_non_matching_is_a_prompt() {
    let hub = hub();
    drawer(&hub);
    let (outcome, _) = answer(&hub, &said("ajoute une slide"), Some(FocusSource::Host));
    assert_that!(outcome).is_equal_to(AnswerOutcome::NotAQuestion);
    assert_that!(hub.has_variants()).is_true();
}

/// The modal's guard in the page (`ARMING_MS`) and the host's are one delay, through the fixture.
#[test]
fn arming_equals_the_ui_fixture() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/ui/arming.json");
    let fixture: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_that!(fixture["arming_ms"].as_u64()).is_equal_to(u64::try_from(ARMING.as_millis()).ok());
}
