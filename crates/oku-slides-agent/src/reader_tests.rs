use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use oku_slides_i18n::Localizer;
use oku_slides_mcp::{
    AnswerOutcome, ConfirmAction, ConfirmAsk, ConfirmVerb, HeardUtterance, Lexicon, McpEvent, NoVoice, PresentMode,
    SurfaceHub,
};
use oku_slides_session::agent::{
    AgentWorkspace, AskChoice, FocusSource, FocusedTab, PermissionChoice, PermissionKind, Report, SpokenFailure,
    SpokenText, TabSession, TurnOrigin, forms,
};
use oku_slides_session::{Fonts, Platform, Session, Settings};
use oku_slides_stt::{Speaker, SpeechConfig, Stt};
use rstest::rstest;
use speculoos::prelude::*;
use tokio::sync::broadcast;

use super::{Reader, heard_of, spawn};
use crate::AgentEvent;
use crate::turns::{TurnOut, Turns};
use oku_slides_stt::hand_voice::{HandClock, HandEngine, listen_through_duplex, user_speaks};

/// How long a test waits for the speaker's thread or a hub event: never a sleep.
const WITHIN: Duration = Duration::from_secs(10);

/// The host's focus: always on `0`'s tab, its window focused.
#[derive(Debug)]
struct Focus(String);

impl AgentWorkspace for Focus {
    fn focused_tab(&self) -> Option<FocusedTab> {
        Some(FocusedTab { tab: self.0.clone(), source: FocusSource::Host, focused: true })
    }

    fn unsaved_tabs(&self) -> usize {
        0
    }
}

/// A focus port that panics: the reader's thread must end, logged.
#[derive(Debug)]
struct Broken;

impl AgentWorkspace for Broken {
    fn focused_tab(&self) -> Option<FocusedTab> {
        panic!("focus port broken")
    }

    fn unsaved_tabs(&self) -> usize {
        0
    }
}

/// Tab `t-1`'s hub, a French read-aloud on, and a reader fed by the hub's events.
struct T {
    hub: Arc<SurfaceHub>,
    rx: broadcast::Receiver<McpEvent>,
    reader: Reader,
    voice: Arc<HandEngine>,
    speaker: Speaker,
    stt: Stt,
    clock: Arc<HandClock>,
    session: Arc<Mutex<Session>>,
    turns: Turns,
    rt: tokio::runtime::Runtime,
    _platform: Platform,
    _dir: tempfile::TempDir,
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

/// The fixture; the reader follows `focused`.
fn on(focused: &str) -> T {
    let dir = tempfile::Builder::new().prefix("oku-reader-").tempdir().unwrap_or_else(|e| panic!("dir: {e}"));
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(dir.path().to_owned());
    settings.paths.decks = Some(dir.path().join("decks"));
    settings.paths.models = Some(dir.path().join("models"));
    settings.stt.apple = false;
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    let stt = Stt::new(&platform);
    stt.configure_speech(Some(true), SpeechConfig::default());
    let (voice, clock) = (Arc::new(HandEngine::default()), HandClock::new());
    let speaker = Speaker::with_clock(voice.clone(), Some(stt.clone()), clock.clone());
    let session = Arc::new(Mutex::new(Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("{e}"))));
    let tab = TabSession::new("t-1", session.clone());
    let hub = SurfaceHub::new(tab, platform.live_localizer.clone(), Arc::new(NoVoice));
    let rx = hub.subscribe();
    let reader = Reader::new(speaker.clone(), Some(stt.clone()), Arc::new(Focus(focused.into())));
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("{e}"));
    T {
        hub,
        rx,
        reader,
        voice,
        speaker,
        stt,
        clock,
        session,
        turns: Turns::default(),
        rt,
        _platform: platform,
        _dir: dir,
    }
}

impl T {
    /// Hands the reader every event sent so far, then waits for the speaker.
    fn pump(&mut self) {
        let hub = Arc::downgrade(&self.hub);
        while let Ok(e) = self.rx.try_recv() {
            if let Some(heard) = heard_of("t-1", &hub, &e) {
                self.reader.hear(heard);
            }
        }
        assert!(self.speaker.settle(WITHIN), "the speaker thread should answer");
    }

