//! Côté hôte : le travailleur partagé, ses emplacements et leurs rendus.
//!
//! - [`RenderWorker`] : le lien partagé (sondes, packs fautifs, emplacements).
//! - [`RenderSlot`] : un emplacement, même surface que `Renderer`.
//! - [`RemoteCompiled`] : une génération rendue ; SVG, PNG et PDF à la demande.
//!
//! Règles :
//! - Un `Drop` ne prend jamais le verrou du lien : `Release` et `CloseSlot`
//!   vont dans une file à part, envoyée (réponses ignorées) avant la requête
//!   suivante. Un échec du lien pendant cet envoi est rendu à l'appelant.
//! - Époque : le nombre de relances du travailleur (`Link::restarts`). Les
//!   générations repartent de 1 dans un travailleur neuf : chaque
//!   [`RemoteCompiled`] et chaque `Release` en file portent l'époque de leur
//!   rendu. Un `Release` d'une autre époque est jeté sans envoi ; un export
//!   d'une autre époque répond `Stale` sans appel.
//! - Un échec borné (délai, mémoire, arrêt) pendant la sonde d'un pack, ou
//!   pendant une opération d'un emplacement (pose des widgets, rendu, export),
//!   envoi de la file compris, marque le pack fautif : celui posé par
//!   `set_widgets`, celui de l'emplacement pour les autres. Il n'est plus
//!   exécuté dans ce travailleur, et le rendu de l'emplacement répond
//!   `PackDisabled` sans appel.
//! - Assets : l'hôte ne garde, parmi ceux envoyés, que ceux du dernier rendu
//!   de chaque emplacement (même règle que le travailleur).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use oku_slides_core::{Deck, WidgetRegistry};
use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{Diagnostic, Fonts, Layout, PackWidgets, RegistryError, RenderError, registry_from_probe};
use sha2::{Digest, Sha256};

use crate::link::{Link, SlotState, fingerprint_hex};
use crate::protocol::{CompiledHeader, Request, Response, SlotId, WirePack};

/// Une requête mise en file par un `Drop`.
#[derive(Debug)]
enum Deferred {
    /// Libère une génération de l'époque `epoch`.
    Release {
        slot: SlotId,
        generation: u64,
        epoch: u64,
    },
    Close {
        slot: SlotId,
    },
}

/// Ce que partagent les poignées d'un travailleur, hors du verrou du lien.
#[derive(Debug, Default)]
struct Shared {
    /// `Release` et `CloseSlot` mis en file par un `Drop`.
    deferred: Mutex<Vec<Deferred>>,
    /// Registres sondés, par empreinte hexadécimale du pack.
    registries: Mutex<HashMap<String, Arc<WidgetRegistry>>>,
    /// Dernier identifiant d'emplacement attribué.
    last_slot: AtomicU32,
}

impl Shared {
    fn defer(&self, d: Deferred) {
        self.deferred.lock().unwrap_or_else(PoisonError::into_inner).push(d);
    }
}

/// Un travailleur de rendu partagé par les rendus d'une session.
#[derive(Clone)]
pub struct RenderWorker(Arc<Mutex<Link>>, Arc<Shared>);

impl std::fmt::Debug for RenderWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Sans prendre le verrou du lien : un rendu peut le tenir longtemps.
        f.debug_struct("RenderWorker").finish_non_exhaustive()
    }
}

fn wire(pack: &PackWidgets) -> WirePack {
    WirePack { id: pack.id().to_owned(), source: pack.source().to_owned() }
}

fn unexpected(resp: &Response) -> RenderError {
    RenderError::frame(format!("unexpected reply {resp:?}"))
}

/// Une requête qui attend `Ok`.
fn expect_ok(link: &mut Link, req: &Request, payload: &[u8]) -> Result<(), RenderError> {
    match link.call(req, payload)? {
        (Response::Ok, _) => Ok(()),
        (Response::Error { error }, _) => Err(error.into_render()),
        (other, _) => Err(unexpected(&other)),
    }
}

