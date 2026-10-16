//! Un programme externe lancé sans shell, borné par une échéance : passé
//! elle, il est tué avec son groupe de processus (Unix), et ses sorties lues
//! jusqu'à [`MAX_CAPTURE`] octets chacune.

use std::ffi::OsString;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

/// Octets gardés de stdout et de stderr ; le reste est lu et jeté.
pub(crate) const MAX_CAPTURE: usize = 64 * 1024;

/// Intervalle entre deux regards sur le processus.
const POLL: Duration = Duration::from_millis(20);

/// Attente des sorties une fois le processus fini : un petit-enfant sorti du
/// groupe qui garderait les tubes ouverts ne bloque pas l'appelant.
const DRAIN_GRACE: Duration = Duration::from_secs(1);

/// Un programme fini avant l'échéance.
#[derive(Debug)]
pub(crate) struct Finished {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

/// Pourquoi un programme n'a pas fini.
#[derive(Debug)]
pub(crate) enum RunError {
    /// Il ne s'est pas lancé (absent, pas exécutable).
    Spawn(std::io::Error),
    /// Son état est illisible.
    Wait(std::io::Error),
    /// Il a passé l'échéance ; il a été tué.
    Timeout,
}

/// Lance `program` avec `args` (sans shell, chacun séparé) et attend sa fin
/// jusqu'à `deadline`.
pub(crate) fn run(program: &Path, args: &[OsString], deadline: Instant) -> Result<Finished, RunError> {
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Son propre groupe : on le tue en entier, enfants compris.
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(RunError::Spawn)?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if Instant::now() >= deadline => break Err(RunError::Timeout),
            Ok(None) => thread::sleep(POLL),
            Err(e) => break Err(RunError::Wait(e)),
        }
    };
    end(&mut child);
    let status = status?;
    Ok(Finished { status, stdout: collect(&stdout), stderr: collect(&stderr) })
}

/// Tue `child` et son groupe, puis le récolte : rien ne lui survit.
fn end(child: &mut Child) {
    #[cfg(unix)]
    crate::group::kill_group(child.id());
    // Déjà fini : `kill` n'a rien à faire et son erreur ne dit rien.
    let _ = child.kill();
    // Récolté plus tôt par `try_wait`, ou mort de SIGKILL : l'attente est brève.
    let _ = child.wait();
}

/// Lit `pipe` jusqu'au bout sur un fil à lui ; en garde [`MAX_CAPTURE`] octets.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        thread::spawn(move || {
            let mut kept = Vec::new();
            let mut chunk = [0u8; 8192];
            while let Ok(n) = pipe.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                let room = MAX_CAPTURE.saturating_sub(kept.len());
                kept.extend_from_slice(&chunk[..n.min(room)]);
            }
            // L'appelant a pu cesser d'attendre : la sortie n'intéresse plus personne.
            let _ = tx.send(kept);
        });
    }
    rx
}

/// Ce que [`drain`] a lu, vide si le tube n'est pas fermé à temps.
fn collect(rx: &Receiver<Vec<u8>>) -> Vec<u8> {
    rx.recv_timeout(DRAIN_GRACE).unwrap_or_default()
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod process_tests;