    /// Waits until the surface whose id starts with `prefix` opens, then pumps.
    fn pump_until(&mut self, prefix: &str) {
        let hub = Arc::downgrade(&self.hub);
        let deadline = Instant::now() + WITHIN;
        loop {
            assert!(Instant::now() < deadline, "no surface {prefix}");
            match self.rx.try_recv() {
                Ok(e) => {
                    let found = matches!(&e, McpEvent::Surface { surface, .. } if surface.id.starts_with(prefix));
                    if let Some(heard) = heard_of("t-1", &hub, &e) {
                        self.reader.hear(heard);
                    }
                    if found {
                        break;
                    }
                }
                Err(_) => std::thread::yield_now(),
            }
        }
        self.pump();
    }

    /// Opens a turn begun by `origin`, as the relay shows it.
    fn turn(&mut self, origin: TurnOrigin) -> u32 {
        let n = self.hub.begin_turn_from(origin);
        let opened = self.turns.apply(
            &AgentEvent::UserText { turn: n, text: "Fais-le".into(), images: Vec::new() },
            Instant::now(),
            &fr(),
        );
        self.relay(opened);
        n
    }

    /// Emits what the turns' relay would.
    fn relay(&self, out: Vec<TurnOut>) {
        for o in out {
            match o {
                TurnOut::Show(s) => self.hub.emit(McpEvent::surface(s)),
                TurnOut::ShowAloud(s, spoken) => self.hub.emit(McpEvent::surface_aloud(s, Some(spoken))),
                TurnOut::Patch { id, messages } => self.hub.emit(McpEvent::SurfacePatch { id, messages }),
                TurnOut::Evict(_) | TurnOut::Ended { .. } => {}
            }
        }
    }

    /// Shows `report` as turn `turn`'s receipt, built by the session as the `report` tool does.
    fn report(&self, turn: u32, report: &Report) {
        self.report_written(turn, report, &[]);
    }

    /// Shows `report` in turn `turn`, with the host's `written` facts after the agent's.
    fn report_written(&self, turn: u32, report: &Report, written: &[String]) {
        let id = self.hub.next_id("report");
        let surfaces = self
            .session
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .report_surface(&id, report, written)
            .unwrap_or_else(|e| panic!("receipt: {e}"));
        assert!(self.hub.show_report(turn, surfaces), "receipt shown");
    }

    /// Asks the agent's choice question `question` in turn `turn`, read aloud.
    fn ask(&self, turn: u32, question: &str, labels: &[&str]) -> String {
        let options: Vec<serde_json::Value> =
            labels.iter().enumerate().map(|(i, l)| serde_json::json!({ "id": format!("o{i}"), "label": l })).collect();
        let id = self.hub.next_id("ask");
        let spec: AskChoice = serde_json::from_value(serde_json::json!({ "question": question, "options": options }))
            .unwrap_or_else(|e| panic!("choice: {e}"));
        let surface = forms::choice(&id, &spec, &fr());
        let spoken = SpokenText::question(question, labels, &fr());
        let hub = self.hub.clone();
        drop(self.rt.spawn(async move { hub.ask_aloud(Some(turn), surface, None, spoken).await }));
        id
    }

    fn confirm(&self, message: &str, ask: ConfirmAsk) {
        let (hub, message) = (self.hub.clone(), message.to_owned());
        drop(self.rt.spawn(async move { hub.confirm(message, ask).await }));
    }

    /// The agent's `ask_confirm`, opened as its tool opens it.
    fn ask_confirm(&self, message: &str, danger: bool) {
        let l = fr();
        let verb = danger.then(|| l.tr("agent-surface-confirm-danger"));
        let surface = forms::confirm(&self.hub.next_id("ask"), message, verb.clone(), None, &l);
        let spoken = SpokenText::agent_confirm(message, verb.as_deref(), &l);
        let hub = self.hub.clone();
        drop(self.rt.spawn(async move { hub.ask_aloud(None, surface, None, spoken).await }));
    }

    fn texts(&self) -> Vec<String> {
        self.voice.texts()
    }
}

fn report(title: &str, facts: &[&str]) -> Report {
    Report {
        title: title.into(),
        facts: facts.iter().map(|f| (*f).to_owned()).collect(),
        slide: None,
        swatch: None,
        why: None,
        actions: Vec::new(),
    }
}

#[test]
fn receipt_reads_title_and_four_facts() {
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    let facts = ["Diapo 2", "Titre en blanc", "Police plus grande", "Logo à droite"];
    // A fifth fact, the host's own: past the cap, never read.
    t.report_written(n, &report("Fond bleu nuit", &facts), &["Cinquième fait".to_owned()]);
    t.pump();
    assert_that!(t.texts())
        .is_equal_to(vec!["Fond bleu nuit. Diapo 2. Titre en blanc. Police plus grande. Logo à droite".to_owned()]);
}

