//! Rendu Typst dans un processus de travail, borné en temps et en mémoire.
//!
//! Le processus de travail reçoit des trames sur son tube (voir `frame`) et rend
//! les decks à la place du processus principal : un rendu qui boucle ou qui
//! consomme trop de mémoire se tue sans emporter l'application.
//!
//! API publique :
//! - [`RenderWorker`] : côté hôte, le travailleur partagé (sondes, packs
//!   fautifs, relance) ; ouvre des emplacements.
//! - [`RenderSlot`] : un emplacement, même surface que `Renderer`.
//! - [`RemoteCompiled`] : une génération rendue ; SVG, PNG et PDF à la demande.
//! - [`serve`] : la boucle du travailleur ([`ServeError`] si le tube est illisible).
//! - [`exit_if_worker`] : à appeler en tête de `main`.

mod client;
mod error;
mod frame;
mod link;
mod protocol;
mod rss;
mod serve;
#[cfg(test)]
pub(crate) mod test_packs;
mod watchdog;

pub use client::{RemoteCompiled, RenderSlot, RenderWorker};
pub use serve::{ServeError, exit_if_worker, serve};
