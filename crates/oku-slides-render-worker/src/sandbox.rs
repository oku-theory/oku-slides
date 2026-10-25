//! Le bac à sable Seatbelt du travailleur, sur macOS (t-912).
//!
//! `sandbox-exec -p <worker.sb>` pose le profil puis exécute le travailleur
//! (même PID : chien de garde, mémoire et `killpg` inchangés). Le travailleur
//! et LibreOffice, son fils, n'écrivent que dans `work` (leur `TMPDIR`), ne
//! joignent aucune adresse IP, n'exécutent que le travailleur et `soffice`,
//! n'envoient ni Apple Event ni `LSOpen`. Le canari, hors de `work`, prouve à
//! chaque `Hello` que le profil tient. Ailleurs que sur macOS : rien.

use std::path::Path;
use std::process::Command;

use oku_slides_render::RenderError;

/// Le bac à sable d'un travailleur lancé : son dossier, son canari.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(crate) struct Sandbox {
    /// Gardé pour son `Drop` : supprimé avec le bac à sable, après la mort du travailleur (`Conn`).
    _dir: tempfile::TempDir,
    /// Le dossier, en chemin réel : Seatbelt compare des chemins réels.
    work: std::path::PathBuf,
    canary: std::path::PathBuf,
    worker: std::path::PathBuf,
    soffice: std::path::PathBuf,
}

/// Ailleurs que sur macOS, aucun bac à sable n'existe.
#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
pub(crate) enum Sandbox {}

#[cfg(not(target_os = "macos"))]
impl Sandbox {
    /// Sans objet hors macOS.
    pub(crate) fn command(&self) -> Command {
        match *self {}
    }

    /// Sans objet hors macOS.
    pub(crate) fn work(&self) -> &Path {
        match *self {}
    }

    /// Sans objet hors macOS.
    pub(crate) fn canary(&self) -> &Path {
        match *self {}
    }
}

/// Le bac à sable du travailleur `program`, qui pourra lancer `soffice`.
#[cfg(target_os = "macos")]
pub(crate) fn sandbox_for(program: &Path, soffice: Option<&Path>) -> Result<Option<Sandbox>, RenderError> {
    Sandbox::new(program, soffice).map(Some)
}

/// Hors macOS, pas de bac à sable : le travailleur est lancé tel quel.
#[cfg(not(target_os = "macos"))]
pub(crate) fn sandbox_for(_program: &Path, _soffice: Option<&Path>) -> Result<Option<Sandbox>, RenderError> {
    Ok(None)
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;

    use oku_slides_platform::storage::canonical;

    use super::{Command, Path, RenderError, Sandbox};

    /// Pose le profil puis exécute le travailleur, sous le même PID.
    pub(crate) const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
    /// Le profil Seatbelt du travailleur.
    const PROFILE: &str = include_str!("worker.sb");
    /// `SOFFICE` sans LibreOffice : un chemin qui n'existe pas, rien d'autre ne s'exécute.
    const NO_PROGRAM: &str = "/var/empty/none";

    impl Sandbox {
        /// Un dossier neuf pour le travailleur `program`, qui pourra lancer `soffice`.
        pub(crate) fn new(program: &Path, soffice: Option<&Path>) -> Result<Self, RenderError> {
            let lost = |what: &str, e: &dyn std::fmt::Display| RenderError::worker_lost(format!("sandbox {what}: {e}"));
            let dir =
                tempfile::Builder::new().prefix("oku-slides-worker-").tempdir().map_err(|e| lost("work dir", &e))?;
            let work = canonical(dir.path()).map_err(|e| lost("work dir", &e))?;
            let mut canary_name = work
                .file_name()
                .map(OsStr::to_owned)
                .ok_or_else(|| RenderError::worker_lost(format!("sandbox work dir {}", work.display())))?;
            canary_name.push(".canary");
            let canary = work.with_file_name(canary_name);
            let worker = canonical(program).map_err(|e| lost("worker", &e))?;
            let soffice = match soffice {
                Some(p) => canonical(p).map_err(|e| lost("soffice", &e))?,
                None => PathBuf::from(NO_PROGRAM),
            };
            Ok(Self { _dir: dir, work, canary, worker, soffice })
        }

        /// `sandbox-exec -p <profil> -D … <travailleur>`, `TMPDIR` posé, sans ses arguments.
        pub(crate) fn command(&self) -> Command {
            let mut cmd = Command::new(SANDBOX_EXEC);
            cmd.arg("-p").arg(PROFILE);
            for (name, value) in [
                ("WORK", self.work.as_os_str()),
                ("UID", OsStr::new(&uid())),
                ("WORKER", self.worker.as_os_str()),
                ("SOFFICE", self.soffice.as_os_str()),
            ] {
                cmd.arg("-D").arg(define(name, value));
            }
            let mut tmpdir = self.work.clone().into_os_string();
            tmpdir.push("/");
            cmd.arg(&self.worker).env("TMPDIR", tmpdir);
            cmd
        }

        /// Le seul dossier où le travailleur écrit (son `TMPDIR`).
        pub(crate) fn work(&self) -> &Path {
            &self.work
        }

        /// Un chemin hors de `work`, que le travailleur doit échouer à créer.
        pub(crate) fn canary(&self) -> &Path {
            &self.canary
        }
    }

    /// `NOM=valeur`, un paramètre du profil.
    fn define(name: &str, value: &OsStr) -> OsString {
        let mut d = OsString::from(name);
        d.push("=");
        d.push(value);
        d
    }

    /// L'utilisateur courant, dans le nom du socket de LibreOffice.
    #[expect(unsafe_code, reason = "getuid : appel système sans enveloppe sûre")]
    pub(crate) fn uid() -> String {
        // SAFETY: getuid n'a ni précondition ni cas d'échec.
        unsafe { libc::getuid() }.to_string()
    }
}

#[cfg(all(test, target_os = "macos"))]
use macos::uid;

#[cfg(all(test, target_os = "macos"))]
#[path = "sandbox_tests.rs"]
mod sandbox_tests;
