//! Updates from the GitHub releases: check, download, install.
//!
//! A task checks at launch, then whenever `check_due` says so on an hourly tick
//! (`[updates]`); the offer is kept in [`Updates`] with "Later" and a failed-install
//! notice ([`UpdateMarks`]), and each change is signalled (`update-changed`, without
//! content): windows read `update_status` again. Only workspace windows may check, install
//! or open the releases page. Install: download (origin checked, size capped, signature
//! checked by the plugin), then, in one blocking job, `state::settle` and the install, then
//! `request_restart` (back through `RunEvent::Exit`). On Windows the installer ends the
//! process itself.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use oku_slides_platform::config::UpdateSettings;
use oku_slides_platform::errors::one_line;
use oku_slides_platform::updates::{
    Advance, CHECK_TICK, CHECK_TIMEOUT, CONNECT_TIMEOUT, Host, InstallFailure, InstallMode, Progress, READ_TIMEOUT,
    RELEASES_URL, UpdateMarks, UpdateOffer, UpdateSource, UpdateStage, UpdateStatus, check_due, download_source,
};
use oku_slides_session::{Localize, Localizer, Storage};
use snafu::{Backtrace, ResultExt, Snafu};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::state::AppState;

/// Update failure; the message shown depends on the stage.
#[derive(Debug, Snafu)]
pub struct UpdateError(Box<Inner>);

/// Chaque variante a son [`UpdateStage`] (`stage_of`, exhaustif : une variante
/// ajoutée ne compile pas sans son message).
#[derive(Debug, Snafu)]
enum Inner {
    #[snafu(display("update check failed"))]
    Check { source: tauri_plugin_updater::Error, backtrace: Backtrace },
    #[snafu(display("update check timed out"))]
    CheckTimeout { backtrace: Backtrace },
    /// Names the bundle and its endpoint: a proxy may block one host only (t-1212).
    #[snafu(display("update download from {url} ({endpoint}) failed"))]
    Download { url: String, endpoint: UpdateSource, source: tauri_plugin_updater::Error, backtrace: Backtrace },
    #[snafu(display("update download over the size cap"))]
    TooLarge { backtrace: Backtrace },
    #[snafu(display("update download from an untrusted url: {url}"))]
    Untrusted { url: String, backtrace: Backtrace },
    #[snafu(display("releases page not opened"))]
    Open { source: tauri_plugin_opener::Error, backtrace: Backtrace },
    #[snafu(display("no update to install"))]
    Unavailable { backtrace: Backtrace },
    #[snafu(display("update request refused: {why}"))]
    Refused { why: &'static str, backtrace: Backtrace },
    #[snafu(display("an update is already being installed"))]
    AlreadyInstalling { backtrace: Backtrace },
}

fn stage_of(inner: &Inner) -> UpdateStage {
    match inner {
        Inner::Check { .. } | Inner::CheckTimeout { .. } => UpdateStage::Check,
        Inner::Download { .. } | Inner::TooLarge { .. } | Inner::Untrusted { .. } => UpdateStage::Download,
        Inner::Open { .. } => UpdateStage::Open,
        Inner::Unavailable { .. } => UpdateStage::Unavailable,
        Inner::Refused { .. } => UpdateStage::Refused,
        Inner::AlreadyInstalling { .. } => UpdateStage::Busy,
    }
}

impl Localize for UpdateError {
    fn localize(&self, l: &Localizer) -> String {
        stage_of(&self.0).localize(l)
    }
}

/// Offer found, install under way, "Later" and the last check, shared by the windows.
pub struct Updates {
    found: Mutex<Option<(Update, UpdateOffer)>>,
    installing: AtomicBool,
    /// Marks on disk; blocking calls, outside async contexts only (`spawn_blocking`).
    marks: UpdateMarks,
    /// The version put off with "Later", as on disk.
    later: Mutex<Option<String>>,
    /// A failed install not yet told, read at startup; handed to one window, then forgotten.
    notice: Mutex<Option<String>>,
    /// Wall-clock time of the last successful check.
    last_check: Mutex<Option<SystemTime>>,
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates").finish_non_exhaustive()
    }
}

impl Updates {
    /// Reads "Later" and the failed-install notice; `data`: the app data folder.
    ///
    /// Blocking: called from the synchronous Tauri setup.
    pub fn new(storage: Storage, data: &Path) -> Self {
        let marks = UpdateMarks::new(storage, data);
        Self {
            found: Mutex::new(None),
            installing: AtomicBool::new(false),
            later: Mutex::new(marks.later()),
            notice: Mutex::new(marks.notice()),
            last_check: Mutex::new(None),
            marks,
        }
    }

