//! Fin d'un groupe de processus : le témoin LibreOffice peut lancer des
//! enfants (`oosplash`, `soffice.bin` sous Linux) qui survivraient à son PID.
#![expect(unsafe_code, reason = "killpg : std ne sait pas tuer un groupe de processus")]

/// Tue (`SIGKILL`) le groupe de processus `pgid`, sans suite s'il n'existe plus.
pub(crate) fn kill_group(pgid: u32) {
    // 0 viserait notre propre groupe et 1 celui d'init : jamais un enfant lancé ici.
    let Ok(pgid) = libc::pid_t::try_from(pgid) else { return };
    if pgid <= 1 {
        return;
    }
    // SAFETY: `killpg` ne touche à aucune mémoire de Rust ; un groupe déjà vide
    // répond ESRCH, ce qui est le but recherché (rien ne survit au témoin).
    unsafe {
        libc::killpg(pgid, libc::SIGKILL);
    }
}