#[test]
fn question_reads_options_in_order() {
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    t.ask(n, "Quel fond ?", &["Bleu", "Rouge", "Vert"]);
    t.pump_until("ask");
    let l = fr();
    let option = |n: u64, o: &str| l.tr_with("voice-say-option", &[("n", n.into()), ("option", o.into())]);
    let expected = [
        l.tr_with("voice-say-question", &[("question", "Quel fond ?".into())]),
        option(1, "Bleu"),
        option(2, "Rouge"),
        option(3, "Vert"),
    ]
    .join(" ");
    assert_that!(t.texts()).is_equal_to(vec![expected]);
    assert_that!(t.texts()[0].as_str()).contains("Option 1 : Bleu.");
}

#[test]
fn confirmation_reads_verb() {
    let mut t = on("t-1");
    t.confirm("Fusionner le deck ?", ConfirmAsk::Danger(ConfirmVerb::Merge));
    t.pump_until("confirm");
    let l = fr();
    let verb = l.tr(ConfirmVerb::Merge.label_key());
    assert_that!(t.texts()).is_equal_to(vec![l.tr_with("voice-say-confirm", &[("verb", verb.into())])]);
}

#[test]
fn turn_failure_is_read() {
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    let failed =
        AgentEvent::Error { turn: n, message: "L'agent s'est arrêté".into(), spoken: SpokenFailure::Stopped };
    let out = t.turns.apply(&failed, Instant::now(), &fr());
    t.relay(out);
    t.pump();
    assert_that!(t.texts()).is_equal_to(vec![fr().tr("voice-say-agent-stopped")]);
}

#[rstest]
#[case::plain(false)]
#[case::danger(true)]
fn agent_confirm_reads_how_to_answer(#[case] danger: bool) {
    let mut t = on("t-1");
    t.turn(TurnOrigin::Typed);
    t.ask_confirm("On continue ?", danger);
    t.pump_until("ask");
    let l = fr();
    let how = if danger {
        l.tr_with("voice-say-confirm", &[("verb", l.tr("agent-surface-confirm-danger").into())])
    } else {
        l.tr("voice-say-confirm-yes")
    };
    let question = l.tr_with("voice-say-question", &[("question", "On continue ?".into())]);
    assert_that!(t.texts()).is_equal_to(vec![format!("{question} {how}")]);
}

#[test]
fn agent_text_in_turn_failure_is_never_read() {
    use oku_slides_i18n::Localize;
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    let e: crate::AgentError = crate::error::RequestSnafu { message: "SECRET-77 /Users/x" }.build().into();
    let failed = AgentEvent::Error { turn: n, message: e.localize(&fr()), spoken: e.spoken() };
    let out = t.turns.apply(&failed, Instant::now(), &fr());
    t.relay(out);
    t.pump();
    assert_that!(t.texts()).is_equal_to(vec![fr().tr("voice-say-agent-refused")]);
}

#[test]
fn progress_and_history_are_silent() {
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    let call = AgentEvent::ToolCall {
        turn: n,
        id: "c-1".into(),
        title: "Lecture du deck".into(),
        status: "in_progress".into(),
        input: None,
        output: None,
    };
    let out = t.turns.apply(&call, Instant::now(), &fr());
    t.relay(out);
    let chunk = AgentEvent::Chunk { turn: n, text: "Je regarde le deck.".into() };
    let out = t.turns.apply(&chunk, Instant::now(), &fr());
    t.relay(out);
    t.hub.resync();
    t.pump();
    assert_that!(t.texts()).is_empty();
}

#[test]
fn presenting_tab_is_muted() {
    let mut t = on("t-1");
    t.hub.set_presenting(Some(PresentMode::Screen), 1);
    let n = t.turn(TurnOrigin::Wake);
    t.report(n, &report("Diapo suivante", &["Diapo 3"]));
    t.pump();
    assert_that!(t.texts()).is_empty();
}

#[rstest]
#[case::wake_on(TurnOrigin::Wake, true, 1)]
#[case::wake_off(TurnOrigin::Wake, false, 0)]
#[case::voice_on(TurnOrigin::Voice, true, 0)]
fn presenting_wake_turn_reads_with_in_presentation(
    #[case] origin: TurnOrigin,
    #[case] in_presentation: bool,
    #[case] read: usize,
) {
    let mut t = on("t-1");
    t.stt.configure_speech(Some(true), SpeechConfig { in_presentation, ..SpeechConfig::default() });
    t.hub.set_presenting(Some(PresentMode::Window), 1);
    let n = t.turn(origin);
    t.report(n, &report("Diapo suivante", &["Diapo 3"]));
    t.pump();
    assert_that!(t.texts()).has_length(read);
}

