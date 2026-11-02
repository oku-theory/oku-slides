//! A turn's route before `connect`: the local agent's notice, the model download, the retry.
//!
//! The route comes from `AppVoice::turn_route` (see `fallback`). On the local agent, an A2UI
//! line in the turn says why; a missing model is downloaded once the user confirms, its
//! progress shown in the turn; a launch failure of the selected agent found at `connect` routes
//! the turn once more.

use std::sync::Arc;
use std::time::Duration;

use oku_slides_i18n::Localize;
use oku_slides_platform::local_agent::AGENT_MODEL;
use oku_slides_session::agent::receipt;
use oku_slides_session::{LocalModelView, agent_download_progress, gigabytes_text, local_failure_text};
use tokio::sync::mpsc;

use super::{Command, Known, TabAgent};
use crate::AgentEvent;
use crate::error::{AgentError, Reason};
use crate::link::Live;
use crate::voice_port::{AppVoice, TurnRoute, both_reasons};

/// How often the download's progress is read again.
const POLL: Duration = Duration::from_millis(250);

/// What became of a turn's connection.
pub(super) enum Connected {
    /// Connected: the turn goes on.
    Live(Live),
    /// The turn failed; its error is shown.
    Failed,
    /// A command came first: the turn ended, the command is to handle next.
    Interrupted(Option<Command>),
}

/// What a route gives the turn.
enum Routed {
    /// It connects to the selected agent.
    Main,
    /// It connects to the local agent.
    Local,
    /// It failed; its error is shown.
    Failed,
    /// A command came first.
    Interrupted(Option<Command>),
}

/// What became of a model download asked by a turn.
enum Download {
    /// The model is there.
    Done,
    /// The user said no.
    Declined,
    /// The download failed, localized.
    Failed(String),
    /// A command came first.
    Interrupted(Option<Command>),
}

impl TabAgent {
    /// Routes turn `n`, then connects; a launch failure of the selected agent routes it once more.
    ///
    /// The route switched the link: `known` is looked up again, for the agent it runs now.
    pub(super) async fn connect_routed(
        &mut self,
        rx: &mut mpsc::UnboundedReceiver<Command>,
        n: u32,
        known: &mut Known,
    ) -> Connected {
        let Some(voice) = self.ctx.voice.clone() else {
            return match self.link.connect().await {
                Ok(live) => Connected::Live(live),
                Err(e) => {
                    self.connect_failed(n, &e, None);
                    Connected::Failed
                }
            };
        };
        let mut first: Option<AgentError> = None;
        loop {
            let on_main = match self.take_route(&voice, rx, n).await {
                Routed::Main => true,
                Routed::Local => false,
                Routed::Failed => return Connected::Failed,
                Routed::Interrupted(next) => return Connected::Interrupted(next),
            };
            // Routed again on the selected agent: its launch would fail the same way.
            if on_main && let Some(e) = first.take() {
                self.connect_failed(n, &e, None);
                return Connected::Failed;
            }
            if self.link.agent_id() != known.agent {
                *known = self.known_session().await;
            }
            match self.link.connect().await {
                Ok(live) => {
                    if on_main {
                        voice.main_served();
                    }
                    return Connected::Live(live);
                }
                // The link remembered the failure: the next route takes it into account.
                Err(e) if on_main && e.unusable().is_some() && first.is_none() => {
                    tracing::info!(tab = %self.ctx.tab.tab(), error = %e.one_line(), "agent choisi inutilisable : tour réorienté");
                    first = Some(e);
                }
                Err(e) => {
                    let standing = if on_main { None } else { voice.standing_reason() };
                    self.connect_failed(n, &e, standing.as_ref());
                    return Connected::Failed;
                }
            }
        }
    }