    /// The offer, install under way and "Later", as the windows see them.
    pub(crate) fn status(&self) -> UpdateStatus {
        let offer = self.found.lock().unwrap_or_else(PoisonError::into_inner).as_ref().map(|(_, o)| o.clone());
        let later = self.later.lock().unwrap_or_else(PoisonError::into_inner).clone();
        UpdateStatus::new(offer, self.installing.load(Ordering::SeqCst), later.as_deref())
    }
}

/// Runs `job` on the marks on a blocking thread (they touch the disk); `None` if it failed.
async fn with_marks<R: Send + 'static>(
    marks: &UpdateMarks,
    job: impl FnOnce(&UpdateMarks) -> R + Send + 'static,
) -> Option<R> {
    let marks = marks.clone();
    match tauri::async_runtime::spawn_blocking(move || job(&marks)).await {
        Ok(result) => Some(result),
        Err(e) => {
            tracing::warn!(error = %e, "marque de mise à jour non traitée");
            None
        }
    }
}

fn changed(app: &AppHandle) {
    if let Err(e) = app.emit("update-changed", ()) {
        tracing::warn!(error = %e, "changement de mise à jour non diffusé");
    }
}

fn closing(app: &AppHandle) -> bool {
    app.try_state::<AppState>().is_some_and(|s| s.closing.load(Ordering::SeqCst))
}

/// Cherche une version ; garde le résultat (même « à jour ») et le signale.
async fn check(app: &AppHandle) -> Result<UpdateStatus, UpdateError> {
    let Some(updates) = app.try_state::<Updates>() else {
        return Err(RefusedSnafu { why: "not ready" }.build().into());
    };
    // Without these, a mirror that hangs (filtered domain) uses up CHECK_TIMEOUT before GitHub
    // is tried; the found Update keeps this client setup for its download.
    let checker = app
        .updater_builder()
        .configure_client(|client| client.connect_timeout(CONNECT_TIMEOUT).read_timeout(READ_TIMEOUT))
        .build()
        .context(CheckSnafu)?;
    let found = tokio::time::timeout(CHECK_TIMEOUT, checker.check())
        .await
        .map_err(|_| UpdateError::from(CheckTimeoutSnafu.build()))?
        .context(CheckSnafu)?;
    *updates.last_check.lock().unwrap_or_else(PoisonError::into_inner) = Some(SystemTime::now());
    if updates.installing.load(Ordering::SeqCst) {
        return Ok(updates.status());
    }
    let offer = match &found {
        Some(update) => {
            // The bundle URL tells which endpoint answered (t-1212).
            let url = update.download_url.as_str();
            if let Some(endpoint) = download_source(url) {
                tracing::info!(version = %update.version, %url, %endpoint, "update available");
            } else {
                tracing::warn!(version = %update.version, %url, "update available from an untrusted url");
            }
            // Exécutable introuvable : chemin vide, donc ni quarantaine ni image disque supposées.
            let exe = std::env::current_exe().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "exécutable courant inconnu");
                PathBuf::new()
            });
            let version = update.version.clone();
            let failed_before = with_marks(&updates.marks, move |m| m.failed_version().is_some_and(|v| v == version))
                .await
                == Some(true);
            let host = Host {
                os: std::env::consts::OS,
                appimage: std::env::var_os("APPIMAGE").is_some(),
                exe: &exe,
                failed_before,
                debug_build: cfg!(debug_assertions),
            };
            Some(UpdateOffer { version: update.version.clone(), mode: InstallMode::detect(&host) })
        }
        None => None,
    };
    *updates.found.lock().unwrap_or_else(PoisonError::into_inner) = found.zip(offer);
    changed(app);
    Ok(updates.status())
}