/// Marque `pack` fautif si `e` est un échec borné.
fn blame(link: &mut Link, pack: Option<&WirePack>, e: &RenderError) {
    if let Some(p) = pack
        && e.kind().is_bounded()
    {
        link.faulty_packs.insert(fingerprint_hex(&p.id, &p.source));
    }
}

/// Le pack fautif d'un emplacement, s'il l'est.
fn faulty_pack(link: &Link, slot: SlotId) -> Option<String> {
    let pack = link.state.slots.get(&slot)?.pack.as_ref()?;
    link.faulty_packs.contains(&fingerprint_hex(&pack.id, &pack.source)).then(|| pack.id.clone())
}

/// Ne garde des assets envoyés que ceux du dernier rendu de chaque emplacement.
fn evict_assets(link: &mut Link) {
    let live: HashSet<&String> = link.state.slots.values().flat_map(|s| &s.last_assets).collect();
    let kept: HashSet<String> = link.sent_assets.iter().filter(|id| live.contains(id)).cloned().collect();
    link.sent_assets = kept;
}

impl RenderWorker {
    /// Un travailleur, lancé à la première requête.
    pub fn new(launch: RenderLaunch, settings: RenderSettings) -> Self {
        Self(Arc::new(Mutex::new(Link::new(launch, settings))), Arc::new(Shared::default()))
    }

    /// Le verrou du lien, sans rien envoyer.
    fn link(&self) -> MutexGuard<'_, Link> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Envoie les requêtes différées, puis s'assure d'un travailleur connecté.
    ///
    /// Sans connexion, le travailleur neuf ne connaît ni les générations ni
    /// les emplacements fermés : seul l'état de rejeu est mis à jour.
    fn prepare(&self, link: &mut Link) -> Result<(), RenderError> {
        let deferred = std::mem::take(&mut *self.1.deferred.lock().unwrap_or_else(PoisonError::into_inner));
        let mut closed = false;
        for d in &deferred {
            if let Deferred::Close { slot } = d {
                link.state.slots.remove(slot);
                closed = true;
            }
        }
        if closed {
            evict_assets(link);
        }
        link.drop_if_dead();
        if link.connected() {
            let current = link.restarts;
            for d in deferred {
                let req = match d {
                    Deferred::Release { slot, generation, epoch } if epoch == current => {
                        Request::Release { slot, generations: vec![generation] }
                    }
                    // Une génération d'un travailleur précédent : l'actuel ne la connaît pas.
                    Deferred::Release { .. } => continue,
                    Deferred::Close { slot } => Request::CloseSlot { slot },
                };
                link.call(&req, &[])?;
            }
        }
        link.ensure_connected()
    }

    /// [`Self::prepare`] pour une opération d'un emplacement : un échec borné accuse `pack`.
    fn prepare_for(&self, link: &mut Link, pack: Option<&WirePack>) -> Result<(), RenderError> {
        self.prepare(link).inspect_err(|e| blame(link, pack, e))
    }

    /// Le registre des widgets de `pack` (base surchargée), sonde exécutée dans le travailleur, gardé par empreinte.
    pub fn probe(&self, pack: &PackWidgets) -> Result<Arc<WidgetRegistry>, RegistryError> {
        let key = fingerprint_hex(pack.id(), pack.source());
        let mut link = self.link();
        if link.faulty_packs.contains(&key) {
            return Err(RegistryError::worker(RenderError::pack_disabled(pack.id())));
        }
        if let Some(r) = self.1.registries.lock().unwrap_or_else(PoisonError::into_inner).get(&key) {
            return Ok(r.clone());
        }
        self.prepare(&mut link).map_err(RegistryError::worker)?;
        let wire = wire(pack);
        match link.call(&Request::Probe { pack: wire.clone() }, &[]) {
            Ok((Response::Probe { json }, _)) => {
                let registry = Arc::new(registry_from_probe(json)?);
                self.1.registries.lock().unwrap_or_else(PoisonError::into_inner).insert(key, registry.clone());
                Ok(registry)
            }
            Ok((Response::Error { error }, _)) => Err(error.into_registry()),
            Ok((other, _)) => Err(RegistryError::worker(unexpected(&other))),
            Err(e) => {
                blame(&mut link, Some(&wire), &e);
                Err(RegistryError::worker(e))
            }
        }
    }