    /// The connection failed: the error shows in turn `n`, an unusable agent says so to the tab.
    ///
    /// `standing`: the local agent failed, standing in for the selected one unusable for it.
    fn connect_failed(&self, n: u32, e: &AgentError, standing: Option<&Reason>) {
        self.failed(n, e);
        // The agent is unusable: an open import dialog says so without asking again, in the
        // same text as `Stack::availability` (both reasons when the local agent failed).
        if let Some(u) = e.unusable() {
            let l = self.ctx.localizer.current();
            let unusable = Some(match standing {
                Some(main) => both_reasons(main, &u, &l),
                None => u.localize(&l),
            });
            self.ctx.emit(AgentEvent::Availability { unusable });
        }
    }

    /// Where turn `n` runs, a download included; its notice, or its failure, shows in the turn.
    async fn take_route(&mut self, voice: &Arc<AppVoice>, rx: &mut mpsc::UnboundedReceiver<Command>, n: u32) -> Routed {
        let mut downloaded = false;
        loop {
            match self.route_of(voice).await {
                TurnRoute::Main => {
                    self.tell_fallback(None);
                    return Routed::Main;
                }
                TurnRoute::Local { reason } => {
                    self.notice(n, &reason);
                    self.tell_fallback(Some(&reason));
                    return Routed::Local;
                }
                TurnRoute::Unusable { main, local } => {
                    let text = both_reasons(&main, &local, &self.ctx.localizer.current());
                    // No agent can run: an open import dialog says so, as `Stack::availability`.
                    self.ctx.emit(AgentEvent::Availability { unusable: Some(text.clone()) });
                    self.fail_with(n, text);
                    return Routed::Failed;
                }
                // Downloaded, yet still missing: said as the selected agent's failure.
                TurnRoute::NeedsDownload { reason, .. } if downloaded => {
                    self.fail_with(n, reason.localize(&self.ctx.localizer.current()));
                    return Routed::Failed;
                }
                TurnRoute::NeedsDownload { reason, size } => match self.download(voice, rx, n, &reason, size).await {
                    Download::Done => downloaded = true,
                    // The local agent can still run once downloaded: the tab is not told it is unusable.
                    Download::Declined => {
                        self.fail_with(n, reason.localize(&self.ctx.localizer.current()));
                        return Routed::Failed;
                    }
                    Download::Failed(message) => {
                        self.fail_with(n, message);
                        return Routed::Failed;
                    }
                    Download::Interrupted(next) => {
                        self.interrupted(n);
                        return Routed::Interrupted(next);
                    }
                },
            }
        }
    }

    /// The route of this tab's turn, decided off the runtime threads (it reads the disk).
    async fn route_of(&self, voice: &Arc<AppVoice>) -> TurnRoute {
        let (voice, tab) = (voice.clone(), self.ctx.tab.tab().to_owned());
        match tokio::task::spawn_blocking(move || voice.turn_route(&tab)).await {
            Ok(route) => route,
            Err(e) => {
                tracing::warn!(error = %e, "route du tour perdue : agent choisi");
                TurnRoute::Main
            }
        }
    }

