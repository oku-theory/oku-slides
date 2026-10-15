//! Côté hôte : le travailleur partagé, ses emplacements et leurs rendus.
//!
//! - [`RenderWorker`] : le lien partagé (sondes, packs fautifs, emplacements).
//! - [`RenderSlot`] (`slot`) : un emplacement, même surface que `Renderer`.
//! - [`RemoteCompiled`](crate::RemoteCompiled) (`compiled`) : une génération rendue ; SVG, PNG et
//!   PDF à la demande.
//! - [`RemoteReference`](crate::RemoteReference) (`reference`) : le rendu de référence d'une
//!   présentation importée (témoin) ; PNG de ses pages à la demande.
//!
//! Les règles ci-dessous valent pour les trois.
//!
//! Règles :
//! - Un `Drop` ne prend jamais le verrou du lien : `Release` et `CloseSlot`
//!   vont dans une file à part, envoyée (réponses ignorées) avant la requête
//!   suivante. Un échec du lien pendant cet envoi est rendu à l'appelant.
//! - Époque : le nombre de relances du travailleur (`Link::restarts`). Les
//!   générations repartent de 1 dans un travailleur neuf : chaque
//!   [`RemoteCompiled`](crate::RemoteCompiled) et chaque `Release` en file portent l'époque de leur
//!   rendu. Un `Release` d'une autre époque est jeté sans envoi ; un export
//!   d'une autre époque répond `Stale` sans appel.
//! - Un échec borné (délai, mémoire, arrêt) pendant la sonde d'un pack, ou
//!   pendant une opération d'un emplacement (pose des widgets, rendu, export),
//!   envoi de la file compris, marque le pack fautif : celui posé par
//!   `set_widgets`, celui de l'emplacement pour les autres. Il n'est plus
//!   exécuté dans ce travailleur, et le rendu de l'emplacement répond
//!   `PackDisabled` sans appel.
//! - Un emplacement d'essai (`trial_slot`, aperçu d'un widget en cours
//!   d'écriture) n'accuse aucun pack et pose même un pack fautif. Après une
//!   relance, le rejeu a écarté ce pack fautif : son rendu répond alors
//!   `PackDisabled` plutôt que de rendre sans le widget.
//! - Assets : l'hôte ne garde, parmi ceux envoyés, que ceux du dernier rendu
//!   de chaque emplacement (même règle que le travailleur).
//! - Polices : avant chaque requête (`prepare`), celles qu'aucun emplacement
//!   ouvert n'utilise sont oubliées de l'état de rejeu, et du travailleur par
//!   `DropFonts` : sa mémoire ne grandit pas au fil des polices des decks.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use oku_slides_core::WidgetRegistry;
use oku_slides_i18n::Localizer;
use oku_slides_platform::RenderLaunch;
use oku_slides_platform::config::RenderSettings;
use oku_slides_render::{FileProbeCache, Fonts, PackWidgets, RenderError};
use sha2::{Digest, Sha256};

use crate::link::{Link, SlotState, fingerprint_hex};
use crate::protocol::{Request, Response, SlotId, WirePack};
use crate::slot::RenderSlot;

/// Une requête mise en file par un `Drop`.
#[derive(Debug)]
pub(crate) enum Deferred {
    /// Libère une génération de l'époque `epoch`.
    Release {
        slot: SlotId,
        generation: u64,
        epoch: u64,
    },
    Close {
        slot: SlotId,
    },
    /// Oublie une référence de l'époque `epoch`.
    ReleaseReference {
        reference: u64,
        epoch: u64,
    },
}

/// Ce que partagent les poignées d'un travailleur, hors du verrou du lien.
#[derive(Debug, Default)]
pub(crate) struct Shared {
    /// `Release` et `CloseSlot` mis en file par un `Drop`.
    deferred: Mutex<Vec<Deferred>>,
    /// Dernier identifiant d'emplacement attribué.
    last_slot: AtomicU32,
    /// Sondes de fichiers, par nom et `probe_key` (échecs bornés compris) :
    /// quelques clés par nom, le cache ne grandit pas à l'édition.
    pub(crate) file_probes: Mutex<FileProbeCache>,
    /// Registres assemblés par `probe_pack`, par empreinte du pack réduit aux
    /// valides, du moins au plus récent.
    pub(crate) assembled: Mutex<VecDeque<([u8; 32], Arc<WidgetRegistry>)>>,
    /// Langue des diagnostics de la sonde, et son code : construite une fois par langue.
    pub(crate) probe_localizer: Mutex<Option<(String, Arc<Localizer>)>>,
    /// Nombre de fichiers sondés d'un pack, au plus.
    pub(crate) max_widgets: usize,
    /// Taille d'un fichier sondé, au plus (octets).
    pub(crate) max_widget_bytes: usize,
    /// Durée totale des sondes d'un pack, au plus (`probe_budget_ms`).
    pub(crate) probe_budget: std::time::Duration,
}

