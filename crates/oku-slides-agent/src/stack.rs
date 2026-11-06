//! La pile de l'agent pour un hôte : un agent par onglet (outils MCP, fil
//! gardé) sur la connexion ACP partagée, et le relais des évènements vers
//! l'interface, enveloppés `{tab, event}`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Duration;

use oku_slides_i18n::Localize;
use oku_slides_mcp::{
    AnswerOutcome, CloseDecision, HubError, McpEvent, Prefill, ShowOutcome, SurfaceAction, SurfaceHub,
};
use oku_slides_session::Platform;
use oku_slides_session::agent::{AgentUi, AgentWorkspace, AppPort, AttachmentRef, Sessions, TurnOrigin};
use oku_slides_stt::{Speaker, Stt};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::agent_command::{AgentCommand, MAX_DRAFT_BYTES, MAX_ORIGIN_BYTES};
use crate::choice::AgentChoice;
use crate::command_error::{CommandError, DraftTooLongSnafu, NoAgentSnafu, OriginTooLongSnafu};
use crate::drafts_port::{Starter, StarterParts};
use crate::keeper::{Keep, Keeper};
use crate::page_reports::{PageContext, PageReports, ReportFrom};
use crate::settings_port::AppSettings;
use crate::shortcut::is_spoken_theme_shortcut;
use crate::tab_agent::{Command, Instruction};
use crate::tab_entry::{Out, TabLink, emit};
use crate::tab_port::StackTabs;
use crate::table::Table;
use crate::voice_port::AppVoice;
use crate::{AgentEvent, Link};

/// Clé Fluent du texte du fil pour le fine-tuning d'un import.
const TUNE_PROMPT: &str = "agent-tune-prompt";

/// Attente de l'arrêt de l'agent ACP à la fin de l'app (`Stack::shutdown`).
const SHUTDOWN_WITHIN: Duration = Duration::from_secs(2);

/// The host's dictation and read-aloud voice, both optional, for [`Stack::start`].
#[derive(Debug, Clone, Default)]
pub struct Voices {
    /// The dictation: the settings box, the `settings` tool and the voice router.
    pub stt: Option<Stt>,
    /// The process-wide read-aloud voice; `None`: nothing is read aloud.
    pub speaker: Option<Speaker>,
}

/// Les agents des onglets d'un hôte, sur un seul agent ACP ; chaque onglet
/// démarre à sa première commande et s'arrête à sa fermeture (ou avec la pile).
///
/// Garde une copie de la [`Platform`] : comme elle, à lâcher hors d'un
/// contexte async si c'est sa dernière copie (`Storage` possède un runtime).
#[derive(Debug)]
pub struct Stack {
    link: Link,
    /// Starts the tabs' entries, and prefills and pages them for the tab port.
    starter: Arc<Starter>,
    pub(crate) platform: Platform,
    rt: Handle,
    table: Arc<Mutex<Table>>,
    out_tx: mpsc::UnboundedSender<Out>,
    out_rx: Mutex<Option<mpsc::UnboundedReceiver<Out>>>,
    /// L'agent choisi : celui du prochain lancement, sa disponibilité, le panneau.
    choice: Arc<AgentChoice>,
    /// La dictée et l'agent de l'app : la boîte des réglages de chaque onglet.
    pub(crate) voice: Arc<AppVoice>,
    /// The host's workspace: the tab that has the focus.
    pub(crate) workspace: Arc<dyn AgentWorkspace>,
    /// The app itself: updates, and quitting or installing once a turn has ended.
    pub(crate) app: Arc<dyn AppPort>,
    /// What each tab's page last reported: context, presenting.
    pub(crate) reports: PageReports,
    /// Set by [`Stack::shutdown`]: the voice router ends.
    closing: AtomicBool,
    /// The read-aloud voice: the router drops what the microphone heard of it.
    pub(crate) speaker: Option<Speaker>,
}

