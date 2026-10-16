//! Rendu Typst dans un processus de travail, borné en temps et en mémoire.
//!
//! Le processus de travail reçoit des trames sur son tube (voir `frame`) et rend
//! les decks à la place du processus principal : un rendu qui boucle ou qui
//! consomme trop de mémoire se tue sans emporter l'application.
//!
//! API publique :
//! - [`RenderWorker`] : côté hôte, le travailleur partagé (sondes, packs
//!   fautifs, relance) ; ouvre des emplacements ; `probe_pack` (sonde par
//!   fichier, sans accusation), `trial_slot` (emplacement d'aperçu).
//! - [`PackProbe`] : la sonde d'un pack, fichier par fichier.
//! - [`RenderSlot`] : un emplacement, même surface que `Renderer`.
//! - [`RemoteCompiled`] : une génération rendue ; SVG, PNG et PDF à la demande.
//! - [`RemoteReference`] : le rendu de référence d'une présentation importée,
//!   fait par le témoin ([`ReferenceWitness`]) dans le travailleur ; PNG de
//!   ses pages à la demande.
//! - [`serve`] : la boucle du travailleur ([`ServeError`] si le tube est illisible).
//! - [`exit_if_worker`] : à appeler en tête de `main`.

mod client;
mod compiled;
mod error;
#[cfg(target_os = "macos")]
mod footprint;
mod frame;
mod link;
mod pack_probe;
mod protocol;
mod reference;
mod rss;
mod serve;
mod slot;
#[cfg(unix)]
mod stdio;
#[cfg(test)]
pub(crate) mod test_packs;
mod watchdog;

pub use client::RenderWorker;
pub use compiled::RemoteCompiled;
pub use pack_probe::PackProbe;
pub use reference::{ReferenceWitness, RemoteReference};
pub use serve::{ServeError, exit_if_worker, serve};
pub use slot::RenderSlot;