    /// `pack` a fait échouer le rendu (délai, mémoire, arrêt) : il n'est plus exécuté dans ce travailleur.
    pub fn is_faulty(&self, pack: &PackWidgets) -> bool {
        self.link().faulty_packs.contains(&fingerprint_hex(pack.id(), pack.source()))
    }

    /// Ouvre un emplacement avec ces polices, les widgets de base et la langue `l`.
    pub fn slot(&self, fonts: &Fonts, l: &Localizer, base: Arc<WidgetRegistry>) -> Result<RenderSlot, RenderError> {
        let mut link = self.link();
        self.prepare(&mut link)?;
        let mut ids = Vec::new();
        for bytes in fonts.files() {
            let id = hex::encode(Sha256::digest(bytes));
            if !link.state.fonts.contains_key(&id) {
                link.state.fonts.insert(id.clone(), Arc::from(bytes));
            }
            if !link.sent_fonts.contains(&id) {
                expect_ok(&mut link, &Request::PutFont { id: id.clone() }, bytes)?;
                link.sent_fonts.insert(id.clone());
            }
            ids.push(id);
        }
        set_locale(&mut link, l)?;
        let slot = self.1.last_slot.fetch_add(1, Ordering::Relaxed) + 1;
        expect_ok(&mut link, &Request::OpenSlot { slot, fonts: ids.clone() }, &[])?;
        link.state.slots.insert(slot, SlotState { fonts: ids, pack: None, last_assets: Vec::new() });
        drop(link);
        Ok(RenderSlot { worker: self.clone(), id: slot, registry: base, chart_palette: Vec::new() })
    }

    /// Tue le travailleur sans toucher à l'état du lien (tests de relance).
    #[doc(hidden)]
    pub fn kill_for_test(&self) {
        self.link().kill();
    }
}

/// Pose la langue `l` dans le travailleur si elle n'y est pas déjà.
fn set_locale(link: &mut Link, l: &Localizer) -> Result<(), RenderError> {
    let locale = l.locale();
    if link.state.locale.as_deref() == Some(locale) {
        return Ok(());
    }
    expect_ok(link, &Request::SetLanguage { locale: locale.to_owned() }, &[])?;
    link.state.locale = Some(locale.to_owned());
    Ok(())
}

/// Un emplacement du travailleur : un rendu, ses widgets et sa langue.
///
/// La langue est celle du travailleur, commune à ses emplacements.
pub struct RenderSlot {
    worker: RenderWorker,
    id: SlotId,
    registry: Arc<WidgetRegistry>,
    chart_palette: Vec<String>,
}

impl std::fmt::Debug for RenderSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderSlot").field("id", &self.id).finish_non_exhaustive()
    }
}

impl RenderSlot {
    /// Widgets rendus : la base surchargée par `pack` (ou seule, sans pack).
    pub fn set_widgets(
        &mut self,
        pack: Option<&PackWidgets>,
        registry: Arc<WidgetRegistry>,
    ) -> Result<(), RenderError> {
        let mut link = self.worker.link();
        let wire = pack.map(wire);
        if let Some(p) = &wire
            && link.faulty_packs.contains(&fingerprint_hex(&p.id, &p.source))
        {
            return Err(RenderError::pack_disabled(p.id.clone()));
        }
        self.worker.prepare_for(&mut link, wire.as_ref())?;
        let req = Request::SetWidgets { slot: self.id, pack: wire.clone() };
        if let Err(e) = expect_ok(&mut link, &req, &[]) {
            blame(&mut link, wire.as_ref(), &e);
            return Err(e);
        }
        if let Some(state) = link.state.slots.get_mut(&self.id) {
            state.pack = wire;
        }
        self.registry = registry;
        Ok(())
    }