impl Stack {
    /// The agent stack; tabs, tools and agents start lazily.
    ///
    /// Nothing starts here: a tab's tools and guard start at its first command,
    /// `resync` included; the ACP agent at the first prompt; all in `rt`. `stt`: the host's
    /// dictation, for the settings box and the `settings` tool. Only the dictation watch and
    /// the box's follow-up start here,
    /// with the voice router over the dictation's utterances, and the after-turn thread.
    /// `workspace` tells the focused tab; `app` answers updates and quits after a turn.
    /// `speaker`, the process's read-aloud voice, starts the reader of the focused tab.
    pub fn start(
        sessions: Arc<dyn Sessions>,
        platform: &Platform,
        voices: Voices,
        workspace: Arc<dyn AgentWorkspace>,
        app: Arc<dyn AppPort>,
        rt: Handle,
    ) -> Arc<Self> {
        let Voices { stt, speaker } = voices;
        let a = platform.settings.agent.active();
        let model = a.builtin.is_none().then(|| a.model.clone()).flatten();
        let launch = AgentChoice::launch_of(a, platform);
        let link = Link::for_preset(platform.settings.agent.selected(), launch, model, platform.storage.clone());
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        let choice = Arc::new(AgentChoice::new(platform, link.clone()));
        let voice = AppVoice::new(stt.clone(), speaker.clone(), choice.clone(), rt.clone());
        // The reader's thread ends once the last tab relay is gone: never joined.
        let reader = speaker
            .clone()
            .map(|s| crate::reader::spawn(crate::reader::Reader::new(s, stt.clone(), workspace.clone())).0);
        let table = Arc::new(Mutex::new(Table::default()));
        let utterances = stt.as_ref().map(Stt::utterances);
        let (after_tx, after_rx) = std::sync::mpsc::channel();
        crate::voice_refresh::spawn(&rt, Arc::downgrade(&voice), Arc::downgrade(&table), out_tx.clone());
        // The tabs' port reaches back to the stack weakly: the stack's table holds the tabs.
        let stack = Arc::new_cyclic(|this: &Weak<Self>| {
            let starter = Starter::new(StarterParts {
                sessions,
                platform: platform.clone(),
                settings: Arc::new(AppSettings::new(platform, stt.clone()).with_speaker(speaker.clone())),
                voice: voice.clone(),
                workspace: workspace.clone(),
                tabs: Arc::new(StackTabs { stack: this.clone() }),
                app: app.clone(),
                after: after_tx,
                link: link.clone(),
                rt: rt.clone(),
                out_tx: out_tx.clone(),
                table: table.clone(),
                reader,
            });
            Self {
                speaker,
                choice,
                voice,
                workspace,
                app,
                reports: PageReports::default(),
                closing: AtomicBool::new(false),
                link,
                starter,
                platform: platform.clone(),
                rt,
                table,
                out_tx,
                out_rx: Mutex::new(Some(out_rx)),
            }
        });
        stack.run_after_turns(after_rx);
        match utterances {
            // The thread ends by itself once the stack closes: never joined.
            Some(Ok(rx)) => drop(stack.route_utterances(rx)),
            Some(Err(e)) => tracing::warn!(error = %snafu::Report::from_error(&e), "voice router without utterances"),
            None => {}
        }
        stack
    }

    /// Handles an interface command; the interface reads the error.
    ///
    /// A page report ([`AgentCommand::is_page_report`]) is refused: it goes through
    /// [`Stack::handle_from`], with its sender checked. May block (a click in the settings box may
    /// write `config.toml`, the box reads the model's state): call it off the runtime threads; a
    /// "theme" shortcut renders up to 12 thumbnails, 4 s at most.
    ///
    /// # Errors
    ///
    /// Returns the localized message when the command is refused: rejected instruction, draft or
    /// pasted image, refused surface action, no started agent, or packs panel failure.
    pub fn handle(&self, cmd: AgentCommand) -> Result<(), String> {
        self.handle_from(cmd, &ReportFrom::Other)
    }

