//! macOS : l'empreinte physique d'un processus (`phys_footprint`, celle du
//! Moniteur d'activité et de la pression mémoire du noyau), mémoire compressée
//! comprise, là où la mémoire résidente l'ignore.
#![expect(unsafe_code, reason = "proc_pid_rusage : appel système sans enveloppe sûre")]

/// Empreinte physique de `pid` en octets ; `None` si le processus n'existe pas
/// (ou n'est pas lisible).
pub(crate) fn phys_footprint(pid: u32) -> Option<u64> {
    let pid = libc::c_int::try_from(pid).ok()?;
    let mut info = std::mem::MaybeUninit::<libc::rusage_info_v4>::zeroed();
    // SAFETY: `info` est un `rusage_info_v4` (aligné, assez grand pour la
    // version `RUSAGE_INFO_V4` demandée) que le noyau remplit ; le pointeur
    // reste valide pendant l'appel. Le type `*mut rusage_info_t` de `libc`
    // (un `void **`) n'est que la convention de l'en-tête C : le noyau écrit
    // la structure à cette adresse.
    let r =
        unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V4, info.as_mut_ptr().cast::<libc::rusage_info_t>()) };
    if r != 0 {
        return None;
    }
    // SAFETY: l'appel a réussi, la structure est remplie ; partie de zéros,
    // elle était de toute façon valide (des entiers seulement).
    let info = unsafe { info.assume_init() };
    Some(info.ri_phys_footprint)
}

#[cfg(test)]
#[path = "footprint_tests.rs"]
mod footprint_tests;
