//! Mises à jour depuis les releases GitHub : recherche, téléchargement, installation.
//!
//! Une tâche cherche au lancement puis à intervalle (`[updates]`) ; l'offre est
//! gardée dans [`Updates`] et chaque changement est signalé (`update-changed`,
//! sans contenu) : les fenêtres relisent `update_status`. Seules les fenêtres de
//! l'espace de travail peuvent chercher, installer ou ouvrir la page des
//! releases. Installer : télécharger (origine vérifiée, taille plafonnée,
//! signature vérifiée par le greffon), puis, dans un même travail bloquant,
//! `state::settle` et l'installation, puis `request_restart` (qui repasse par
//! `RunEvent::Exit`). Sous Windows, l'installeur ferme lui-même le processus.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_platform::config::UpdateSettings;
use oku_slides_platform::errors::one_line;
use oku_slides_platform::updates::{
    Advance, CHECK_TIMEOUT, FAILED_VERSION_FILE, Host, InstallMode, Progress, RELEASES_URL, UpdateOffer, UpdateStage,
    UpdateStatus, trusted_download,
};
use oku_slides_session::{Localize, Localizer, Storage};
use snafu::{Backtrace, ResultExt, Snafu};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::agent_window;
use crate::state::AppState;

/// Échec d'une mise à jour ; le message dépend de l'étape.
#[derive(Debug, Snafu)]
pub struct UpdateError(Inner);

/// Chaque variante a son [`UpdateStage`] (`stage_of`, exhaustif : une variante
/// ajoutée ne compile pas sans son message).
#[derive(Debug, Snafu)]
enum Inner {
    #[snafu(display("update check failed"))]
    Check { source: tauri_plugin_updater::Error, backtrace: Backtrace },
    #[snafu(display("update check timed out"))]
    CheckTimeout { backtrace: Backtrace },
    #[snafu(display("update download failed"))]
    Download { source: tauri_plugin_updater::Error, backtrace: Backtrace },
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
}

fn stage_of(inner: &Inner) -> UpdateStage {
    match inner {
        Inner::Check { .. } | Inner::CheckTimeout { .. } => UpdateStage::Check,
        Inner::Download { .. } | Inner::TooLarge { .. } | Inner::Untrusted { .. } => UpdateStage::Download,
        Inner::Open { .. } => UpdateStage::Open,
        Inner::Unavailable { .. } => UpdateStage::Unavailable,
        Inner::Refused { .. } => UpdateStage::Refused,
    }
}

impl Localize for UpdateError {
    fn localize(&self, l: &Localizer) -> String {
        stage_of(&self.0).localize(l)
    }
}

/// Offre trouvée et installation en cours, partagées par les fenêtres.
pub struct Updates {
    found: Mutex<Option<(Update, UpdateOffer)>>,
    installing: AtomicBool,
    /// Pour lire et écrire [`FAILED_VERSION_FILE`] (hors contexte async seulement).
    storage: Storage,
    failed_file: PathBuf,
}

impl std::fmt::Debug for Updates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updates").finish_non_exhaustive()
    }
}

impl Updates {
    /// État vide ; `data` : dossier de données de l'app.
    pub fn new(storage: Storage, data: &Path) -> Self {
        Self {
            found: Mutex::new(None),
            installing: AtomicBool::new(false),
            storage,
            failed_file: data.join(FAILED_VERSION_FILE),
        }
    }

    fn status(&self) -> UpdateStatus {
        let offer = self.found.lock().unwrap_or_else(PoisonError::into_inner).as_ref().map(|(_, o)| o.clone());
        UpdateStatus { offer, installing: self.installing.load(Ordering::SeqCst) }
    }

    /// Version dont l'installation a échoué (absente : aucune) ; travail bloquant.
    fn failed_version(&self) -> Option<String> {
        match self.storage.read_limited(&self.failed_file, 64) {
            Ok(bytes) => String::from_utf8(bytes).ok().map(|v| v.trim().to_owned()),
            Err(e) => {
                tracing::debug!(error = %one_line(&e), "aucune version en échec lue");
                None
            }
        }
    }
}

fn changed(app: &AppHandle) {
    if let Err(e) = app.emit("update-changed", ()) {
        tracing::warn!(error = %e, "changement de mise à jour non diffusé");
    }
}

/// Refuse la fenêtre de l'agent : elle affiche du contenu venu de l'agent et des imports.
fn workspace_window(window: &WebviewWindow) -> Result<(), UpdateError> {
    if window.label() == agent_window::LABEL {
        return Err(RefusedSnafu { why: "agent window" }.build().into());
    }
    Ok(())
}

fn closing(app: &AppHandle) -> bool {
    app.try_state::<AppState>().is_some_and(|s| s.closing.load(Ordering::SeqCst))
}