/// Automatic check: at launch, then on each tick when one is due; a failure is only logged.
///
/// Timers follow a monotonic clock that a sleeping Mac pauses, so the wall clock decides
/// (`check_due`) and the tick is short ([`CHECK_TICK`], never above the interval, which
/// counts whole hours): after a wake, the check comes within one tick (t-976). A failed
/// check is tried again next tick.
pub fn spawn_checks(app: &AppHandle, settings: &UpdateSettings) {
    if !settings.auto_check(cfg!(debug_assertions)) {
        tracing::debug!("recherche automatique des mises à jour désactivée");
        return;
    }
    let app = app.clone();
    let interval = settings.interval();
    tauri::async_runtime::spawn(async move {
        while !closing(&app) {
            let last =
                app.try_state::<Updates>().and_then(|u| *u.last_check.lock().unwrap_or_else(PoisonError::into_inner));
            if check_due(last, SystemTime::now(), interval) {
                match check(&app).await {
                    // An offer is logged by `check`, with its url and endpoint.
                    Ok(UpdateStatus { offer: Some(_), .. }) => {}
                    Ok(UpdateStatus { offer: None, .. }) => tracing::debug!("aucune mise à jour"),
                    Err(e) => tracing::warn!(error = %one_line(&e), "recherche de mise à jour"),
                }
            }
            tokio::time::sleep(CHECK_TICK).await;
        }
    });
}

fn refuse(state: &AppState, e: &UpdateError) -> String {
    tracing::warn!(error = %one_line(e), "mise à jour refusée");
    e.localize(&state.localizer)
}

/// L'état gardé, pour une fenêtre ouverte ou prévenue après coup.
#[tauri::command]
pub async fn update_status(updates: State<'_, Updates>) -> Result<UpdateStatus, String> {
    Ok(updates.status())
}

/// Check asked by the user (menu); forgets "Later" first. No `offer`: the app is up to date.
#[tauri::command]
pub async fn check_update(app: AppHandle, state: State<'_, AppState>) -> Result<UpdateStatus, String> {
    check_asked(&app).await.map_err(|e| refuse(&state, &e))
}

/// A check someone asked for (menu, agent): forgets "Later" first, then checks.
pub(crate) async fn check_asked(app: &AppHandle) -> Result<UpdateStatus, UpdateError> {
    let Some(updates) = app.try_state::<Updates>() else {
        return Err(RefusedSnafu { why: "not ready" }.build().into());
    };
    *updates.later.lock().unwrap_or_else(PoisonError::into_inner) = None;
    // Shown again even if the check below fails.
    changed(app);
    with_marks(&updates.marks, |m| {
        if let Err(e) = m.clear_later() {
            tracing::warn!(error = %one_line(&e), "« Plus tard » non oublié sur le disque");
        }
    })
    .await;
    check(app).await
}

/// "Later": puts `version` off, if it is still the offer; the status that follows.
#[tauri::command]
pub async fn later_update(
    app: AppHandle,
    updates: State<'_, Updates>,
    version: String,
) -> Result<UpdateStatus, String> {
    Ok(put_off(&app, &updates, version).await)
}

/// Puts `version` off if it is still the offer; the status that follows.
pub(crate) async fn put_off(app: &AppHandle, updates: &Updates, version: String) -> UpdateStatus {
    // Decided under the offer's lock: a newer offer found meanwhile was never shown, so it is
    // not put off, and a check cannot replace the offer between the test and the mark.
    let accepted = {
        let found = updates.found.lock().unwrap_or_else(PoisonError::into_inner);
        let current = found.as_ref().is_some_and(|(_, offer)| offer.version == version);
        if current {
            *updates.later.lock().unwrap_or_else(PoisonError::into_inner) = Some(version.clone());
        }
        current
    };
    if accepted {
        changed(app);
        with_marks(&updates.marks, move |m| {
            if let Err(e) = m.set_later(&version) {
                tracing::warn!(error = %one_line(&e), "« Plus tard » gardé pour cette session seulement");
            }
        })
        .await;
    }
    updates.status()
}

/// The stage `e` failed at, which tells the agent's refusal.
pub(crate) fn stage(e: &UpdateError) -> UpdateStage {
    stage_of(&e.0)
}

/// The failed-install notice, localized, for the first window that asks; then `None`.
///
/// Removed from disk only now: a launch that never shows a window tells it next time.
#[tauri::command]
pub async fn take_update_notice(
    state: State<'_, AppState>,
    updates: State<'_, Updates>,
) -> Result<Option<String>, String> {
    let version = updates.notice.lock().unwrap_or_else(PoisonError::into_inner).take();
    if version.is_some() {
        with_marks(&updates.marks, |m| {
            if let Err(e) = m.clear_notice() {
                tracing::warn!(error = %one_line(&e), "notice d'échec de mise à jour non retirée");
            }
        })
        .await;
    }
    Ok(version.map(|version| InstallFailure { version }.localize(&state.localizer)))
}

