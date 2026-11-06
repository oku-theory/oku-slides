//! The read-aloud of the focused tab: receipts, questions and errors, as their surfaces open.
//!
//! Each tab's MCP relay ([`heard_of`]) hands the reader the surfaces that carry a
//! [`SpokenText`], built host-side, and those that open a turn; nothing is called from the
//! hub's `emit`. The reader runs on its own thread, reads the focused tab with no hub lock
//! held, and speaks through the process-wide [`Speaker`], which cuts the speech in progress.
//!
//! It speaks only for the focused tab, once the user turned it on. While that tab presents it
//! is muted, unless `in_presentation` is on and the turn began with the wake phrase. A new
//! turn cuts the speech; a question cuts it and is read, its arming moved to the end of its
//! speech. Spoken text and tab ids are never logged with it.

use std::collections::HashMap;
use std::sync::{Arc, Weak, mpsc};

use oku_slides_mcp::{McpEvent, SurfaceHub};
use oku_slides_session::agent::{AgentWorkspace, SpokenKind, SpokenText, TurnOrigin};
use oku_slides_stt::{Speaker, Speech, SpeechEnded, SpeechRate, Stt};

/// A surface that opened in a tab: what the reader may say of it.
#[derive(Debug)]
pub(crate) struct Heard {
    /// The tab it opened in.
    pub tab: String,
    /// That tab's hub: its turn, presentation and pending questions.
    pub hub: Weak<SurfaceHub>,
    /// The surface's id: a question's, for its arming.
    pub id: String,
    /// The turn it belongs to, if it is of the thread.
    pub turn: Option<u32>,
    /// What to say of it; `None`: nothing (a turn opening may still cut).
    pub spoken: Option<SpokenText>,
}

/// What the reader hears of `event` in `tab`; `None` for anything but a surface.
pub(crate) fn heard_of(tab: &str, hub: &Weak<SurfaceHub>, event: &McpEvent) -> Option<Heard> {
    let McpEvent::Surface { surface, spoken } = event else { return None };
    Some(Heard {
        tab: tab.to_owned(),
        hub: hub.clone(),
        id: surface.id.clone(),
        turn: surface.turn,
        spoken: spoken.clone(),
    })
}

/// Where the tabs' relays send what opened; cheap to clone, never blocks.
#[derive(Debug, Clone)]
pub(crate) struct ReaderTx(mpsc::Sender<Heard>);

impl ReaderTx {
    /// Hands `heard` to the reader; `false` once its thread has ended.
    pub(crate) fn hear(&self, heard: Heard) -> bool {
        self.0.send(heard).is_ok()
    }
}

/// The reader's state: the voice, the settings and the turns seen per tab.
#[derive(Debug)]
pub(crate) struct Reader {
    speaker: Speaker,
    /// Where the read-aloud settings are saved; `None`: never on.
    stt: Option<Stt>,
    workspace: Arc<dyn AgentWorkspace>,
    /// The newest turn seen opening, per tab.
    turns: HashMap<String, u32>,
}

impl Reader {
    /// A reader speaking through `speaker`, with the settings of `stt`, for `workspace`'s focus.
    pub(crate) fn new(speaker: Speaker, stt: Option<Stt>, workspace: Arc<dyn AgentWorkspace>) -> Self {
        Self { speaker, stt, workspace, turns: HashMap::new() }
    }

    /// Says what `heard` should, if anything; may block on the focus port, never on a hub lock.
    pub(crate) fn hear(&mut self, heard: Heard) {
        if self.workspace.focused_tab().is_none_or(|f| f.tab != heard.tab) {
            return;
        }
        let Some(hub) = heard.hub.upgrade() else { return };
        if let Some(turn) = heard.turn {
            let last = self.turns.entry(heard.tab.clone()).or_default();
            if turn > *last {
                *last = turn;
                self.speaker.cut();
            }
        }
        let Some(spoken) = heard.spoken else { return };
        let Some(stt) = &self.stt else { return };
        let (read_aloud, config) = stt.speech_settings();
        if read_aloud != Some(true) {
            return;
        }
        if hub.presenting().is_some() {
            let origin = heard.turn.or_else(|| hub.turn()).and_then(|t| hub.turn_origin(t));
            if !(config.in_presentation && origin == Some(TurnOrigin::Wake)) {
                tracing::debug!("read-aloud: muted while presenting");
                return;
            }
        }
        let ended = (spoken.kind() == SpokenKind::Question).then(|| {
            let (hub, id) = (heard.hub.clone(), heard.id);
            SpeechEnded::new(move |at| {
                if let Some(hub) = hub.upgrade() {
                    hub.spoken_until(&id, at);
                }
            })
        });
        let rate = SpeechRate::new(config.rate).unwrap_or_else(|_| {
            // Only a hand-edited config.json holds a rate set_speech would refuse.
            tracing::warn!(rate = config.rate, "saved speech rate out of range: the system's own applies");
            SpeechRate::default()
        });
        // A question's words are the user's answers: never taken for an echo of it.
        let asks = spoken.kind() == SpokenKind::Question;
        let speech = Speech {
            text: spoken.text().to_owned(),
            lang: spoken.lang().to_owned(),
            voice: config.voice,
            rate,
            ended,
            asks,
        };
        // A new speech cuts the one in progress: a question never waits behind a receipt.
        self.speaker.say(speech);
    }
}

/// Starts the reader's thread; it ends once every [`ReaderTx`] is gone, or on a panic, logged.
pub(crate) fn spawn(reader: Reader) -> (ReaderTx, Option<std::thread::JoinHandle<()>>) {
    let (tx, rx) = mpsc::channel::<Heard>();
    let spawned = std::thread::Builder::new().name("oku-reader".into()).spawn(move || run(reader, &rx));
    match spawned {
        Ok(handle) => (ReaderTx(tx), Some(handle)),
        Err(e) => {
            tracing::warn!(error = %e, "read-aloud reader not started: nothing is read aloud");
            (ReaderTx(tx), None)
        }
    }
}

/// Reads until the channel closes; a panic is logged and ends the thread, never the app.
fn run(mut reader: Reader, rx: &mpsc::Receiver<Heard>) {
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        while let Ok(heard) = rx.recv() {
            reader.hear(heard);
        }
    }));
    if ran.is_err() {
        tracing::error!("read-aloud reader panicked: nothing more is read aloud");
    }
}

#[cfg(test)]
#[path = "reader_tests.rs"]
mod reader_tests;