impl Shared {
    pub(crate) fn defer(&self, d: Deferred) {
        self.deferred.lock().unwrap_or_else(PoisonError::into_inner).push(d);
    }
}

/// Un travailleur de rendu partagé par les rendus d'une session.
#[derive(Clone)]
pub struct RenderWorker(pub(crate) Arc<Mutex<Link>>, pub(crate) Arc<Shared>);

impl std::fmt::Debug for RenderWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Sans prendre le verrou du lien : un rendu peut le tenir longtemps.
        f.debug_struct("RenderWorker").finish_non_exhaustive()
    }
}

pub(crate) fn wire(pack: &PackWidgets) -> WirePack {
    WirePack { id: pack.id().to_owned(), files: pack.files().iter().map(|(n, s)| (n.to_string(), s.clone())).collect() }
}

pub(crate) fn unexpected(resp: &Response) -> RenderError {
    RenderError::frame(format!("unexpected reply {resp:?}"))
}

/// Une requête qui attend `Ok`.
pub(crate) fn expect_ok(link: &mut Link, req: &Request, payload: &[u8]) -> Result<(), RenderError> {
    match link.call(req, payload)? {
        (Response::Ok, _) => Ok(()),
        (Response::Error { error }, _) => Err(error.into_render()),
        (other, _) => Err(unexpected(&other)),
    }
}

/// Marque `pack` fautif si `e` est un échec borné.
pub(crate) fn blame(link: &mut Link, pack: Option<&WirePack>, e: &RenderError) {
    if let Some(p) = pack
        && e.kind().is_bounded()
    {
        link.faulty_packs.insert(fingerprint_hex(p));
    }
}

/// Le pack fautif d'un emplacement, s'il l'est.
pub(crate) fn faulty_pack(link: &Link, slot: SlotId) -> Option<String> {
    let pack = link.state.slots.get(&slot)?.pack.as_ref()?;
    link.faulty_packs.contains(&fingerprint_hex(pack)).then(|| pack.id.clone())
}

/// Ne garde des assets envoyés que ceux du dernier rendu de chaque emplacement.
pub(crate) fn evict_assets(link: &mut Link) {
    let live: HashSet<&String> = link.state.slots.values().flat_map(|s| &s.last_assets).collect();
    let kept: HashSet<String> = link.sent_assets.iter().filter(|id| live.contains(id)).cloned().collect();
    link.sent_assets = kept;
}

/// Oublie les polices qu'aucun emplacement ouvert n'utilise : de l'état de
/// rejeu, et du travailleur connecté (`DropFonts`) pour celles qu'il a reçues.
pub(crate) fn evict_fonts(link: &mut Link) -> Result<(), RenderError> {
    let live: HashSet<&String> = link.state.slots.values().flat_map(|s| &s.fonts).collect();
    let dead: Vec<String> = link.state.fonts.keys().filter(|id| !live.contains(id)).cloned().collect();
    for id in &dead {
        link.state.fonts.remove(id);
    }
    let sent: Vec<String> = dead.into_iter().filter(|id| link.sent_fonts.remove(id)).collect();
    if sent.is_empty() || !link.connected() {
        return Ok(());
    }
    expect_ok(link, &Request::DropFonts { ids: sent }, &[])
}

impl RenderWorker {
    /// Un travailleur, lancé à la première requête.
    pub fn new(launch: RenderLaunch, settings: RenderSettings) -> Self {
        let shared = Shared {
            max_widgets: settings.max_widgets,
            max_widget_bytes: settings.max_widget_bytes(),
            probe_budget: settings.probe_budget(),
            ..Shared::default()
        };
        Self(Arc::new(Mutex::new(Link::new(launch, settings))), Arc::new(shared))
    }

