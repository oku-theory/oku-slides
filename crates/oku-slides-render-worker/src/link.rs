//! Lien avec un travailleur : transport, bornes, relance et rejeu.
//!
//! Deux transports : un sous-processus (`RenderLaunch::Process`), borné en
//! temps et en mémoire et tué au moindre échec ; ou un fil du processus
//! (`RenderLaunch::InProcess`), borné en temps seulement.
//!
//! Limite de `InProcess` : un fil ne se tue pas. Un délai dépassé l'abandonne ;
//! il finit seul quand son calcul se termine, écrit dans un tube fermé et
//! sort. Un rendu qui boucle sans fin y garde donc un cœur occupé jusqu'à la
//! fin du processus : ce transport sert aux tests et aux outils.
//!
//! Après tout échec du lien, la requête suivante lance un travailleur neuf et
//! lui rejoue [`ReplayState`] : langue, polices, emplacements et leurs packs
//! (sauf les packs fautifs, remplacés par la base).
//!
//! Chaque connexion commence par `Hello` : un travailleur d'une autre version
//! du protocole (autre binaire) est refusé (`Protocol`) et arrêté.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use oku_slides_platform::config::RenderSettings;
use oku_slides_platform::{RENDER_WORKER_ENV, RenderLaunch};
use oku_slides_render::RenderError;
use sha2::{Digest, Sha256};

use crate::error::FrameError;
use crate::frame::{Frame, FrameLimits, read_frame_with, write_frame};
use crate::protocol::{PROTOCOL_VERSION, Request, Response, SlotId, WirePack};
use crate::rss::memory_bytes;

/// Intervalle du relevé de mémoire et du contrôle du délai.
const TICK: Duration = Duration::from_millis(100);
/// Attente du travailleur à la fermeture, avant `kill`.
const GRACE: Duration = Duration::from_millis(500);
/// Intervalle des `try_wait` pendant [`GRACE`].
const GRACE_POLL: Duration = Duration::from_millis(20);

/// Empreinte hexadécimale d'un pack, égale à celle de `PackWidgets::fingerprint`.
pub(crate) fn fingerprint_hex(pack: &WirePack) -> String {
    let mut h = Sha256::new();
    h.update(pack.id.as_bytes());
    h.update([0]);
    for (name, source) in &pack.files {
        h.update(name.as_bytes());
        h.update([0]);
        h.update(source.as_bytes());
        h.update([0]);
    }
    hex::encode(h.finalize())
}

/// Erreur du lien pour une trame de sortie illisible, réponse à `req`.
///
/// Un tube rompu ou une trame tronquée : le travailleur est mort en écrivant
/// (`WorkerLost`). Une trame trop grande ou un en-tête non JSON : `Frame` ;
/// le SVG d'une page au-delà de sa borne le dit (`page_too_large`).
fn frame_failure(e: FrameError, req: &Request) -> RenderError {
    match e {
        FrameError::Io { .. } | FrameError::Truncated { .. } => RenderError::worker_lost(e.to_string()),
        FrameError::TooLarge { payload: true, max, .. } if matches!(req, Request::Svg { .. }) => {
            RenderError::page_too_large(u64::try_from(max / (1024 * 1024)).unwrap_or(u64::MAX))
        }
        FrameError::TooLarge { .. } | FrameError::Json { .. } => RenderError::frame(e.to_string()),
    }
}

/// Le statut de sortie d'un fils dont la sortie s'est fermée, attendu au plus
/// [`GRACE`] : le tube se ferme juste avant la fin du processus. `None` s'il
/// vit encore (il a fermé sa sortie sans sortir).
fn exit_status(child: &mut Child) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(GRACE_POLL),
            Ok(None) | Err(_) => return None,
        }
    }
}

/// La réponse à `Hello` dit la version du protocole de l'hôte ; sinon `Protocol`.
///
/// Un travailleur plus ancien que `Hello` y répond par une erreur : version inconnue.
pub(crate) fn check_hello(reply: &Response) -> Result<(), RenderError> {
    match reply {
        Response::Hello { version } if *version == PROTOCOL_VERSION => Ok(()),
        Response::Hello { version } => Err(RenderError::protocol(PROTOCOL_VERSION, Some(*version))),
        _ => Err(RenderError::protocol(PROTOCOL_VERSION, None)),
    }
}