#[test]
fn new_turn_cuts_speech() {
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    t.report(n, &report("Fond bleu nuit", &["Diapo 2"]));
    t.pump();
    assert_that!(t.speaker.speaking()).is_true();
    t.turn(TurnOrigin::Typed);
    t.pump();
    assert_that!(t.speaker.speaking()).is_false();
    assert_that!(t.voice.calls().stopped.len()).is_equal_to(1);
}

#[test]
fn other_tab_is_not_read() {
    let mut t = on("t-2");
    let n = t.turn(TurnOrigin::Typed);
    t.report(n, &report("Fond bleu nuit", &["Diapo 2"]));
    t.pump();
    assert_that!(t.texts()).is_empty();
}

#[test]
fn agent_text_in_permission_is_never_read() {
    let mut t = on("t-1");
    let hub = t.hub.clone();
    let options = vec![
        PermissionChoice { name: "Oui, lire ~/secret".into(), kind: PermissionKind::AllowOnce },
        PermissionChoice { name: "Non merci".into(), kind: PermissionKind::RejectOnce },
    ];
    drop(t.rt.spawn(async move { hub.permission("Lire le fichier SECRET-42 ?", &options).await }));
    t.pump_until("permission");
    assert_that!(t.texts()).is_equal_to(vec![fr().tr("voice-say-permission")]);
}

#[rstest]
#[case::discard_names_a_tab("Fermer « Rapport confidentiel » sans enregistrer ?", ConfirmVerb::Discard, "confidentiel")]
#[case::import_holds_a_path("Importer /Users/ana/Secret/plan.pptx ?", ConfirmVerb::Import, "plan.pptx")]
fn confirmation_message_is_never_read(#[case] message: &str, #[case] verb: ConfirmVerb, #[case] private: &str) {
    let mut t = on("t-1");
    t.confirm(message, ConfirmAsk::Danger(verb));
    t.pump_until("confirm");
    let said = t.texts();
    assert_that!(said).has_length(1);
    assert_that!(said[0].as_str()).does_not_contain(private);
}

#[test]
fn agent_why_and_slide_content_are_never_read() {
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    let mut r = report("Titre changé", &["Diapo 1"]);
    r.why = Some("POURQUOI-SECRET".into());
    t.report(n, &r);
    t.pump();
    // The fallback receipt is the agent's narration: never read.
    let m = t.turn(TurnOrigin::Typed);
    t.hub.end_turn(m, "NARRATION-SECRETE du contenu de la diapo");
    t.pump();
    let said = t.texts().join(" | ");
    assert_that!(said.as_str()).contains("Titre changé");
    assert_that!(said.as_str()).does_not_contain("POURQUOI-SECRET");
    assert_that!(said.as_str()).does_not_contain("NARRATION-SECRETE");
}

#[test]
fn new_question_cuts_speech() {
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    t.report(n, &report("Fond bleu nuit", &["Diapo 2"]));
    t.pump();
    t.ask(n, "Et le titre ?", &["Blanc", "Noir"]);
    t.pump_until("ask");
    assert_that!(t.texts()).has_length(2);
    // The receipt was cut by the question (`speak` cuts): its late DONE leaves the question playing.
    t.voice.done(1);
    assert!(t.speaker.settle(WITHIN), "the speaker thread should answer");
    assert_that!(t.speaker.speaking()).is_true();
    t.voice.done(2);
    assert!(t.speaker.settle(WITHIN), "the speaker thread should answer");
    assert_that!(t.speaker.speaking()).is_false();
}

#[test]
fn question_speech_end_moves_arming() {
    let mut t = on("t-1");
    t.confirm("Fusionner ?", ConfirmAsk::Plain(ConfirmAction::Save));
    t.pump_until("confirm");
    // The question's speech ends 10 s later, by the speaker's clock.
    t.clock.advance(Duration::from_secs(10));
    let ended = t.clock_now();
    t.voice.finish_all();
    assert!(t.speaker.settle(WITHIN), "the speaker thread should answer");
    let lexicon = Lexicon::for_locales("fr", Some("fr"));
    let heard = |seq: u64, ms: u64| HeardUtterance {
        seq,
        text: "non".into(),
        started: ended.checked_add(Duration::from_millis(ms)).unwrap_or_else(|| panic!("clock overflow")),
    };
    let early = t.hub.answer_voice(&heard(1, 200), &lexicon, Some(FocusSource::Host), None);
    assert_that!(early).is_equal_to(AnswerOutcome::KeptOpen);
    let armed = t.hub.answer_voice(&heard(2, 600), &lexicon, Some(FocusSource::Host), None);
    assert_that!(armed).is_equal_to(AnswerOutcome::Answered);
}

