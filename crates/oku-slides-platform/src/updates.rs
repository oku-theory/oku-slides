//! Mises à jour de l'app : ce qui se décide sans Tauri, testé ici.
//!
//! Utilisé par l'hôte Tauri seulement ; vit ici pour que `just test` le couvre.
//! L'hôte (`src-tauri/src/updates.rs`) cherche, télécharge et installe ; ce
//! module dit s'il peut installer ([`InstallMode`]), ce qu'il montre
//! ([`UpdateStatus`], [`UpdateProgress`]), quand signaler l'avancée ou couper
//! ([`Progress`]), d'où il accepte un paquet ([`trusted_download`]) et dans
//! quels mots dire un échec ([`UpdateStage`]).

use std::path::Path;
use std::time::Duration;

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
/// Fichier (sous le dossier de données) de la dernière version qui n'a pas pu s'installer.
pub const FAILED_VERSION_FILE: &str = "updates/failed-version";

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

/// État partagé par les fenêtres : l'offre gardée, et une installation en cours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateStatus {
    /// Offre trouvée ; `None` : l'app est à jour (ou rien n'a encore été cherché).
    pub offer: Option<UpdateOffer>,
    /// Un téléchargement ou une installation a commencé.
    pub installing: bool,
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

/// The bundle comes from an Oku Slides release or its mirror, over HTTPS, without `..`.
///
/// On the mirror, only `v<major.minor.patch>/<one plain file name>` is accepted.
#[must_use]
pub fn trusted_download(url: &str) -> bool {
    if let Some(rest) = url.strip_prefix(DOWNLOAD_PREFIX) {
        return !rest.split('/').any(|part| part == "..");
    }
    url.strip_prefix(MIRROR_DOWNLOAD_PREFIX)
        .and_then(|rest| rest.split_once('/'))
        .is_some_and(|(version, name)| release_version(version) && mirrored_file_name(name))
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
        }
    }
}

impl Localize for UpdateStage {
    fn localize(&self, l: &Localizer) -> String {
        l.tr(self.key())
    }
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod updates_tests;
