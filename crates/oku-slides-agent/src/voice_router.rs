//! The host voice router: each closed utterance is routed before any agent launch.
//!
//! A thread of its own takes the utterances of the dictation ([`Stt::utterances`]) and routes
//! each one ([`route_voice`]): the stop word, then the wake phrase, then focus gating, then a
//! theme shortcut word (not while presenting), then an open question or drawer, then the
//! presentation lexicon (a command sent to the page at once, else nothing), else a prompt.
//! It holds the stack weakly (the stack holds the dictation, which feeds this thread) and ends
//! when the stack is gone or closing. Utterance text is never logged: logs carry the seq only.
//!
//! [`Stt::utterances`]: oku_slides_stt::Stt::utterances

use std::panic::AssertUnwindSafe;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use oku_slides_mcp::{AnswerOutcome, HeardUtterance, Lexicon, McpEvent, Nav, PresentCommand};
use oku_slides_session::agent::{FocusSource, TurnOrigin};
use oku_slides_stt::Utterance;

use crate::Stack;
use crate::stack::show_theme_shortcut;
use crate::tab_agent::{Command, Instruction};

/// How long the router waits for an utterance before checking the stack is still there.
const POLL: Duration = Duration::from_millis(250);

/// Where a closed utterance goes, decided before any agent launch.
#[derive(Debug)]
pub(crate) enum VoiceRoute {
    /// The stop word: cancel that tab's turn, no new prompt.
    Stop { tab: String },
    /// An answer to the tab's open question or variant drawer.
    Answer { tab: String, heard: HeardUtterance },
    /// A presentation command.
    Nav { tab: String, nav: Nav },
    /// A theme shortcut word: the host's picker, no turn; `text` goes to the agent out of deck mode.
    Shortcut { tab: String, text: String },
    /// A prompt for the tab's agent; `origin` says whether the wake phrase began it.
    Prompt { tab: String, text: String, origin: TurnOrigin },
    /// Nowhere; logged with the seq only.
    Drop(DropReason),
}

/// Why an utterance went nowhere; logged with the seq only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DropReason {
    /// No tab to route to: no window, a closed tab, or an agent that cannot start.
    NoTarget,
    /// No window of the app has the focus, and it is neither stop nor wake.
    Background,
    /// The tab presents, and it is not a presentation command.
    Presenting,
    /// Nothing left to say once trimmed: a bare wake phrase, or filler words only.
    Noise,
}

/// The listening tab and its state, read from the workspace and the hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VoiceTarget {
    /// The tab utterances go to.
    pub tab: String,
    /// Who says its window has the focus; `None` while the app is in the background.
    pub focus: Option<FocusSource>,
    /// Its page last said it presents.
    pub presenting: bool,
    /// One of its questions waits for an answer.
    pub open_question: bool,
    /// It shows a variant drawer.
    pub drawer: bool,
}

/// Routes `u` to `target`; the lexicon tells its command words, the theme shortcut included.
///
/// Order: stop, wake phrase, focus gating, shortcut (not while presenting), open question or
/// drawer, filler-only noise, presentation (nav, else dropped), prompt.
pub(crate) fn route_voice(u: &Utterance, target: Option<&VoiceTarget>, lexicon: &Lexicon) -> VoiceRoute {
    let Some(t) = target else { return VoiceRoute::Drop(DropReason::NoTarget) };
    let text = u.text.trim();
    let tab = t.tab.clone();
    if text.is_empty() {
        return VoiceRoute::Drop(DropReason::Noise);
    }
    if lexicon.stop(text) {
        return VoiceRoute::Stop { tab };
    }
    if let Some(rest) = lexicon.wake(text) {
        let rest = rest.trim();
        // A wake phrase trailing off in filler ("oku, uh…") is noise too: no prompt cuts the turn.
        if rest.is_empty() || lexicon.is_filler_only(rest) {
            return VoiceRoute::Drop(DropReason::Noise);
        }
        return VoiceRoute::Prompt { tab, text: rest.to_owned(), origin: TurnOrigin::Wake };
    }
    if t.focus.is_none() {
        return VoiceRoute::Drop(DropReason::Background);
    }
    if !t.presenting && lexicon.theme_shortcut(text) {
        return VoiceRoute::Shortcut { tab, text: text.to_owned() };
    }
    if t.open_question || t.drawer {
        let heard = HeardUtterance { seq: u.seq.0, text: text.to_owned(), started: u.started };
        return VoiceRoute::Answer { tab, heard };
    }
    // Only filler words ("hmm", "uh"): noise, never a prompt that would cut the turn. After
    // answers, since a question reads pt "um" (an en filler) as an ordinal.
    if lexicon.is_filler_only(text) {
        return VoiceRoute::Drop(DropReason::Noise);
    }
    if t.presenting {
        return match lexicon.nav(text) {
            Some(nav) => VoiceRoute::Nav { tab, nav },
            None => VoiceRoute::Drop(DropReason::Presenting),
        };
    }
    VoiceRoute::Prompt { tab, text: text.to_owned(), origin: TurnOrigin::Voice }
}