#[test]
fn barge_in_answer_over_question_is_taken() {
    let mut t = on("t-1");
    let duplex = Arc::new(HandEngine::default());
    listen_through_duplex(&t.stt, duplex.clone());
    let start = t.clock_now();
    t.confirm("Fusionner ?", ConfirmAsk::Plain(ConfirmAction::Save));
    t.pump_until("confirm");
    assert_that!(duplex.texts()).has_length(1);
    // Mid-question, the user answers over it: the barge-in cuts it 10 s in.
    let answer = HeardUtterance {
        seq: 1,
        text: "oui".into(),
        started: start.checked_add(Duration::from_secs(5)).unwrap_or_else(|| panic!("clock overflow")),
    };
    t.clock.advance(Duration::from_secs(10));
    user_speaks(&t.stt, true);
    t.speaker.tick();
    assert!(t.speaker.settle(WITHIN), "the speaker thread should answer");
    assert_that!(t.speaker.speaking()).is_false();
    let lexicon = Lexicon::for_locales("fr", Some("fr"));
    let outcome = t.hub.answer_voice(&answer, &lexicon, Some(FocusSource::Host), None);
    assert_that!(outcome).is_equal_to(AnswerOutcome::Answered);
}

#[test]
fn barge_in_answer_over_agent_question_is_taken() {
    let mut t = on("t-1");
    let duplex = Arc::new(HandEngine::default());
    listen_through_duplex(&t.stt, duplex.clone());
    let start = t.clock_now();
    let n = t.turn(TurnOrigin::Typed);
    t.ask(n, "Et le titre ?", &["Blanc", "Noir"]);
    t.pump_until("ask");
    assert_that!(duplex.texts()).has_length(1);
    let answer = HeardUtterance {
        seq: 1,
        text: "blanc".into(),
        started: start.checked_add(Duration::from_secs(5)).unwrap_or_else(|| panic!("clock overflow")),
    };
    t.clock.advance(Duration::from_secs(10));
    user_speaks(&t.stt, true);
    t.speaker.tick();
    assert!(t.speaker.settle(WITHIN), "the speaker thread should answer");
    let lexicon = Lexicon::for_locales("fr", Some("fr"));
    // Not a new instruction: it answers the question it cut.
    let outcome = t.hub.answer_voice(&answer, &lexicon, Some(FocusSource::Host), None);
    assert_that!(outcome).is_equal_to(AnswerOutcome::Answered);
}

#[test]
fn reader_panic_is_logged_and_thread_ends() {
    let t = on("t-1");
    let reader = Reader::new(t.speaker.clone(), Some(t.stt.clone()), Arc::new(Broken));
    let (tx, handle) = spawn(reader);
    let heard =
        || super::Heard { tab: "t-1".into(), hub: Arc::downgrade(&t.hub), id: "x".into(), turn: None, spoken: None };
    assert_that!(tx.hear(heard())).is_true();
    let ended = handle.unwrap_or_else(|| panic!("reader thread")).join();
    assert_that!(ended.is_ok()).is_true();
    assert_that!(tx.hear(heard())).is_false();
}

impl T {
    fn clock_now(&self) -> Instant {
        oku_slides_stt::SpeakerClock::now(&*self.clock)
    }
}

#[test]
fn reader_question_speech_is_never_taken_for_an_echo() {
    let mut t = on("t-1");
    t.turn(TurnOrigin::Typed);
    t.ask_confirm("On continue ?", true);
    t.pump_until("ask");
    // The danger verb the speech tells the user to say: their answer, not an echo.
    assert_that!(
        t.speaker.echoes(&fr().tr("agent-surface-confirm-danger"), oku_slides_stt::SpeakerClock::now(&*t.clock))
    )
    .is_false();
}

#[test]
fn reader_receipt_speech_is_taken_for_an_echo() {
    let mut t = on("t-1");
    let n = t.turn(TurnOrigin::Typed);
    t.report(n, &report("Fond bleu nuit", &["Titre en blanc"]));
    t.pump();
    assert_that!(t.speaker.echoes("titre en blanc", oku_slides_stt::SpeakerClock::now(&*t.clock))).is_true();
}