    /// Handles an interface command sent by `from`, as the host checked it.
    ///
    /// Only page reports read `from`; the rest is as [`Stack::handle`].
    ///
    /// # Errors
    ///
    /// As [`Stack::handle`]; a page report from another window, over its limits or with the
    /// last number is refused too.
    pub fn handle_from(&self, cmd: AgentCommand, from: &ReportFrom) -> Result<(), String> {
        if self.is_closed(cmd.tab()) {
            tracing::debug!(tab = cmd.tab(), "commande pour un onglet fermé : ignorée");
            return Ok(());
        }
        match cmd {
            AgentCommand::Prompt { tab, text, ui, answer, images } => {
                if let Some((t, images)) = self.route_prompt(&tab, answer, &text, ui.slide.as_deref(), &images)? {
                    t.send(Command::Prompt(Instruction { text, ui, tune: false, images, origin: TurnOrigin::Typed }));
                }
            }
            AgentCommand::Tune { tab } => {
                let Some(t) = self.open(&tab) else { return Ok(()) };
                let text = self.platform.live_localizer.current().tr(TUNE_PROMPT);
                let ui = AgentUi::default();
                t.send(Command::Prompt(Instruction {
                    text,
                    ui,
                    tune: true,
                    images: Vec::new(),
                    origin: TurnOrigin::Typed,
                }));
            }
            AgentCommand::Cancel { tab } => self.cancel(&tab),
            // Un onglet sans agent n'a aucune surface de tour à fermer : rien à faire pour lui.
            AgentCommand::NewConversation { tab } => {
                if let Some(t) = self.tab(&tab) {
                    t.send(Command::NewConversation);
                }
            }
            AgentCommand::SurfaceAction { tab, surface, name, context } => {
                // Un onglet sans agent n'a aucune surface ouverte. L'interface affiche le texte
                // reçu : dit dans la langue de l'utilisateur, l'erreur complète reste au journal.
                let acted = match self.tab(&tab) {
                    Some(t) => t.hub.act(SurfaceAction { surface, name, context }),
                    None => Err(HubError::unknown_surface(surface)),
                };
                return acted.map_err(|e| {
                    tracing::info!(tab, error = %snafu::Report::from_error(&e), "action de surface refusée");
                    e.localize(&self.platform.live_localizer.current())
                });
            }
            AgentCommand::ShowPacks { tab } => return self.show_packs(&tab),
            AgentCommand::ShowThemes { tab, slide } => return self.show_themes(&tab, slide.as_deref()),
            // Starts the tab (tools, keeper; not the ACP agent): a reopened tab gets its thread back.
            // The agent's state first, then the thread (shown by the keeper, after its replay).
            AgentCommand::Resync { tab } => {
                if let Some(t) = self.open(&tab) {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Show);
                } else {
                    // Closed in the meantime: nothing to show.
                    self.replay_closed(&tab);
                }
            }
            AgentCommand::Follow { tab } => {
                // The lookup and the empty replay share one table lock: a prefill inserts its entry
                // under it, so its draft event cannot land between them and be overwritten.
                let link = {
                    let table = self.table();
                    let link = table.tabs.get(&tab).map(|e| e.link.clone());
                    if link.is_none() {
                        self.replay_closed(&tab);
                    }
                    link
                };
                if let Some(t) = link {
                    t.replay(&self.out_tx);
                    t.keeper.send(Keep::Show);
                }
            }
            AgentCommand::SetDraft { tab, text, origin } => return self.set_draft(&tab, text, origin),
            // Démarre l'onglet (outils, gardien ; pas l'agent ACP) : la boîte va au hub de l'onglet.
            AgentCommand::ShowSettings { tab } => {
                let Some(t) = self.open(&tab) else { return Ok(()) };
                // Opened by the user: no turn, so the box cannot leave the thread.
                t.hub.show_settings(None);
            }
            AgentCommand::PasteImage { tab, data } => {
                let l = self.platform.live_localizer.current();
                let Some(t) = self.tab(&tab) else {
                    return Err(CommandError::from(NoAgentSnafu.build()).localize(&l));
                };
                return crate::paste::paste_image(&t, &self.out_tx, &self.platform.settings.paste, &data).map_err(
                    |e| {
                        tracing::info!(tab, error = %snafu::Report::from_error(&e), "image collée refusée");
                        e.localize(&l)
                    },
                );
            }
            AgentCommand::RemoveImage { tab, id } => {
                if let Some(t) = self.tab(&tab) {
                    t.remove_image(&self.out_tx, &id);
                } else {
                    let e = CommandError::from(NoAgentSnafu.build());
                    return Err(e.localize(&self.platform.live_localizer.current()));
                }
            }
            AgentCommand::Availability { tab } => {
                emit(&self.out_tx, "agent", &tab, &AgentEvent::Availability { unusable: self.availability() });
            }
            AgentCommand::Context { tab, page, ui, seq, has_focus } => {
                self.context_report(&tab, from, &page, seq, PageContext { ui, has_focus })?;
            }
            AgentCommand::Presenting { tab, page, mode, seq } => {
                self.presenting_report(&tab, from, &page, seq, mode)?;
            }
        }
        Ok(())
    }

    /// Cancels `tab`'s turn, releases its questions and drops its after-turn actions; only that tab.
    pub(crate) fn cancel(&self, tab: &str) {
        if let Some(t) = self.tab(tab) {
            t.hub.drop_after_turn();
            t.hub.cancel_asks();
            t.send(Command::Cancel);
        }
    }

    /// True once [`Stack::shutdown`] began.
    pub(crate) fn is_closing(&self) -> bool {
        self.closing.load(Ordering::SeqCst)
    }

    /// How turn `turn` of `tab` began, while it is that tab's last turn; for the read-aloud.
    #[must_use]
    pub fn turn_origin(&self, tab: &str, turn: u32) -> Option<TurnOrigin> {
        self.tab(tab).and_then(|t| t.hub.turn_origin(turn))
    }

    /// Routes an instruction: answer, theme shortcut, or the agent with its images.
    ///
    /// Returns the tab and resolved images of an instruction that goes to the agent; `None` when
    /// nothing goes to the agent: tab closed, question answered or kept open (a guard or a
    /// permission typed to), or theme picker shown.
    pub(crate) fn route_prompt(
        &self,
        tab: &str,
        answer: Option<String>,
        text: &str,
        slide: Option<&str>,
        images: &[String],
    ) -> Result<Option<(TabLink, Vec<AttachmentRef>)>, String> {
        // Closed in the meantime: ignored, as in `handle`.
        let Some(t) = self.open(tab) else { return Ok(None) };
        // An answer to the designated question, never to another one. If it no longer waits
        // (answered or cancelled in the meantime), the instruction is not lost: it goes like any
        // instruction (a new turn; during a turn, it interrupts then replaces it).
        // A guarded confirmation or a permission keeps it open ("say it or click"): no prompt,
        // and the running turn goes on.
        if images.is_empty()
            && let Some(id) = answer
        {
            match t.hub.answer(&id, text) {
                AnswerOutcome::Answered | AnswerOutcome::KeptOpen => return Ok(None),
                AnswerOutcome::NotAQuestion => {}
            }
        }
        // "Theme" shortcut, in the interface or the dictation language: the host's picker, with
        // no turn nor agent; a running turn goes on.
        // Outside deck mode, or if it fails (logged), the instruction goes to the agent.
        let ui = self.platform.live_localizer.current();
        if images.is_empty()
            && is_spoken_theme_shortcut(text, &ui, || self.voice.dictation_locale())
            && show_theme_shortcut(&t, tab, slide)
        {
            return Ok(None);
        }
        let images =
            crate::paste::resolve_images(&t, images, self.platform.settings.paste.max_draft_images).map_err(|e| {
                tracing::info!(tab, error = %snafu::Report::from_error(&e), "consigne refusée");
                e.localize(&self.platform.live_localizer.current())
            })?;
        Ok(Some((t, images)))
    }

    /// Sets the tab's draft after checking its sizes; the error is localized.
    fn set_draft(&self, tab: &str, text: Option<String>, origin: Option<String>) -> Result<(), String> {
        let checked = match (text.as_ref().map(String::len), origin.as_ref().map(String::len)) {
            (Some(bytes), _) if bytes > MAX_DRAFT_BYTES => {
                Err(DraftTooLongSnafu { bytes, max: MAX_DRAFT_BYTES }.build())
            }
            (_, Some(bytes)) if bytes > MAX_ORIGIN_BYTES => {
                Err(OriginTooLongSnafu { bytes, max: MAX_ORIGIN_BYTES }.build())
            }
            _ => Ok(()),
        };
        if let Err(inner) = checked {
            let e = CommandError::from(inner);
            tracing::info!(tab, error = %snafu::Report::from_error(&e), "brouillon refusé");
            return Err(e.localize(&self.platform.live_localizer.current()));
        }
        // Refused, not ignored: the writing view waits for an echo or an error (t-654).
        let Some(t) = self.tab(tab) else {
            let e = CommandError::from(NoAgentSnafu.build());
            tracing::debug!(tab, error = %snafu::Report::from_error(&e), "brouillon refusé");
            return Err(e.localize(&self.platform.live_localizer.current()));
        };
        t.set_draft(&self.out_tx, text, origin);
        Ok(())
    }

    /// Shows the deck's packs panel in the tab; the error is localized.
    ///
    /// Starts the tab (tools, keeper; not the ACP agent): the panel goes to the tab's hub.
    fn show_packs(&self, tab: &str) -> Result<(), String> {
        let Some(t) = self.open(tab) else { return Ok(()) };
        match t.hub.show_packs(None) {
            Ok(ShowOutcome::Shown) => Ok(()),
            // The button exists only in deck mode: a mode change got ahead of it. No turn given,
            // so `TurnOver` cannot come.
            Ok(ShowOutcome::OutOfMode | ShowOutcome::TurnOver) => {
                tracing::debug!(tab, "panneau des packs hors du mode deck : ignoré");
                Ok(())
            }
            Err(e) => {
                tracing::info!(tab, error = %snafu::Report::from_error(&e), "panneau des packs impossible");
                Err(e.localize(&self.platform.live_localizer.current()))
            }
        }
    }

    /// Shows the theme picker on `slide` in the tab; the error is localized.
    ///
    /// Starts the tab (tools, keeper; not the ACP agent): the picker goes to the tab's hub.
    fn show_themes(&self, tab: &str, slide: Option<&str>) -> Result<(), String> {
        let Some(t) = self.open(tab) else { return Ok(()) };
        match t.hub.show_themes(slide, None) {
            Ok(ShowOutcome::Shown) => Ok(()),
            // The button exists only in deck mode: a mode change got ahead of it. No turn given,
            // so `TurnOver` cannot come.
            Ok(ShowOutcome::OutOfMode | ShowOutcome::TurnOver) => {
                tracing::debug!(tab, "sélecteur de thème hors du mode deck : ignoré");
                Ok(())
            }
            Err(e) => {
                tracing::info!(tab, error = %snafu::Report::from_error(&e), "sélecteur de thème impossible");
                Err(e.localize(&self.platform.live_localizer.current()))
            }
        }
    }

    /// Starts the fine-tuning turn of the imported tab `tab` (see [`AgentCommand::Tune`]); `true` if it started.
    ///
    /// For the host, when an import asks for its fine-tuning; starts the tab's agent. Refused
    /// only if neither the selected agent nor the local one can run (a model download included):
    /// nothing starts, and the tab gets the reason (`AgentEvent::Availability`), as after a failed
    /// launch; a refused command is logged. May block (reads the disk): call it off the runtime threads.
    pub fn tune(&self, tab: &str) -> bool {
        if let Some(reason) = self.availability() {
            tracing::warn!(tab, reason, "import fine-tuning not started: agent unusable");
            emit(&self.out_tx, "agent", tab, &AgentEvent::Availability { unusable: Some(reason) });
            return false;
        }
        match self.handle(AgentCommand::Tune { tab: tab.to_owned() }) {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!(tab, error = %e, "import fine-tuning not started");
                false
            }
        }
    }

    /// Why no agent can serve a turn, localized; `None` if one seems usable.
    ///
    /// The selected agent's reason (its program not found, the local engine's state for the
    /// built-in agent, else the recent failure of its last launch), and the local agent's when
    /// it cannot stand in for it either (a model to download counts as able). May block: reads
    /// the disk.
    pub fn availability(&self) -> Option<String> {
        self.voice.availability()
    }

    /// Takes close decision `id` once, for a page of `window`, from the hub of the tab holding it.
    ///
    /// `None` when no tab holds it, or when it is pending, expired or another window's.
    #[must_use]
    pub fn take_close_decision(&self, window: &str, id: &str) -> Option<CloseDecision> {
        let hubs: Vec<Arc<SurfaceHub>> = self.table().tabs.values().map(|e| e.link.hub.clone()).collect();
        hubs.iter().find_map(|hub| hub.take_close_decision(window, id))
    }

    /// Le deck de l'onglet a changé depuis l'interface (voir
    /// `Request::changes_deck`) : fermer les surfaces orphelines.
    pub fn after_change(&self, tab: &str) {
        if let Some(t) = self.tab(tab) {
            t.hub.prune();
        }
    }

    /// The tab is closed: its agent stops (turn cancelled, questions released), its tools too.
    pub fn tab_closed(&self, tab: &str) {
        self.voice.tab_closed(tab);
        self.reports.forget_tab(tab);
        if let Some(t) = self.tab(tab) {
            self.copy_presenting(&t.hub);
        }
        let gone = {
            let mut table = self.table();
            table.closed.insert(tab.to_owned());
            table.tabs.remove(tab)
        };
        // Lâché hors du verrou : l'arrêt d'un onglet ne bloque pas les autres.
        drop(gone);
    }

    /// Fin de l'app : chaque onglet s'arrête, les conversations en file sont
    /// écrites, puis l'agent ACP est tué (chacun attendu au plus 2 s) ; toute
    /// commande ensuite est ignorée.
    pub async fn shutdown(&self) {
        // The voice router ends at its next poll: no utterance is routed after this.
        self.closing.store(true, Ordering::SeqCst);
        // Plus aucun changement d'agent : un choix en attente tombe.
        self.voice.shutdown();
        // Le `.part` du modèle (2,5 Go) ne doit pas grossir après la fin de l'app.
        self.choice.cancel_download();
        let gone = {
            let mut table = self.table();
            table.shut = true;
            std::mem::take(&mut table.tabs)
        };
        let keepers: Vec<Keeper> = gone.values().map(|e| e.link.keeper.clone()).collect();
        drop(gone);
        // Les fins de tour encore en file sont écrites avant l'arrêt (bornées, comme l'agent).
        let flushed = async {
            for keeper in &keepers {
                keeper.flush().await;
            }
        };
        if tokio::time::timeout(SHUTDOWN_WITHIN, flushed).await.is_err() {
            tracing::warn!("conversations de l'agent pas toutes écrites à temps");
        }
        self.link.shutdown(SHUTDOWN_WITHIN).await;
    }

    /// Ce que voit une vue d'un onglet sans agent démarré.
    ///
    /// Repos, aucun brouillon, aucune surface.
    fn replay_closed(&self, tab: &str) {
        emit(&self.out_tx, "agent", tab, &AgentEvent::idle());
        emit(&self.out_tx, "agent", tab, &AgentEvent::Draft { text: None, origin: None, images: Vec::new() });
        emit(&self.out_tx, "mcp", tab, &McpEvent::Surfaces { list: Vec::new() });
    }

    fn is_closed(&self, tab: &str) -> bool {
        self.table().refuses(tab)
    }

    /// Relaie les évènements de tous les onglets vers `sink` : canal
    /// (`"mcp"`, `"agent"`) et JSON `{"tab": …, "event": …}`. Une seule fois.
    pub fn forward(&self, sink: impl Fn(&'static str, String) + Send + Sync + 'static) {
        let taken = self.out_rx.lock().unwrap_or_else(PoisonError::into_inner).take();
        let Some(mut rx) = taken else {
            tracing::warn!("forward déjà appelé : ignoré");
            return;
        };
        self.rt.spawn(async move {
            while let Some((channel, json)) = rx.recv().await {
                sink(channel, json);
            }
        });
    }

    fn table(&self) -> std::sync::MutexGuard<'_, Table> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Prefills `tab`'s draft with `text` (see [`Starter::prefill`]); for the host's tab port.
    pub(crate) fn prefill(&self, tab: &str, text: String) -> Prefill {
        self.starter.prefill(tab, text)
    }

    /// Shows `slide` on `tab`'s pages (see [`Starter::page`]); for the host's tab port.
    pub(crate) fn page(&self, tab: &str, slide: String) -> bool {
        self.starter.page(tab, slide)
    }

    /// The tab's link if it already has an agent; the lock is not held.
    pub(crate) fn tab(&self, tab: &str) -> Option<TabLink> {
        self.table().tabs.get(tab).map(|e| e.link.clone())
    }

    /// The tab, with its entry started if it has none; `None` if closed (or the app over).
    ///
    /// Checked under the lock the entry would be inserted under.
    pub(crate) fn open(&self, tab: &str) -> Option<TabLink> {
        self.starter.open(tab)
    }
}

/// Shows the theme picker for a shortcut word; false out of deck mode or on failure (logged).
pub(crate) fn show_theme_shortcut(t: &TabLink, tab: &str, slide: Option<&str>) -> bool {
    match t.hub.show_themes(slide, None) {
        Ok(ShowOutcome::Shown) => true,
        // No turn given: only the mode can refuse it.
        Ok(ShowOutcome::OutOfMode | ShowOutcome::TurnOver) => false,
        Err(e) => {
            tracing::warn!(tab, error = %snafu::Report::from_error(&e), "theme picker not shown");
            false
        }
    }
}