    /// Asks for the local model, then waits for its download, its progress shown in turn `n`.
    ///
    /// A download already running is waited for without asking. A command ends the wait; a
    /// cancel also stops the download.
    async fn download(
        &mut self,
        voice: &Arc<AppVoice>,
        rx: &mut mpsc::UnboundedReceiver<Command>,
        n: u32,
        reason: &Reason,
        size: u64,
    ) -> Download {
        let running = matches!(self.local_model(voice).await, Some(LocalModelView::Running { .. }));
        if !running {
            let l = self.ctx.localizer.current();
            let message = l.tr_with(
                "agent-fallback-download",
                &[
                    ("reason", reason.localize(&l).into()),
                    ("model", AGENT_MODEL.name.into()),
                    ("gigabytes", gigabytes_text(size, &l).into()),
                ],
            );
            let yes = tokio::select! {
                yes = self.ctx.hub.confirm(message, false) => yes,
                next = rx.recv() => {
                    self.ctx.hub.cancel_asks();
                    return Download::Interrupted(next);
                }
            };
            if !yes {
                return Download::Declined;
            }
            let starter = voice.clone();
            // False: it runs already, or nothing is missing any more; the state below tells.
            let _started = tokio::task::spawn_blocking(move || starter.start_fallback_download()).await;
        }
        let id = self.ctx.hub.next_id("note");
        let mut shown = None;
        let mut tick = tokio::time::interval(POLL);
        loop {
            tokio::select! {
                _ = tick.tick() => {}
                next = rx.recv() => {
                    if matches!(next, Some(Command::Cancel) | None) {
                        voice.cancel_local_download();
                    }
                    return Download::Interrupted(next);
                }
            }
            let l = self.ctx.localizer.current();
            match self.local_model(voice).await {
                None => return Download::Done,
                Some(LocalModelView::Running { percent }) => {
                    if shown != Some(percent) {
                        shown = Some(percent);
                        self.line(n, &id, &agent_download_progress(percent, &l));
                    }
                }
                Some(LocalModelView::Failed { failure, .. }) => {
                    return Download::Failed(local_failure_text(&failure, &l));
                }
                Some(LocalModelView::NeedsDownload { .. }) => {
                    tracing::warn!(tab = %self.ctx.tab.tab(), "téléchargement du modèle local non lancé");
                    return Download::Failed(l.tr("agent-model-download-failed"));
                }
            }
        }
    }

    /// The local model's state, read off the runtime threads (it reads the disk).
    async fn local_model(&self, voice: &Arc<AppVoice>) -> Option<LocalModelView> {
        let voice = voice.clone();
        tokio::task::spawn_blocking(move || voice.local_model()).await.unwrap_or_else(|e| {
            tracing::warn!(error = %e, "état du modèle local perdu");
            Some(LocalModelView::Failed { size: AGENT_MODEL.size, failure: oku_slides_session::LocalFailure::Failed })
        })
    }

    /// The notice of turn `n`: it runs on the local agent, and why the selected one does not.
    fn notice(&self, n: u32, reason: &Reason) {
        let l = self.ctx.localizer.current();
        let text = l.tr_with("agent-fallback-notice", &[("reason", reason.localize(&l).into())]);
        self.line(n, &self.ctx.hub.next_id("note"), &text);
    }

    /// Shows (or replaces) the line `id` of turn `n`.
    fn line(&self, n: u32, id: &str, text: &str) {
        let l = self.ctx.localizer.current();
        let Some(card) = receipt::fallback(id, text, &[], &[], &l) else {
            tracing::debug!("ligne du repli vide : non montrée");
            return;
        };
        if !self.ctx.hub.show_static(n, card) {
            tracing::debug!(turn = n, "ligne du repli refusée : tour déjà clos");
        }
    }

    /// Tells the tab whether its turns run on the local agent; nothing if that did not change.
    fn tell_fallback(&mut self, reason: Option<&Reason>) {
        if reason.is_none() && !self.on_local {
            return;
        }
        self.on_local = reason.is_some();
        let reason = reason.map(|r| r.localize(&self.ctx.localizer.current()));
        self.ctx.emit(AgentEvent::Fallback { reason });
    }

    /// Turn `n` fails with the localized `message`.
    fn fail_with(&self, n: u32, message: String) {
        tracing::warn!(tab = %self.ctx.tab.tab(), message, "agent : échec avant le lancement");
        self.ctx.hub.cancel_asks();
        self.ctx.emit(AgentEvent::Error { turn: n, message });
        self.ctx.emit(AgentEvent::idle());
    }

    /// Turn `n` ends before its agent started: a command came first.
    fn interrupted(&self, n: u32) {
        self.ctx.hub.cancel_asks();
        self.ctx.emit(AgentEvent::TurnEnded { turn: n, stop: "cancelled".to_owned() });
        self.ctx.emit(AgentEvent::idle());
    }
}
