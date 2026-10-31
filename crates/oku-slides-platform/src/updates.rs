//! App updates: the decisions made without Tauri, tested here.
//!
//! Used by the Tauri host only; lives here so that `just test` covers it.
//! The host (`src-tauri/src/updates.rs`) checks, downloads and installs; this
//! module says whether it can install ([`InstallMode`]), what it shows
//! ([`UpdateStatus`], [`UpdateProgress`]), when to report progress or stop
//! ([`Progress`]), where it accepts a bundle from and which endpoint that is
//! ([`download_source`], [`UpdateSource`]), in which words to report a failure
//! ([`UpdateStage`], [`InstallFailure`]), when the next automatic check is due
//! ([`check_due`], [`CHECK_TICK`]) and what it keeps on disk ([`UpdateMarks`]).

use std::path::Path;
use std::time::{Duration, SystemTime};

mod marks;
pub use marks::UpdateMarks;

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;

/// Page des releases, ouverte quand l'app ne peut pas s'installer elle-même.
pub const RELEASES_URL: &str = "https://github.com/oku-theory/oku-slides/releases/latest";
/// Download prefix of GitHub release assets (`latest.json` is unsigned, bundles are).
pub const DOWNLOAD_PREFIX: &str = "https://github.com/oku-theory/oku-slides/releases/download/";
/// Download prefix of the update mirror, read first (ADR-0010, t-1209).
///
/// Must match `plugins.updater.endpoints` in `tauri.conf.json` and `MIRROR_URL` in
/// `.github/workflows/update-mirror.yml`.
pub const MIRROR_DOWNLOAD_PREFIX: &str = "https://updates.okutheory.com/v";
/// Taille maximale d'un paquet téléchargé, vérifiée avant la signature.
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
/// Longest a version check may take, across every updater endpoint.
///
/// Above two [`CONNECT_TIMEOUT`] plus [`READ_TIMEOUT`], so a hanging mirror still leaves time
/// for the GitHub endpoint (t-1209).
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(60);
/// Longest wait for one updater connection (TCP, proxy tunnel, TLS) before the next endpoint.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Longest silence on an updater response, check or download, before giving up.
pub const READ_TIMEOUT: Duration = Duration::from_secs(20);
/// File (under the data folder) of the last version that could not be installed.
pub(crate) const FAILED_VERSION_FILE: &str = "updates/failed-version";
/// File of a failed install not yet told to the user, told at the next launch (t-985).
pub(crate) const FAILED_NOTICE_FILE: &str = "updates/failed-notice";
/// File of the version the user put off with "Later" (t-979).
pub(crate) const LATER_VERSION_FILE: &str = "updates/later-version";
/// Wake-up period of the automatic check; each wake checks whether one is due (t-976).
///
/// Short, because a sleeping Mac pauses the monotonic clock behind timers: the wall clock
/// ([`check_due`]) decides, and a wake waits at most one tick.
pub const CHECK_TICK: Duration = Duration::from_secs(3600);

/// Is an automatic check due, `interval` after the `last` one?
///
/// Due without a last check, once `interval` has passed on the wall clock, and when the
/// clock went back before `last`.
#[must_use]
pub fn check_due(last: Option<SystemTime>, now: SystemTime, interval: Duration) -> bool {
    last.is_none_or(|last| !now.duration_since(last).is_ok_and(|elapsed| elapsed < interval))
}

/// Ce que l'app peut faire d'une nouvelle version sur ce système.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMode {
    /// Télécharger, installer et redémarrer.
    Install,
    /// Seulement prévenir : l'utilisateur installe depuis la page des releases.
    NotifyOnly,
}

/// Ce que l'hôte sait de lui-même pour choisir un [`InstallMode`].
#[derive(Debug, Clone, Copy)]
pub struct Host<'a> {
    /// `std::env::consts::OS`.
    pub os: &'a str,
    /// `APPIMAGE` est posée (Linux : lancé depuis un AppImage).
    pub appimage: bool,
    /// Exécutable courant (`std::env::current_exe`).
    pub exe: &'a Path,
    /// L'installation de cette même version a déjà échoué.
    pub failed_before: bool,
    /// Version de développement (`cfg!(debug_assertions)`).
    pub debug_build: bool,
}

impl InstallMode {
    /// Le mode d'après l'hôte : paquet du système, image disque, quarantaine, version de développement ou échec passé.
    #[must_use]
    pub fn detect(host: &Host<'_>) -> Self {
        let exe = host.exe.to_string_lossy();
        let unwritable = match host.os {
            "linux" => !host.appimage,
            // Lancée depuis Téléchargements (quarantaine) ou depuis le .dmg monté : le
            // paquet ne peut pas être remplacé. Hors d'un paquet `.app`, l'installation
            // remplacerait le dossier de l'exécutable. `/Volumes/` couvre aussi les apps
            // sur disque externe (faux positif accepté : elles ne font que prévenir).
            "macos" => {
                exe.contains("/AppTranslocation/")
                    || exe.starts_with("/Volumes/")
                    || !exe.contains(".app/Contents/MacOS/")
            }
            _ => false,
        };
        if unwritable || host.failed_before || host.debug_build { Self::NotifyOnly } else { Self::Install }
    }
}

/// Une version plus récente, telle que l'interface la montre.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateOffer {
    /// Version proposée, sans `v`.
    pub version: String,
    /// Installer ou seulement prévenir.
    pub mode: InstallMode,
}