/// La commande d'un travailleur `program` : ses bornes (le chien de garde
/// du fils les applique si l'hôte disparaît), ses tubes, et
/// [`RENDER_WORKER_ENV`] : le fils ne lance jamais de travailleur à son tour.
pub(crate) fn worker_command(program: &Path, settings: &RenderSettings) -> Command {
    let mut cmd = Command::new(program);
    cmd.args(["--render-worker", "--max-frame-bytes", &settings.max_frame_bytes().to_string()])
        .args(["--timeout-ms", &settings.timeout_ms.to_string()])
        .args(["--memory-bytes", &settings.memory_bytes().to_string()])
        .env(RENDER_WORKER_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    cmd
}

/// Tube d'entrée, flux de sortie et fils éventuel d'un travailleur lancé.
type Pipes = (Box<dyn Write + Send>, Box<dyn Read + Send>, Option<Child>);

/// Une connexion ouverte : tube d'entrée, trames de sortie, fils éventuel.
struct Conn {
    input: Box<dyn Write + Send>,
    output: Receiver<Result<Frame, FrameError>>,
    /// Bornes de la réponse attendue (selon la requête envoyée), lues par le
    /// fil lecteur au premier octet de chaque trame.
    expected: Arc<Mutex<FrameLimits>>,
    child: Option<Child>,
}

impl Drop for Conn {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Ce que le travailleur sait d'un emplacement.
#[derive(Debug, Clone, Default)]
pub(crate) struct SlotState {
    pub(crate) fonts: Vec<String>,
    pub(crate) pack: Option<WirePack>,
    pub(crate) last_assets: Vec<String>,
    /// Derniers jetons de thème posés ; rejoués après une relance.
    pub(crate) theme: Option<serde_json::Map<String, serde_json::Value>>,
}

/// Ce qu'un travailleur neuf doit recevoir pour reprendre là où l'ancien était.
#[derive(Debug, Default)]
pub(crate) struct ReplayState {
    pub(crate) locale: Option<String>,
    pub(crate) fonts: HashMap<String, Arc<[u8]>>,
    pub(crate) slots: BTreeMap<SlotId, SlotState>,
}

/// Le lien de l'hôte avec son travailleur.
pub(crate) struct Link {
    launch: RenderLaunch,
    settings: RenderSettings,
    conn: Option<Conn>,
    sys: sysinfo::System,
    pub(crate) state: ReplayState,
    pub(crate) sent_fonts: HashSet<String>,
    pub(crate) sent_assets: HashSet<String>,
    /// Empreintes (hex) des packs fautifs : rejoués avec la base.
    pub(crate) faulty_packs: HashSet<String>,
    pub(crate) restarts: u64,
    started: bool,
}

impl Link {
    /// Un lien sans travailleur : le premier appel le lance.
    pub(crate) fn new(launch: RenderLaunch, settings: RenderSettings) -> Self {
        Self {
            launch,
            settings,
            conn: None,
            sys: sysinfo::System::new(),
            state: ReplayState::default(),
            sent_fonts: HashSet::new(),
            sent_assets: HashSet::new(),
            faulty_packs: HashSet::new(),
            restarts: 0,
            started: false,
        }
    }

    /// Envoie une requête et attend sa réponse, dans les bornes.
    ///
    /// Relance le travailleur au besoin (rejeu de `state`). Un échec borné tue
    /// le travailleur ; une réponse `Error` n'est pas un échec du lien.
    pub(crate) fn call(&mut self, req: &Request, payload: &[u8]) -> Result<(Response, Vec<u8>), RenderError> {
        self.ensure_connected()?;
        self.raw_call(req, payload)
    }

    /// Oublie la connexion d'un travailleur mort au repos (tué par l'OS, son
    /// chien de garde) : la requête suivante en relance un sans échouer, et
    /// n'accuse donc aucun pack (t-340). Seul un sous-processus se vérifie.
    pub(crate) fn drop_if_dead(&mut self) {
        let dead = self
            .conn
            .as_mut()
            .and_then(|c| c.child.as_mut())
            .is_some_and(|child| !matches!(child.try_wait(), Ok(None)));
        if dead {
            tracing::warn!("processus de rendu arrêté au repos : relancé à la prochaine requête");
            self.conn = None;
        }
    }

    /// Un travailleur a déjà été lancé, même mort depuis.
    pub(crate) fn started(&self) -> bool {
        self.started
    }

    /// Une connexion est ouverte (le travailleur peut être mort sans le savoir).
    pub(crate) fn connected(&self) -> bool {
        self.conn.is_some()
    }

    /// Lance et rejoue un travailleur neuf s'il n'y a pas de connexion.
    ///
    /// À appeler avant de lire `sent_fonts` ou `sent_assets` : la relance les
    /// vide, et ils ne disent vrai que du travailleur connecté.
    pub(crate) fn ensure_connected(&mut self) -> Result<(), RenderError> {
        self.drop_if_dead();
        if self.conn.is_none() {
            self.connect()?;
        }
        Ok(())
    }

    fn connect(&mut self) -> Result<(), RenderError> {
        let max = self.settings.max_frame_bytes();
        let (input, output, child) = self.spawn(max)?;
        let (tx, rx) = channel();
        let mut output = output;
        let expected = Arc::new(Mutex::new(FrameLimits::uniform(0)));
        let limits = expected.clone();
        std::thread::Builder::new()
            .name("oku-render-reader".into())
            .spawn(move || {
                loop {
                    let limits = || *limits.lock().unwrap_or_else(PoisonError::into_inner);
                    match read_frame_with(&mut output, limits) {
                        Ok(Some(frame)) => {
                            if tx.send(Ok(frame)).is_err() {
                                return;
                            }
                        }
                        Ok(None) => return,
                        Err(e) => {
                            let _ = tx.send(Err(e));
                            return;
                        }
                    }
                }
            })
            .map_err(|e| RenderError::worker_lost(e.to_string()))?;
        self.conn = Some(Conn { input, output: rx, expected, child });
        if self.started {
            self.restarts += 1;
        }
        self.started = true;
        self.sent_fonts.clear();
        self.sent_assets.clear();
        let ready = self.hello().and_then(|()| self.replay());
        if let Err(e) = &ready {
            // Un travailleur d'un autre protocole, ou à moitié rejoué : abandonné.
            tracing::warn!(error = %e, "processus de rendu non prêt");
            self.conn = None;
        }
        ready
    }

    /// Poignée de main : le travailleur doit parler la version du protocole de l'hôte.
    fn hello(&mut self) -> Result<(), RenderError> {
        let (reply, _) = self.raw_call(&Request::Hello { version: PROTOCOL_VERSION }, &[])?;
        check_hello(&reply)
    }

    /// Lance le travailleur selon `launch` : tube d'entrée, flux de sortie, fils.
    fn spawn(&self, max: usize) -> Result<Pipes, RenderError> {
        match &self.launch {
            RenderLaunch::Process { program } => {
                let mut child = worker_command(program, &self.settings)
                    .spawn()
                    .map_err(|e| RenderError::worker_lost(format!("spawn {}: {e}", program.display())))?;
                let stdin = child.stdin.take().ok_or_else(|| RenderError::worker_lost("no stdin"))?;
                let stdout = child.stdout.take().ok_or_else(|| RenderError::worker_lost("no stdout"))?;
                Ok((Box::new(stdin), Box::new(stdout), Some(child)))
            }
            RenderLaunch::InProcess => {
                let (req_r, req_w) = std::io::pipe().map_err(|e| RenderError::worker_lost(e.to_string()))?;
                let (resp_r, resp_w) = std::io::pipe().map_err(|e| RenderError::worker_lost(e.to_string()))?;
                std::thread::Builder::new()
                    .name("oku-render-worker".into())
                    .spawn(move || {
                        let _ = crate::serve(req_r, resp_w, max);
                    })
                    .map_err(|e| RenderError::worker_lost(e.to_string()))?;
                Ok((Box::new(req_w), Box::new(resp_r), None))
            }
        }
    }

    /// Rejoue langue, polices et emplacements ; un pack fautif est rejoué par la base.
    fn replay(&mut self) -> Result<(), RenderError> {
        if let Some(locale) = self.state.locale.clone() {
            self.expect_ok(&Request::SetLanguage { locale }, &[])?;
        }
        let slots: Vec<(SlotId, SlotState)> = self.state.slots.iter().map(|(k, v)| (*k, v.clone())).collect();
        for (slot, st) in slots {
            for id in &st.fonts {
                if self.sent_fonts.insert(id.clone()) {
                    let bytes = self
                        .state
                        .fonts
                        .get(id)
                        .cloned()
                        .ok_or_else(|| RenderError::frame(format!("font {id} missing from replay state")))?;
                    self.expect_ok(&Request::PutFont { id: id.clone() }, &bytes)?;
                }
            }
            self.expect_ok(&Request::OpenSlot { slot, fonts: st.fonts.clone() }, &[])?;
            let pack = st.pack.filter(|p| !self.faulty_packs.contains(&fingerprint_hex(p)));
            if pack.is_some() {
                self.expect_ok(&Request::SetWidgets { slot, pack }, &[])?;
            }
            if st.theme.is_some() {
                self.expect_ok(&Request::SetTheme { slot, tokens: st.theme }, &[])?;
            }
        }
        Ok(())
    }

    fn expect_ok(&mut self, req: &Request, payload: &[u8]) -> Result<(), RenderError> {
        match self.raw_call(req, payload)? {
            (Response::Ok, _) => Ok(()),
            (Response::Error { error }, _) => Err(error.into_render()),
            (other, _) => Err(RenderError::frame(format!("unexpected reply {other:?}"))),
        }
    }

    /// Un échange sur la connexion courante ; tout échec l'abandonne.
    ///
    /// Sauf une trame refusée avant l'envoi (trop grande) : rien n'est écrit,
    /// la connexion reste, et l'erreur est `Frame` (non bornée).
    fn raw_call(&mut self, req: &Request, payload: &[u8]) -> Result<(Response, Vec<u8>), RenderError> {
        let max = self.settings.max_frame_bytes();
        let conn = self.conn.as_mut().ok_or_else(|| RenderError::worker_lost("not connected"))?;
        // Avant l'envoi : la réponse ne peut pas arriver avant.
        *conn.expected.lock().unwrap_or_else(PoisonError::into_inner) = FrameLimits::reply_to(req, max);
        match write_frame(&mut conn.input, req, payload, FrameLimits::requests(max)) {
            Ok(()) => {}
            Err(e @ (FrameError::TooLarge { .. } | FrameError::Json { .. })) => {
                return Err(RenderError::frame(e.to_string()));
            }
            Err(e) => {
                self.conn = None;
                return Err(RenderError::worker_lost(e.to_string()));
            }
        }
        let result = self.reply(req);
        if result.is_err() {
            // Tout échec du lien le rend inutilisable : `Conn::drop` tue le fils,
            // et le suivant est neuf.
            self.conn = None;
        }
        result
    }

    /// Attend la réponse à `req`, envoyée, dans les bornes.
    fn reply(&mut self, req: &Request) -> Result<(Response, Vec<u8>), RenderError> {
        let (timeout, memory) = (self.settings.timeout(), self.settings.memory_bytes());
        let conn = self.conn.as_mut().ok_or_else(|| RenderError::worker_lost("not connected"))?;
        let started = Instant::now();
        loop {
            let left = timeout.saturating_sub(started.elapsed());
            match conn.output.recv_timeout(left.min(TICK)) {
                Ok(Ok(frame)) => {
                    let resp: Response =
                        serde_json::from_slice(&frame.header).map_err(|e| RenderError::frame(e.to_string()))?;
                    return Ok((resp, frame.payload));
                }
                Ok(Err(e)) => return Err(frame_failure(e, req)),
                Err(RecvTimeoutError::Disconnected) => {
                    let status = conn.child.as_mut().and_then(exit_status);
                    let message = status.map_or_else(|| "pipe closed".to_owned(), |s| s.to_string());
                    return Err(RenderError::worker_lost(message));
                }
                Err(RecvTimeoutError::Timeout) => {
                    if let Some(child) = &conn.child
                        && memory_bytes(&mut self.sys, child.id()).is_some_and(|b| b > memory)
                    {
                        return Err(RenderError::out_of_memory(self.settings.memory_mib));
                    }
                    if started.elapsed() >= timeout {
                        return Err(RenderError::timeout(self.settings.timeout_ms));
                    }
                }
            }
        }
    }

    /// Tue le travailleur sans toucher à l'état (tests de relance).
    pub(crate) fn kill(&mut self) {
        if let Some(child) = self.conn.as_mut().and_then(|c| c.child.as_mut()) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl std::fmt::Debug for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Link")
            .field("launch", &self.launch)
            .field("connected", &self.conn.is_some())
            .field("restarts", &self.restarts)
            .finish_non_exhaustive()
    }
}

impl Drop for Link {
    /// Fermeture douce : fin du tube d'entrée, le travailleur sort de lui-même ;
    /// au-delà de `GRACE`, `Conn::drop` le tue.
    fn drop(&mut self) {
        let Some(mut conn) = self.conn.take() else { return };
        conn.input = Box::new(std::io::sink());
        if let Some(child) = &mut conn.child {
            let deadline = Instant::now() + GRACE;
            while Instant::now() < deadline {
                if matches!(child.try_wait(), Ok(Some(_))) {
                    break;
                }
                std::thread::sleep(GRACE_POLL);
            }
        }
    }
}

#[cfg(test)]
#[path = "link_tests.rs"]
mod link_tests;

#[cfg(test)]
#[path = "link_fake_tests.rs"]
mod link_fake_tests;