    /// Le verrou du lien, sans rien envoyer.
    pub(crate) fn link(&self) -> MutexGuard<'_, Link> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Envoie les requêtes différées, puis s'assure d'un travailleur connecté.
    ///
    /// Sans connexion, le travailleur neuf ne connaît ni les générations ni
    /// les emplacements fermés : seul l'état de rejeu est mis à jour.
    pub(crate) fn prepare(&self, link: &mut Link) -> Result<(), RenderError> {
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
                    Deferred::ReleaseReference { reference, epoch } if epoch == current => {
                        Request::ReleaseReference { reference }
                    }
                    Deferred::ReleaseReference { .. } => continue,
                };
                link.call(&req, &[])?;
            }
        }
        evict_fonts(link)?;
        link.ensure_connected()
    }

    /// [`Self::prepare`] pour une opération d'un emplacement : un échec borné accuse `pack`.
    pub(crate) fn prepare_for(&self, link: &mut Link, pack: Option<&WirePack>) -> Result<(), RenderError> {
        self.prepare(link).inspect_err(|e| blame(link, pack, e))
    }

    /// Un travailleur a déjà été lancé (la première requête le lance) : pour les tests.
    #[doc(hidden)]
    pub fn is_started(&self) -> bool {
        self.link().started()
    }

    /// `pack` a fait échouer le rendu (délai, mémoire, arrêt) : il n'est plus exécuté dans ce travailleur.
    pub fn is_faulty(&self, pack: &PackWidgets) -> bool {
        self.link().faulty_packs.contains(&hex::encode(pack.fingerprint()))
    }

    /// Ouvre un emplacement avec ces polices, les widgets de base et la langue `l`.
    pub fn slot(&self, fonts: &Fonts, l: &Localizer, base: Arc<WidgetRegistry>) -> Result<RenderSlot, RenderError> {
        self.open_slot(fonts, l, base, true)
    }

    /// Ouvre un emplacement d'essai : aucun échec n'y accuse un pack.
    ///
    /// Pour l'aperçu d'un widget en cours d'écriture : le pack n'est pas
    /// écarté, et un pack déjà fautif y est tout de même posé.
    pub fn trial_slot(
        &self,
        fonts: &Fonts,
        l: &Localizer,
        base: Arc<WidgetRegistry>,
    ) -> Result<RenderSlot, RenderError> {
        self.open_slot(fonts, l, base, false)
    }

    /// Ouvre un emplacement ; `accuse` : un échec borné y accuse le pack.
    fn open_slot(
        &self,
        fonts: &Fonts,
        l: &Localizer,
        base: Arc<WidgetRegistry>,
        accuse: bool,
    ) -> Result<RenderSlot, RenderError> {
        let mut link = self.link();
        self.prepare(&mut link)?;
        let ids = put_fonts(&mut link, fonts)?;
        set_locale(&mut link, l)?;
        let slot = self.1.last_slot.fetch_add(1, Ordering::Relaxed) + 1;
        expect_ok(&mut link, &Request::OpenSlot { slot, fonts: ids.clone() }, &[])?;
        link.state.slots.insert(slot, SlotState { fonts: ids, pack: None, last_assets: Vec::new(), theme: None });
        drop(link);
        Ok(RenderSlot::opened(self.clone(), slot, base, accuse))
    }

    /// Tue le travailleur sans toucher à l'état du lien (tests de relance).
    #[doc(hidden)]
    pub fn kill_for_test(&self) {
        self.link().kill();
    }
}

/// Envoie au travailleur celles de `fonts` qu'il n'a pas ; leurs identifiants (empreintes).
pub(crate) fn put_fonts(link: &mut Link, fonts: &Fonts) -> Result<Vec<String>, RenderError> {
    let mut ids = Vec::new();
    for bytes in fonts.files() {
        let id = hex::encode(Sha256::digest(bytes));
        if !link.state.fonts.contains_key(&id) {
            link.state.fonts.insert(id.clone(), Arc::from(bytes));
        }
        if !link.sent_fonts.contains(&id) {
            expect_ok(link, &Request::PutFont { id: id.clone() }, bytes)?;
            link.sent_fonts.insert(id.clone());
        }
        ids.push(id);
    }
    Ok(ids)
}

/// Pose la langue `l` dans le travailleur si elle n'y est pas déjà.
pub(crate) fn set_locale(link: &mut Link, l: &Localizer) -> Result<(), RenderError> {
    let locale = l.locale();
    if link.state.locale.as_deref() == Some(locale) {
        return Ok(());
    }
    expect_ok(link, &Request::SetLanguage { locale: locale.to_owned() }, &[])?;
    link.state.locale = Some(locale.to_owned());
    Ok(())
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

#[cfg(test)]
#[path = "client_fonts_tests.rs"]
mod client_fonts_tests;