/// State shared by the windows: the kept offer, an install under way, "Later".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateStatus {
    /// Offer found; `None`: the app is up to date (or nothing was checked yet).
    pub offer: Option<UpdateOffer>,
    /// A download or an install has started.
    pub installing: bool,
    /// The user put this offer's version off with "Later".
    pub dismissed: bool,
}

impl UpdateStatus {
    /// The status of `offer`, dismissed when its version is the one put off (`later`).
    #[must_use]
    pub fn new(offer: Option<UpdateOffer>, installing: bool, later: Option<&str>) -> Self {
        let dismissed = offer.as_ref().is_some_and(|o| later == Some(o.version.as_str()));
        Self { offer, installing, dismissed }
    }
}

/// Avancée d'un téléchargement, diffusée aux fenêtres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UpdateProgress {
    /// Octets reçus.
    pub received: u64,
    /// Taille annoncée ; `None` si inconnue (ou annoncée nulle).
    pub total: Option<u64>,
}

/// Ce que [`Progress::advance`] demande de faire du morceau reçu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Advance {
    /// Signaler cette avancée.
    Report(UpdateProgress),
    /// Rien à signaler.
    Quiet,
    /// Au-delà de [`MAX_DOWNLOAD_BYTES`] : couper le téléchargement.
    TooLarge,
}

/// Octets entre deux signalements quand la taille totale est inconnue.
const UNKNOWN_TOTAL_STEP: u64 = 1024 * 1024;

/// Avancée d'un téléchargement : pas de 1 % (ou de 1 Mio), coupé au-delà du plafond.
#[derive(Debug, Default)]
pub struct Progress {
    received: u64,
    reported: Option<u64>,
}

impl Progress {
    /// Rien de reçu.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute `chunk` octets et dit s'il faut signaler, se taire ou couper.
    pub fn advance(&mut self, chunk: usize, total: Option<u64>) -> Advance {
        let total = total.filter(|&t| t > 0);
        self.received = self.received.saturating_add(u64::try_from(chunk).unwrap_or(u64::MAX));
        if self.received > MAX_DOWNLOAD_BYTES || total.is_some_and(|t| t > MAX_DOWNLOAD_BYTES) {
            return Advance::TooLarge;
        }
        let step = total.map_or(UNKNOWN_TOTAL_STEP, |t| (t / 100).max(1));
        let due = match self.reported {
            None => true,
            Some(last) => {
                let crossed_total = total.is_some_and(|t| self.received >= t && last < t);
                (self.received.saturating_sub(last) >= step && total.is_none_or(|t| last < t)) || crossed_total
            }
        };
        if !due {
            return Advance::Quiet;
        }
        self.reported = Some(self.received);
        Advance::Report(UpdateProgress { received: self.received, total })
    }
}

/// Updater endpoint a bundle URL comes from, logged with download failures (t-1212).
///
/// Each endpoint's `latest.json` names only its own bundles, so the URL tells which one answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSource {
    /// `updates.okutheory.com`, read first (ADR-0010).
    Mirror,
    /// GitHub release assets.
    GitHub,
}

impl UpdateSource {
    /// Lowercase label written in logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mirror => "mirror",
            Self::GitHub => "github",
        }
    }
}

impl std::fmt::Display for UpdateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Endpoint of a trusted bundle URL; `None` when the host must refuse it.
///
/// Trusted: an Oku Slides release or its mirror, over HTTPS, without `..`. On the mirror,
/// only `v<major.minor.patch>/<one plain file name>` is accepted.
#[must_use]
pub fn download_source(url: &str) -> Option<UpdateSource> {
    if let Some(rest) = url.strip_prefix(DOWNLOAD_PREFIX) {
        return (!rest.split('/').any(|part| part == "..")).then_some(UpdateSource::GitHub);
    }
    url.strip_prefix(MIRROR_DOWNLOAD_PREFIX)
        .and_then(|rest| rest.split_once('/'))
        .is_some_and(|(version, name)| release_version(version) && mirrored_file_name(name))
        .then_some(UpdateSource::Mirror)
}

/// `major.minor.patch`, each part made of ASCII digits.
fn release_version(version: &str) -> bool {
    let mut parts = 0;
    let digits = version.split('.').all(|part| {
        parts += 1;
        !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())
    });
    digits && parts == 3
}

/// One path segment, with only the characters `update-mirror.yml` allows in a name.
fn mirrored_file_name(name: &str) -> bool {
    !name.is_empty() && !name.contains("..") && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
}

/// Étape d'une mise à jour qui a échoué : choisit le message montré.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdateStage {
    /// Recherche d'une version (réseau, délai, `latest.json`).
    Check,
    /// Téléchargement, taille, origine ou signature du paquet.
    Download,
    /// Ouverture de la page des releases.
    Open,
    /// Aucune version à installer ici (rien de trouvé, ou paquet à installer à la main).
    Unavailable,
    /// Demande venue d'une fenêtre qui n'y a pas droit, ou app en train de quitter.
    Refused,
    /// Another window already started the install (t-986).
    Busy,
}

impl UpdateStage {
    /// Clé Fluent du message de cette étape.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Check => "update-error-check",
            Self::Download => "update-error-download",
            Self::Open => "update-error-open",
            Self::Unavailable => "update-error-unavailable",
            Self::Refused => "update-error-refused",
            Self::Busy => "update-error-busy",
        }
    }
}

impl Localize for UpdateStage {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(self.key())
    }
}

/// An install that failed before the restart, told once at the next launch (t-985).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallFailure {
    /// The version that could not be installed.
    pub version: String,
}

impl Localize for InstallFailure {
    fn localize(&self, l: &Localizer) -> String {
        l.tr_with("update-error-install", &[("version", self.version.as_str().into())])
    }
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