    /// Registre des widgets rendus.
    pub fn registry(&self) -> &Arc<WidgetRegistry> {
        &self.registry
    }

    /// Langue des cadres de repli et des libellés des graphiques.
    pub fn set_language(&mut self, l: &Localizer) -> Result<(), RenderError> {
        let mut link = self.worker.link();
        self.worker.prepare(&mut link)?;
        set_locale(&mut link, l)
    }

    /// Rend `deck` sous une nouvelle génération.
    pub fn render(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> Result<RemoteCompiled, RenderError> {
        let mut link = self.worker.link();
        if let Some(pack) = faulty_pack(&link, self.id) {
            return Err(RenderError::pack_disabled(pack));
        }
        let pack = link.state.slots.get(&self.id).and_then(|s| s.pack.clone());
        self.worker.prepare_for(&mut link, pack.as_ref())?;
        for (id, bytes) in assets {
            if !link.sent_assets.contains(id) {
                expect_ok(&mut link, &Request::PutAsset { id: id.clone() }, bytes)?;
                link.sent_assets.insert(id.clone());
            }
        }
        let ids: Vec<String> = assets.keys().cloned().collect();
        let req = Request::Render { slot: self.id, deck: Box::new(deck.clone()), assets: ids.clone() };
        let header = match link.call(&req, &[]) {
            Ok((Response::Compiled(header), _)) => header,
            Ok((Response::Error { error }, _)) => return Err(error.into_render()),
            Ok((other, _)) => return Err(unexpected(&other)),
            Err(e) => {
                blame(&mut link, pack.as_ref(), &e);
                return Err(e);
            }
        };
        // L'époque du travailleur qui a répondu : `call` ne relance pas une connexion ouverte.
        let epoch = link.restarts;
        if let Some(state) = link.state.slots.get_mut(&self.id) {
            state.last_assets = ids;
        }
        evict_assets(&mut link);
        drop(link);
        let CompiledHeader { generation, page_hashes, mut layout, tokens, warnings, chart_palette, .. } = *header;
        // Créée d'abord : si l'en-tête est invalide, son `Drop` libère la génération.
        let release = Release { shared: self.worker.1.clone(), slot: self.id, generation, epoch };
        let hashes = page_hashes
            .iter()
            .map(|h| u128::from_str_radix(h, 16).map_err(|e| RenderError::frame(format!("page hash {h}: {e}"))))
            .collect::<Result<Vec<_>, _>>()?;
        layout.tokens = tokens;
        self.chart_palette = chart_palette;
        Ok(RemoteCompiled {
            warnings,
            worker: self.worker.clone(),
            slot: self.id,
            generation,
            epoch,
            hashes,
            layout,
            _release: Some(release),
        })
    }

    /// Palette des graphiques du dernier rendu (celle du thème du deck).
    pub fn chart_palette(&self) -> &[String] {
        &self.chart_palette
    }
}

impl Drop for RenderSlot {
    /// Met `CloseSlot` en file : envoyé avant la requête suivante.
    fn drop(&mut self) {
        self.worker.1.defer(Deferred::Close { slot: self.id });
    }
}

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

    /// Un export de cette génération ; un échec borné rend le pack de l'emplacement fautif.
    ///
    /// Une génération d'un travailleur précédent répond `Stale` sans appel.
    fn bytes(&self, req: &Request) -> Result<Vec<u8>, RenderError> {
        let mut link = self.worker.link();
        if link.restarts != self.epoch {
            return Err(RenderError::stale());
        }
        let pack = link.state.slots.get(&self.slot).and_then(|s| s.pack.clone());
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

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
