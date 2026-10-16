//! Rendu de référence d'une présentation importée : un témoin indépendant
//! d'Oku rend le `.pptx` en PDF, dont les pages sont rastérisées pour être
//! comparées au rendu Oku (écart visuel du hi-fi, `oku_slides_import::metrics`).
//!
//! - [`ReferenceRenderer`] : un témoin (`.pptx` + polices → PDF, une tranche
//!   de slides [`SlideSpan`] ou toutes) : [`Office2Pdf`] (crate `office2pdf`,
//!   Rust pur, sur Typst ; [`WITNESS`], son nom) ou [`Soffice`] (LibreOffice,
//!   programme externe borné par son délai ; [`find_soffice`] le trouve).
//! - [`ReferencePdf`] : le PDF rendu, ses pages, rastérisées par `hayro`
//!   ([`ReferencePdf::raster`]) à une largeur donnée.
//! - [`Raster`] : une image RGBA 8 bits opaque ; [`Raster::from_png`],
//!   [`Raster::to_png`] (le tube du processus de rendu transporte des PNG),
//!   [`Raster::crop`].
//! - [`ReferenceError`], [`ReferenceErrorKind`] : opaques ; jamais fatals pour
//!   l'import, qui dit au rapport que la référence manque.
//!
//! Le témoin lit un fichier non fiable sans borne de temps ni de mémoire :
//! l'appelant le fait tourner dans le processus de rendu
//! (`oku-slides-render-worker`), qui le borne.

mod error;
#[cfg(unix)]
mod group;
mod office2pdf;
mod pdf;
mod process;
mod raster;
mod soffice;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use error::{ReferenceError, ReferenceErrorKind};
pub use office2pdf::{Office2Pdf, WITNESS};
pub use pdf::{MAX_RASTER_SIDE, ReferencePdf};
pub use raster::Raster;
pub use soffice::{SOFFICE_WITNESS, Soffice, find_soffice};

/// Un témoin : rend une présentation en PDF, une page par slide.
///
/// Implement this trait to add a reference witness (LibreOffice, PowerPoint…).
/// Le PDF rendu doit garder les slides masquées (l'import les reprend
/// visibles) et utiliser les polices fournies en priorité : l'écart mesure la
/// mise en page, pas la substitution des polices.
pub trait ReferenceRenderer: std::fmt::Debug + Send + Sync {
    /// Nom et version du témoin, tel que le rapport l'écrit (`office2pdf 0.8.1`).
    fn witness(&self) -> &str;

    /// Le PDF de `pptx` rendu avec `fonts`, ses slides `slides` ou toutes.
    ///
    /// `fonts` : octets de fichiers de police, cédés au témoin (il les garde le
    /// temps du rendu). Une page par slide de la tranche, dans l'ordre.
    fn render(
        &self,
        pptx: &[u8],
        fonts: Vec<Vec<u8>>,
        slides: Option<SlideSpan>,
    ) -> Result<ReferencePdf, ReferenceError>;
}

/// Une tranche de slides : numéros source de la première et de la dernière (1.., inclus).
///
/// Les slides masquées comptent : la tranche suit les numéros de la présentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlideSpan {
    /// Numéro de la première slide (1..).
    pub first: u32,
    /// Numéro de la dernière slide, incluse.
    pub last: u32,
}