/// How the voice router thread ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterEnd {
    /// The stack is gone or closing.
    Closed,
    /// The dictation dropped its sender.
    Disconnected,
    /// A routing step panicked; the panic was logged.
    Panicked,
}

/// Feeds `step` each utterance (`None` after each idle [`POLL`]) until it returns false.
///
/// A panic in `step` is caught and logged: the thread ends cleanly.
pub(crate) fn run(rx: &Receiver<Utterance>, mut step: impl FnMut(Option<Utterance>) -> bool) -> RouterEnd {
    let looped = std::panic::catch_unwind(AssertUnwindSafe(|| {
        loop {
            let next = match rx.recv_timeout(POLL) {
                Ok(u) => Some(u),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return RouterEnd::Disconnected,
            };
            if !step(next) {
                return RouterEnd::Closed;
            }
        }
    }));
    looped.unwrap_or_else(|_| {
        tracing::error!("voice router panicked: no more utterances are routed");
        RouterEnd::Panicked
    })
}

/// Starts the router thread over `rx` for `stack`, held weakly.
pub(crate) fn spawn(stack: Weak<Stack>, rx: Receiver<Utterance>) -> std::io::Result<JoinHandle<RouterEnd>> {
    std::thread::Builder::new().name("voice-router".into()).spawn(move || {
        run(&rx, |u| {
            let Some(stack) = stack.upgrade() else { return false };
            if stack.is_closing() {
                return false;
            }
            if let Some(u) = u {
                stack.on_utterance(&u);
            }
            true
        })
    })
}

impl Stack {
    /// Routes the utterances of `rx` on a thread of their own until the stack shuts down.
    ///
    /// [`Stack::start`] does it for its dictation; a host with another source calls it once.
    /// Returns the router thread, `None` (logged) if it could not start.
    pub fn route_utterances(self: &Arc<Self>, rx: Receiver<Utterance>) -> Option<JoinHandle<RouterEnd>> {
        spawn(Arc::downgrade(self), rx).inspect_err(|e| tracing::warn!(error = %e, "voice router not started")).ok()
    }

    /// Routes one utterance: never its text in the log.
    pub(crate) fn on_utterance(&self, u: &Utterance) {
        let seq = u.seq.0;
        // Begun while the read-aloud spoke, or within its guard: the microphone heard the speakers.
        if self.speaker.as_ref().is_some_and(|s| !s.hears(u.started)) {
            tracing::debug!(seq, "voice: dropped, heard during the read-aloud");
            return;
        }
        // Begun just after, but repeating words of a recent speech: its echo, not the user.
        if self.speaker.as_ref().is_some_and(|s| s.echoes(&u.text, u.started)) {
            tracing::debug!(seq, "voice: dropped, an echo of the read-aloud");
            return;
        }
        let target = self.voice_target();
        let ui = self.platform.live_localizer.current();
        let dictation = self.voice.dictation_locale();
        let lexicon = Lexicon::for_locales(ui.locale(), dictation);
        match route_voice(u, target.as_ref(), &lexicon) {
            VoiceRoute::Stop { tab } => {
                tracing::debug!(seq, "voice: stop");
                self.cancel(&tab);
            }
            VoiceRoute::Answer { tab, heard } => {
                let (focus, presenting) = target.as_ref().map_or((None, false), |t| (t.focus, t.presenting));
                self.voice_answer(&tab, heard, &lexicon, focus, presenting);
            }
            VoiceRoute::Nav { tab, nav } => self.voice_nav(seq, &tab, nav),
            VoiceRoute::Shortcut { tab, text } => self.voice_shortcut(seq, &tab, text),
            VoiceRoute::Prompt { tab, text, origin } => self.voice_prompt(seq, &tab, text, origin),
            VoiceRoute::Drop(reason) => tracing::debug!(seq, ?reason, "voice: dropped"),
        }
    }

