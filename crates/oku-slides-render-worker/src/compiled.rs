//! Une génération rendue par le travailleur : SVG, PNG et PDF à la demande.
//!
//! Règles (voir aussi `client`) : un export d'une autre époque répond
//! `Stale` sans appel ; un échec borné accuse le pack (hors essai) ; le
//! `Drop` met `Release` en file.

use std::sync::Arc;

use oku_slides_render::{Diagnostic, Layout, RenderError};

use crate::client::{Deferred, RenderWorker, Shared, blame, unexpected};
use crate::protocol::{CompiledHeader, Request, Response, SlotId};

/// Libère une génération au `Drop` de sa poignée.
///
/// À part de [`RemoteCompiled`] : un type sans `Drop` se construit par mise à
/// jour de structure (`..autre`).
struct Release {
    shared: Arc<Shared>,
    slot: SlotId,
    generation: u64,
    epoch: u64,
}

impl Drop for Release {
    fn drop(&mut self) {
        let (slot, generation, epoch) = (self.slot, self.generation, self.epoch);
        self.shared.defer(Deferred::Release { slot, generation, epoch });
    }
}

/// Une génération rendue par le travailleur ; les pages s'exportent à la demande.
pub struct RemoteCompiled {
    pub warnings: Vec<Diagnostic>,
    /// Un échec borné y accuse le pack ; `false` pour un emplacement d'essai.
    accuse: bool,
    worker: RenderWorker,
    slot: SlotId,
    pub(crate) generation: u64,
    /// Époque du travailleur qui a rendu cette génération.
    epoch: u64,
    hashes: Vec<u128>,
    layout: Layout,
    /// `None` pour une copie de test, qui ne libère rien.
    _release: Option<Release>,
}

impl std::fmt::Debug for RemoteCompiled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteCompiled")
            .field("slot", &self.slot)
            .field("generation", &self.generation)
            .field("pages", &self.hashes.len())
            .field("warnings", &self.warnings)
            .finish_non_exhaustive()
    }
}

impl RemoteCompiled {
    /// La génération que décrit `header`, rendue dans l'emplacement `slot` à l'époque `epoch`.
    ///
    /// Un en-tête invalide est une erreur ; la génération est alors libérée.
    pub(crate) fn from_header(
        worker: RenderWorker,
        slot: SlotId,
        epoch: u64,
        accuse: bool,
        header: CompiledHeader,
    ) -> Result<Self, RenderError> {
        let CompiledHeader { generation, page_hashes, mut layout, tokens, warnings, .. } = header;
        // Créée d'abord : si l'en-tête est invalide, son `Drop` libère la génération.
        let release = Release { shared: worker.1.clone(), slot, generation, epoch };
        let hashes = page_hashes
            .iter()
            .map(|h| u128::from_str_radix(h, 16).map_err(|e| RenderError::frame(format!("page hash {h}: {e}"))))
            .collect::<Result<Vec<_>, _>>()?;
        layout.tokens = tokens;
        Ok(Self { warnings, accuse, worker, slot, generation, epoch, hashes, layout, _release: Some(release) })
    }

    pub fn page_count(&self) -> usize {
        self.hashes.len()
    }

    /// Empreinte d'une page : sert de clé de cache pour son SVG.
    pub fn page_hash(&self, index: usize) -> Option<u128> {
        self.hashes.get(index).copied()
    }

    /// Pages, boîtes, zones et jetons du document.
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// SVG d'une page (à partir de 0).
    pub fn svg(&self, index: usize) -> Result<String, RenderError> {
        let bytes = self.bytes(&Request::Svg { slot: self.slot, generation: self.generation, page: index })?;
        String::from_utf8(bytes).map_err(|e| RenderError::frame(e.to_string()))
    }

    /// PNG de la page `index` (0 = première), à `pixel_per_pt` pixels par point.
    pub fn png(&self, index: usize, pixel_per_pt: f32) -> Result<Vec<u8>, RenderError> {
        self.bytes(&Request::Png { slot: self.slot, generation: self.generation, page: index, pixel_per_pt })
    }

    pub fn pdf(&self) -> Result<Vec<u8>, RenderError> {
        self.bytes(&Request::Pdf { slot: self.slot, generation: self.generation })
    }

    /// Un export de cette génération ; un échec borné accuse le pack (hors essai).
    ///
    /// Une génération d'un travailleur précédent répond `Stale` sans appel.
    fn bytes(&self, req: &Request) -> Result<Vec<u8>, RenderError> {
        let mut link = self.worker.link();
        if link.restarts != self.epoch {
            return Err(RenderError::stale());
        }
        let pack = link.state.slots.get(&self.slot).and_then(|s| s.pack.clone()).filter(|_| self.accuse);
        self.worker.prepare_for(&mut link, pack.as_ref())?;
        // `prepare` a pu relancer le travailleur.
        if link.restarts != self.epoch {
            return Err(RenderError::stale());
        }
        match link.call(req, &[]) {
            Ok((Response::Bytes, payload)) => Ok(payload),
            Ok((Response::Error { error }, _)) => Err(error.into_render()),
            Ok((other, _)) => Err(unexpected(&other)),
            Err(e) => {
                blame(&mut link, pack.as_ref(), &e);
                Err(e)
            }
        }
    }

    /// Copie de la poignée, sans `Release` au `Drop`.
    #[cfg(test)]
    pub(crate) fn clone_handle_for_test(&self) -> Self {
        Self {
            warnings: self.warnings.clone(),
            accuse: self.accuse,
            worker: self.worker.clone(),
            slot: self.slot,
            generation: self.generation,
            epoch: self.epoch,
            hashes: self.hashes.clone(),
            layout: self.layout.clone(),
            _release: None,
        }
    }
}
