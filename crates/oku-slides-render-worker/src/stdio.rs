//! Sortie du protocole du travailleur : sa propre copie de stdout ; stdout
//! (descripteur 1) pointe ensuite sur stderr, pour qu'aucun `print!` (une
//! dépendance, Typst) ne se glisse entre deux trames.
#![expect(unsafe_code, reason = "dup2 : std ne sait pas rediriger un descripteur ouvert")]

use std::io::{self, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};

/// Copie `out` dans un nouveau descripteur (fermé à l'`exec`), puis fait
/// pointer `out` sur la cible de `err`. Rend la copie.
pub(crate) fn divert(out: BorrowedFd<'_>, err: BorrowedFd<'_>) -> io::Result<OwnedFd> {
    let copy = out.try_clone_to_owned()?;
    // SAFETY: `dup2` ne touche à aucune mémoire de Rust. `out` et `err` sont
    // empruntés, donc ouverts pendant l'appel. `out` reste ouvert : il désigne
    // désormais la cible de `err`, et son propriétaire n'en voit ni la
    // fermeture ni la réattribution à un autre objet.
    let r = unsafe { libc::dup2(err.as_raw_fd(), out.as_raw_fd()) };
    if r < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(copy)
}

/// La sortie du protocole : le stdout d'origine ; stdout écrit ensuite sur stderr.
pub(crate) fn protocol_output() -> io::Result<std::fs::File> {
    io::stdout().flush()?;
    let copy = divert(io::stdout().as_fd(), io::stderr().as_fd())?;
    Ok(std::fs::File::from(copy))
}

#[cfg(test)]
#[path = "stdio_tests.rs"]
mod stdio_tests;