    /// The tab that listens, as the host names it: focused, or left in the background.
    ///
    /// `None` without any, or when the agent cannot start. A dormant tab is not started here:
    /// it has no flags yet, and only the arms that need it (prompt, shortcut) start it.
    fn voice_target(&self) -> Option<VoiceTarget> {
        let listening = self.workspace.focused_tab()?;
        let (tab, focus) = (listening.tab, listening.focused.then_some(listening.source));
        if self.availability().is_some() {
            return None;
        }
        let t = self.tab(&tab);
        let flag = |f: fn(&oku_slides_mcp::SurfaceHub) -> bool| t.as_ref().is_some_and(|t| f(&t.hub));
        Some(VoiceTarget {
            presenting: flag(|hub| hub.presenting().is_some()),
            open_question: flag(oku_slides_mcp::SurfaceHub::has_pending_ask),
            drawer: flag(oku_slides_mcp::SurfaceHub::has_variants),
            tab,
            focus,
        })
    }

    /// Answers `tab`'s open question or drawer; a prompt only if neither takes the utterance.
    ///
    /// The hub decides under one lock, so the prompt follows only a `NotAQuestion`. While the
    /// tab presents, what no question takes is a presentation command or nothing.
    fn voice_answer(
        &self,
        tab: &str,
        heard: HeardUtterance,
        lexicon: &Lexicon,
        focus: Option<FocusSource>,
        presenting: bool,
    ) {
        let Some(t) = self.open(tab) else { return };
        // The slide the page shows: its sheet question is the one the page stages.
        let staged = self.reports.last_context(tab).and_then(|c| c.ui.slide);
        match t.hub.answer_voice(&heard, lexicon, focus, staged.as_deref()) {
            AnswerOutcome::NotAQuestion if presenting => {
                if let Some(nav) = lexicon.nav(&heard.text) {
                    self.voice_nav(heard.seq, tab, nav);
                } else {
                    tracing::debug!(seq = heard.seq, reason = ?DropReason::Presenting, "voice: dropped");
                }
            }
            AnswerOutcome::NotAQuestion => self.voice_prompt(heard.seq, tab, heard.text, TurnOrigin::Voice),
            outcome => tracing::debug!(seq = heard.seq, ?outcome, "voice: answer"),
        }
    }

    /// A presentation command to `tab`'s page, at once and with no agent.
    fn voice_nav(&self, seq: u64, tab: &str, nav: Nav) {
        let Some(t) = self.tab(tab) else { return };
        tracing::debug!(seq, ?nav, "voice: presentation command");
        t.hub.emit(McpEvent::Present { command: PresentCommand::from(nav) });
    }

    /// The theme picker in `tab`, at once: the lexicon matched, so the raw text is not matched again.
    ///
    /// Out of deck mode, or if it fails (logged), the text goes to the agent as a prompt.
    fn voice_shortcut(&self, seq: u64, tab: &str, text: String) {
        let Some(t) = self.open(tab) else { return };
        let slide = self.reports.last_context(tab).and_then(|c| c.ui.slide);
        if show_theme_shortcut(&t, tab, slide.as_deref()) {
            tracing::debug!(seq, "voice: theme shortcut");
        } else {
            self.voice_prompt(seq, tab, text, TurnOrigin::Voice);
        }
    }

    /// A voice prompt to `tab`, with its page's last context; a typed-rule shortcut word shows the picker.
    fn voice_prompt(&self, seq: u64, tab: &str, text: String, origin: TurnOrigin) {
        let ui = self.reports.last_context(tab).map(|c| c.ui).unwrap_or_default();
        match self.route_prompt(tab, None, &text, ui.slide.as_deref(), &[]) {
            Ok(Some((t, images))) => {
                tracing::debug!(seq, ?origin, "voice: prompt");
                t.send(Command::Prompt(Instruction { text, ui, tune: false, images, origin }));
            }
            Ok(None) => tracing::debug!(seq, "voice: handled by the host"),
            // No channel tells a tab an error outside a turn, so the log tells it (never the
            // text). With no images this cannot fail today.
            Err(_) => tracing::warn!(seq, "voice: prompt refused"),
        }
    }
}

#[cfg(test)]
#[path = "voice_router_tests.rs"]
mod voice_router_tests;
