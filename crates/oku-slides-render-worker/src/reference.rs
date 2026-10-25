//! Le rendu de référence d'une présentation importée, fait par le témoin dans
//! le travailleur ([`ReferenceWitness`] : `office2pdf` ou LibreOffice) : le
//! fichier non fiable y est lu sous les bornes du rendu (délai, mémoire ;
//! LibreOffice, programme externe, sous son seul délai, plus court).
//!
//! Règles (voir aussi `client`) : aucun pack n'est accusé (il n'y en a pas) ;
//! une référence d'une autre époque répond `Stale` sans appel ; le `Drop` met
//! `ReleaseReference` en file.

use std::path::PathBuf;

use oku_slides_reference::{SOFFICE_WITNESS, SlideSpan, WITNESS};
use oku_slides_render::{Fonts, RenderError};
use serde::{Deserialize, Serialize};

use crate::client::{Deferred, RenderWorker, put_fonts, unexpected};
use crate::protocol::{Request, Response};

/// Le témoin d'un rendu de référence.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ReferenceWitness {
    /// `oku_slides_reference::Office2Pdf`, dans le travailleur.
    #[default]
    Office2Pdf,
    /// LibreOffice (`oku_slides_reference::Soffice`), lancé par le travailleur.
    Soffice {
        /// Le programme `soffice`.
        program: PathBuf,
        /// Délai du rendu d'une tranche (ms), version comprise ; passé lui, `soffice` est tué.
        timeout_ms: u64,
    },
}

impl ReferenceWitness {
    /// Le nom du témoin, tant que sa version n'est pas connue.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Office2Pdf => WITNESS,
            Self::Soffice { .. } => SOFFICE_WITNESS,
        }
    }
}

/// Le rendu de référence d'une présentation, gardé par le travailleur.
pub struct RemoteReference {
    worker: RenderWorker,
    reference: u64,
    /// Époque du travailleur qui l'a rendu.
    epoch: u64,
    witness: String,
    pages: usize,
}

impl std::fmt::Debug for RemoteReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteReference")
            .field("reference", &self.reference)
            .field("witness", &self.witness)
            .field("pages", &self.pages)
            .finish_non_exhaustive()
    }
}

impl RenderWorker {
    /// The reference render of `pptx` by `witness`, with `fonts` taking priority.
    ///
    /// `slides`: the range to render (source numbers, hidden slides included), or all of them.
    ///
    /// # Errors
    ///
    /// Fails if the witness fails, if a font is rejected, or if the worker does not start, dies or exceeds its bounds.
    pub fn reference(
        &self,
        witness: &ReferenceWitness,
        pptx: &[u8],
        fonts: &Fonts,
        slides: Option<SlideSpan>,
    ) -> Result<RemoteReference, RenderError> {
        let mut link = self.link();
        self.prepare(&mut link)?;
        let ids = put_fonts(&mut link, fonts)?;
        let epoch = link.restarts;
        let slides = slides.map(|s| (s.first, s.last));
        let request = Request::Reference { witness: witness.clone(), fonts: ids, slides };
        match link.call(&request, pptx)? {
            (Response::Reference { reference, witness, pages }, _) => {
                Ok(RemoteReference { worker: self.clone(), reference, epoch, witness, pages })
            }
            (Response::Error { error }, _) => Err(error.into_render()),
            (other, _) => Err(unexpected(&other)),
        }
    }
}

impl RemoteReference {
    /// Nom et version du témoin (`office2pdf 0.8.1`, `LibreOffice 26.8.1.1`).
    #[must_use]
    pub fn witness(&self) -> &str {
        &self.witness
    }

    /// Nombre de pages.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages
    }

    /// PNG of page `index` (0 = first), `width` pixels wide.
    ///
    /// A reference from a previous worker answers `Stale` without a call.
    ///
    /// # Errors
    ///
    /// Fails (`Stale`) if the reference comes from a previous worker, if the page
    /// does not exist or cannot be rasterized, or if the worker does not start, dies or exceeds its bounds.
    pub fn png(&self, index: usize, width: u32) -> Result<Vec<u8>, RenderError> {
        let mut link = self.worker.link();
        if link.restarts != self.epoch {
            return Err(RenderError::stale());
        }
        self.worker.prepare(&mut link)?;
        if link.restarts != self.epoch {
            return Err(RenderError::stale());
        }
        match link.call(&Request::ReferencePng { reference: self.reference, page: index, width }, &[])? {
            (Response::Bytes, png) => Ok(png),
            (Response::Error { error }, _) => Err(error.into_render()),
            (other, _) => Err(unexpected(&other)),
        }
    }
}

impl Drop for RemoteReference {
    fn drop(&mut self) {
        self.worker.1.defer(Deferred::ReleaseReference { reference: self.reference, epoch: self.epoch });
    }
}

#[cfg(test)]
#[path = "reference_tests.rs"]
mod reference_tests;
