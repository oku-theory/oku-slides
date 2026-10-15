//! Le rendu de référence d'une présentation importée, fait par le témoin dans
//! le travailleur (`oku_slides_reference::Office2Pdf`) : le fichier non fiable
//! y est lu sous les bornes du rendu (délai, mémoire).
//!
//! Règles (voir aussi `client`) : aucun pack n'est accusé (il n'y en a pas) ;
//! une référence d'une autre époque répond `Stale` sans appel ; le `Drop` met
//! `ReleaseReference` en file.

use oku_slides_reference::SlideSpan;
use oku_slides_render::{Fonts, RenderError};

use crate::client::{Deferred, RenderWorker, put_fonts, unexpected};
use crate::protocol::{Request, Response};

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
    /// Le rendu de référence de `pptx` par le témoin, avec `fonts` en priorité.
    ///
    /// `slides` : la tranche à rendre (numéros source, masquées comprises), ou toutes.
    pub fn reference(
        &self,
        pptx: &[u8],
        fonts: &Fonts,
        slides: Option<SlideSpan>,
    ) -> Result<RemoteReference, RenderError> {
        let mut link = self.link();
        self.prepare(&mut link)?;
        let ids = put_fonts(&mut link, fonts)?;
        let epoch = link.restarts;
        let slides = slides.map(|s| (s.first, s.last));
        match link.call(&Request::Reference { fonts: ids, slides }, pptx)? {
            (Response::Reference { reference, witness, pages }, _) => {
                Ok(RemoteReference { worker: self.clone(), reference, epoch, witness, pages })
            }
            (Response::Error { error }, _) => Err(error.into_render()),
            (other, _) => Err(unexpected(&other)),
        }
    }
}

impl RemoteReference {
    /// Nom et version du témoin (`office2pdf 0.8.1`).
    pub fn witness(&self) -> &str {
        &self.witness
    }

    /// Nombre de pages.
    pub fn page_count(&self) -> usize {
        self.pages
    }

    /// PNG de la page `index` (0 = première), à `width` pixels de large.
    ///
    /// Une référence d'un travailleur précédent répond `Stale` sans appel.
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