/// Cherche une version ; garde le résultat (même « à jour ») et le signale.
async fn check(app: &AppHandle) -> Result<UpdateStatus, UpdateError> {
    let Some(updates) = app.try_state::<Updates>() else {
        return Err(RefusedSnafu { why: "not ready" }.build().into());
    };
    let updater = app.updater().context(CheckSnafu)?;
    let found = tokio::time::timeout(CHECK_TIMEOUT, updater.check())
        .await
        .map_err(|_| UpdateError::from(CheckTimeoutSnafu.build()))?
        .context(CheckSnafu)?;
    if updates.installing.load(Ordering::SeqCst) {
        return Ok(updates.status());
    }
    let offer = match &found {
        Some(update) => {
            // Exécutable introuvable : chemin vide, donc ni quarantaine ni image disque supposées.
            let exe = std::env::current_exe().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "exécutable courant inconnu");
                PathBuf::new()
            });
            let failed_before = {
                let app = app.clone();
                let version = update.version.clone();
                match tauri::async_runtime::spawn_blocking(move || {
                    app.try_state::<Updates>().and_then(|u| u.failed_version()).is_some_and(|v| v == version)
                })
                .await
                {
                    Ok(failed) => failed,
                    Err(e) => {
                        tracing::warn!(error = %e, "version en échec non lue");
                        false
                    }
                }
            };
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

/// Recherche automatique : au lancement, puis à chaque intervalle ; un échec est seulement journalisé.
///
/// L'horloge de `tokio::time::sleep` est monotone : un Mac en veille repousse la
/// recherche suivante d'autant (accepté, t-976).
pub fn spawn_checks(app: &AppHandle, settings: &UpdateSettings) {
    if !settings.auto_check(cfg!(debug_assertions)) {
        tracing::debug!("recherche automatique des mises à jour désactivée");
        return;
    }
    let app = app.clone();
    let interval = settings.interval();
    tauri::async_runtime::spawn(async move {
        while !closing(&app) {
            match check(&app).await {
                Ok(UpdateStatus { offer: Some(offer), .. }) => {
                    tracing::info!(version = %offer.version, "mise à jour disponible")
                }
                Ok(_) => tracing::debug!("aucune mise à jour"),
                Err(e) => tracing::warn!(error = %one_line(&e), "recherche de mise à jour"),
            }
            tokio::time::sleep(interval).await;
        }
    });
}

fn refuse(state: &AppState, e: &UpdateError) -> String {
    tracing::warn!(error = %one_line(e), "mise à jour refusée");
    e.localize(&state.localizer)
}

/// L'état gardé, pour une fenêtre ouverte ou prévenue après coup.
#[tauri::command]
pub async fn update_status(
    window: WebviewWindow,
    state: State<'_, AppState>,
    updates: State<'_, Updates>,
) -> Result<UpdateStatus, String> {
    workspace_window(&window).map_err(|e| refuse(&state, &e))?;
    Ok(updates.status())
}

/// Recherche demandée (menu) ; `offer` absente : l'app est à jour.
#[tauri::command]
pub async fn check_update(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<UpdateStatus, String> {
    workspace_window(&window).map_err(|e| refuse(&state, &e))?;
    check(&app).await.map_err(|e| refuse(&state, &e))
}

/// Télécharge, puis enregistre l'espace de travail, installe et redémarre.
#[tauri::command]
pub async fn install_update(app: AppHandle, window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    workspace_window(&window).map_err(|e| refuse(&state, &e))?;
    install(&app).await.map_err(|e| refuse(&state, &e))
}

async fn install(app: &AppHandle) -> Result<(), UpdateError> {
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
    if !trusted_download(update.download_url.as_str()) {
        return Err(UntrustedSnafu { url: update.download_url.to_string() }.build().into());
    }
    // Une autre fenêtre installe déjà : rien à refaire, mais elle le sait (`update-changed`).
    if updates.installing.swap(true, Ordering::SeqCst) {
        return Err(RefusedSnafu { why: "already installing" }.build().into());
    }
    changed(app);
    let bytes = match download(app, &update).await {
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
    tracing::info!(version = %offer.version, "mise à jour téléchargée : enregistrement puis installation");
    let job = app.clone();
    let version = offer.version.clone();
    let done = tauri::async_runtime::spawn_blocking(move || {
        crate::state::settle(&job);
        let installed = update.install(bytes);
        if let Err(e) = &installed
            && let Some(updates) = job.try_state::<Updates>()
            && let Err(w) = updates.storage.write_atomic(&updates.failed_file, version.into_bytes())
        {
            tracing::error!(error = %e, write = %one_line(&w), "version en échec non gardée");
        }
        installed
    })
    .await;
    match done {
        Ok(Ok(())) => tracing::info!("mise à jour installée : redémarrage"),
        Ok(Err(e)) => {
            tracing::error!(error = %e, "installation de la mise à jour : redémarrage sur la version actuelle")
        }
        // `settle` a paniqué : rien n'a été installé ; `RunEvent::Exit` rappelle `Workspace::settle`.
        Err(e) => tracing::error!(error = %e, "enregistrement avant mise à jour"),
    }
    app.request_restart();
    Ok(())
}

/// Télécharge le paquet ; coupe au-delà du plafond (avant la vérification de la signature).
async fn download(app: &AppHandle, update: &Update) -> Result<Vec<u8>, UpdateError> {
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
    tokio::select! {
        bytes = fetch => bytes.context(DownloadSnafu).map_err(UpdateError::from),
        () = too_large.notified() => Err(TooLargeSnafu.build().into()),
    }
}

/// Ouvre la page des releases (paquet à installer à la main).
#[tauri::command]
pub async fn open_release_page(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<(), String> {
    workspace_window(&window).map_err(|e| refuse(&state, &e))?;
    app.opener()
        .open_url(RELEASES_URL, None::<&str>)
        .context(OpenSnafu)
        .map_err(|e| refuse(&state, &UpdateError::from(e)))
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