/// Télécharge, puis enregistre l'espace de travail, installe et redémarre.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    install(&app).await.map_err(|e| refuse(&state, &e))
}

/// Downloads, then saves the workspace, installs and restarts; for any caller, window or agent.
pub(crate) async fn install(app: &AppHandle) -> Result<(), UpdateError> {
    let Some(updates) = app.try_state::<Updates>() else {
        return Err(RefusedSnafu { why: "not ready" }.build().into());
    };
    if closing(app) {
        return Err(RefusedSnafu { why: "app closing" }.build().into());
    }
    let found = updates.found.lock().unwrap_or_else(PoisonError::into_inner).clone();
    let Some((update, offer)) = found.filter(|(_, o)| o.mode == InstallMode::Install) else {
        return Err(UnavailableSnafu.build().into());
    };
    let Some(endpoint) = download_source(update.download_url.as_str()) else {
        return Err(UntrustedSnafu { url: update.download_url.to_string() }.build().into());
    };
    // Une autre fenêtre installe déjà : rien à refaire, mais elle le sait (`update-changed`).
    if updates.installing.swap(true, Ordering::SeqCst) {
        return Err(AlreadyInstallingSnafu.build().into());
    }
    changed(app);
    let bytes = match download(app, &update, endpoint).await {
        Ok(bytes) => bytes,
        Err(e) => {
            updates.installing.store(false, Ordering::SeqCst);
            changed(app);
            return Err(e);
        }
    };
    if closing(app) {
        // L'utilisateur a quitté pendant le téléchargement : la fin suit son cours.
        return Err(RefusedSnafu { why: "app closing" }.build().into());
    }
    // The windows' close guards first, as for a quit (t-1377). A quit already underway wins and
    // exits; a quit asked while this install waits joins it, and the app restarts into the update.
    if !crate::quit::guarded(app).await {
        updates.installing.store(false, Ordering::SeqCst);
        return Err(RefusedSnafu { why: "app closing" }.build().into());
    }
    tracing::info!(version = %offer.version, "mise à jour téléchargée : enregistrement puis installation");
    let job = app.clone();
    let version = offer.version.clone();
    let marks = updates.marks.clone();
    let done = tauri::async_runtime::spawn_blocking(move || {
        crate::state::settle(&job);
        let installed = update.install(bytes);
        if let Err(e) = &installed
            && let Err(w) = marks.record_failure(&version)
        {
            tracing::error!(error = %e, write = %one_line(&w), "version en échec non gardée");
        }
        installed
    })
    .await;
    match done {
        Ok(Ok(())) => tracing::info!("mise à jour installée : redémarrage"),
        Ok(Err(e)) => {
            tracing::error!(error = %e, "installation de la mise à jour : redémarrage sur la version actuelle");
        }
        // `settle` a paniqué : rien n'a été installé ; `RunEvent::Exit` rappelle `Workspace::settle`.
        Err(e) => tracing::error!(error = %e, "enregistrement avant mise à jour"),
    }
    app.request_restart();
    Ok(())
}

/// Downloads the bundle; stops past the size cap (before the signature check).
async fn download(app: &AppHandle, update: &Update, endpoint: UpdateSource) -> Result<Vec<u8>, UpdateError> {
    let too_large = Arc::new(tokio::sync::Notify::new());
    let over = Arc::new(AtomicBool::new(false));
    let mut progress = Progress::new();
    let emitter = app.clone();
    let (notify, flag) = (Arc::clone(&too_large), Arc::clone(&over));
    let fetch = update.download(
        move |chunk, total| match progress.advance(chunk, total) {
            Advance::Report(p) => {
                if let Err(e) = emitter.emit("update-progress", p) {
                    tracing::debug!(error = %e, "avancée non diffusée");
                }
            }
            Advance::Quiet => {}
            Advance::TooLarge => {
                if !flag.swap(true, Ordering::SeqCst) {
                    notify.notify_one();
                }
            }
        },
        || {},
    );
    let failed = DownloadSnafu { url: update.download_url.as_str(), endpoint };
    tokio::select! {
        bytes = fetch => bytes.context(failed).map_err(UpdateError::from),
        () = too_large.notified() => Err(TooLargeSnafu.build().into()),
    }
}

/// Ouvre la page des releases (paquet à installer à la main).
#[tauri::command]
pub async fn open_release_page(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    app.opener()
        .open_url(RELEASES_URL, None::<&str>)
        .context(OpenSnafu)
        .map_err(|e| refuse(&state, &UpdateError::from(e)))
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
