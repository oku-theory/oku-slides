//! Versions de pack reçues avec un fichier, et le bandeau qui les signale.
//!
//! À l'ouverture, chaque version embarquée absente de la bibliothèque y est
//! ajoutée en silence (un id inconnu crée le pack). Si la version d'un pack
//! épinglé n'est pas la courante, un bandeau le dit, un pack à la fois (le
//! premier espace de noms, dans l'ordre), avec deux actions :
//! passer le deck à ma version (annulable), ou faire de la sienne ma version
//! courante ; la session le décrit en surface A2UI (`Notice`). Il ne bloque
//! jamais l'ouverture ; fermé, il ne revient plus pour ce deck dans cette
//! session. Un pack du deck seul (`PackScope::Deck`) n'est jamais reçu dans la
//! bibliothèque et n'a pas de bandeau.

use std::sync::atomic::{AtomicBool, Ordering};

use oku_slides_a2ui::{Action, Component, ComponentKind, Dynamic};
use serde_json::json;
use snafu::OptionExt;

use oku_slides_core::{Namespace, Op, PackFiles, PackKind, PackPin};
use oku_slides_i18n::Localizer;

use crate::agent::Surface;
use crate::agent::forms::{Place, surface};
use crate::error::UnknownPackSnafu;
use crate::pack::{PackManifest, Relation};
use crate::{Result, Session};

/// Vrai si la version `files` n'appartient qu'à un deck.
///
/// Une portée illisible ne l'est pas : journalisée, le pack est traité comme de la bibliothèque.
pub(crate) fn is_deck_only(files: &PackFiles) -> bool {
    match files.scope() {
        Ok(scope) => !scope.is_library(),
        Err(e) => {
            tracing::warn!(error = %e, "portée du pack illisible : traité comme de la bibliothèque");
            false
        }
    }
}

/// Les thèmes écartés de la bibliothèque ont déjà été signalés dans le processus.
static THEMES_SKIPPED_SEEN: AtomicBool = AtomicBool::new(false);

/// Le bandeau : la version d'un pack épinglé n'est pas la courante.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PackNotice {
    /// La sorte du pack épinglé.
    pub(crate) kind: PackKind,
    /// L'espace de noms du pack épinglé.
    pub(crate) namespace: Namespace,
    /// Le pack, dans sa version embarquée.
    pub(crate) pin: PackPin,
    pub(crate) name: String,
    /// La version du deck par rapport à ma version courante.
    pub(crate) relation: Relation,
}

/// Surface du bandeau du pack.
const PACK_NOTICE: &str = "host-pack-notice";

/// La surface du bandeau : `message`, « Passer à ma version », puis « Faire de la sienne… » si `make_current`.
///
/// Toujours vivante (`live`) : le mot ne vaut que pour le dock, un bandeau n'y va jamais.
pub(crate) fn pack_notice_banner(message: String, make_current: bool, l: &Localizer) -> Surface {
    let mut buttons =
        vec![Component::button("pack-use-mine", l.tr("pack-notice-use-mine"), true, Action::named("pack_use_mine"))];
    if make_current {
        let label = l.tr("pack-notice-make-current");
        buttons.push(Component::button("pack-make-current", label, false, Action::named("pack_make_current")));
    }
    let root = Component::new(
        "root",
        ComponentKind::Notice {
            text: Dynamic::Literal(message),
            children: buttons.iter().map(|b| b.id.clone()).collect(),
            close: Action::named("pack_notice_close"),
            close_label: Dynamic::Literal(l.tr("pack-notice-close")),
        },
    );
    let components = std::iter::once(root).chain(buttons).collect();
    surface(PACK_NOTICE, Place::Banner, None, components, json!({}))
}

impl Session {
    /// Signale une fois par processus les thèmes de la bibliothèque écartés (`notice-themes-skipped`).
    ///
    /// Un thème d'avant les espaces de noms ne se lit plus : il n'est ni listé ni proposé.
    /// L'avis est gardé (`themes_notice`) pour suivre le document qui remplacerait le deck
    /// vierge avant qu'il soit vu ([`Self::carried_themes_notice`]).
    pub(crate) fn notice_skipped_themes(&mut self) {
        let count = self.themes.skipped();
        if count > 0 && !THEMES_SKIPPED_SEEN.swap(true, Ordering::Relaxed) {
            tracing::warn!(count, "thèmes de la bibliothèque écartés : sans espace de noms ou illisibles");
            let text = self.localizer.tr_with("notice-themes-skipped", &[("count", count.into())]);
            self.themes_notice = Some(text.clone());
            self.set_notice(text);
        }
    }

    /// L'avis des thèmes écartés que le document montré porte encore, pas encore vu : le suivant le reprend.
    ///
    /// Un avis déjà montré (le snapshot le vide) ne revient pas.
    pub(crate) fn carried_themes_notice(&mut self) -> Option<String> {
        let text = self.themes_notice.take()?;
        self.doc.notice.as_deref().is_some_and(|pending| pending.contains(text.as_str())).then_some(text)
    }

    /// Ajoute en silence à la bibliothèque les versions embarquées qu'elle n'a pas.
    ///
    /// Une version déjà connue ou nouvelle d'un pack connu ne change jamais
    /// sa version courante ; un pack inconnu est créé, celle-ci courante.
    /// Une version du deck seul reste dans le deck.
    pub(crate) fn receive_packs(&self) {
        for files in self.doc.sources.packs.values() {
            let id = match PackManifest::from_files(files) {
                Ok(m) if !m.scope.is_library() => continue,
                Ok(m) => m.id,
                Err(e) => {
                    tracing::warn!(version = %files.version(), error = %e, "pack reçu illisible");
                    continue;
                }
            };
            if let Err(e) = self.library.receive_version(id, files) {
                tracing::warn!(pack = %id, error = %e.one_line(), "pack reçu non ajouté à la bibliothèque");
            }
        }
    }

    /// Le bandeau du deck ouvert, s'il en faut un : le premier pack épinglé
    /// (widgets puis layouts, par espace de noms) dont la version n'est pas la courante.
    ///
    /// Jamais pour un pack du deck seul, ni pour un bandeau fermé.
    pub(crate) fn pack_notice_for_deck(&self) -> Option<PackNotice> {
        self.doc.sources.deck().all_pins().find_map(|(kind, ns, pin)| self.pack_notice_for(kind, ns, pin))
    }

    /// Le bandeau du pack `pin` de sorte `kind` épinglé sous `ns`, s'il en faut un.
    fn pack_notice_for(&self, kind: PackKind, ns: &Namespace, pin: &PackPin) -> Option<PackNotice> {
        if self.pack_notices_closed.contains(pin) {
            return None;
        }
        let files = self.doc.sources.packs.get(&pin.version);
        if files.is_some_and(is_deck_only) {
            return None;
        }
        let current = self.library.current(pin.id)?;
        let relation = self.library.relation(pin.id, &pin.version, &current);
        if relation == Relation::Same {
            return None;
        }
        let name = files.and_then(|f| PackManifest::from_files(f).ok()).map(|m| m.name).unwrap_or_default();
        Some(PackNotice { kind, namespace: ns.clone(), pin: pin.clone(), name, relation })
    }

    /// Vrai si le deck épingle un pack qui n'appartient qu'à lui (`PackScope::Deck`) : jamais dans la bibliothèque.
    ///
    /// Lu à chaque instantané : la seule clé `scope`, jamais tout le manifeste.
    pub(crate) fn deck_pack_is_deck_only(&self) -> bool {
        let deck = self.doc.sources.deck();
        deck.all_pins().filter_map(|(_, _, pin)| self.doc.sources.packs.get(&pin.version)).any(is_deck_only)
    }

    /// Vrai si le deck embarque le pack `id` en version du deck seul.
    pub(crate) fn embeds_deck_only_pack(&self, id: uuid::Uuid) -> bool {
        let deck_only = self.user_blobs().packs.values().filter(|f| is_deck_only(f));
        deck_only.filter_map(|f| PackManifest::from_files(f).ok()).any(|m| m.id == id)
    }

    /// Le bandeau, s'il vaut encore pour le pack épinglé dont il parle.
    ///
    /// Le deck a pu changer ce pack depuis l'ouverture (modification,
    /// fusion, restauration) : le bandeau ne parle plus de lui.
    fn live_pack_notice(&self) -> Option<&PackNotice> {
        self.pack_notice.as_ref().filter(|n| self.doc.sources.deck().pins(n.kind).get(&n.namespace) == Some(&n.pin))
    }

    /// Le bandeau à montrer, s'il y en a un : une surface A2UI `Notice`.
    ///
    /// Chaque bouton envoie l'évènement du nom de sa requête : `pack_use_mine`,
    /// `pack_make_current` (absent si la bibliothèque n'a pas la version du deck :
    /// l'action échouerait), `pack_notice_close` pour la croix.
    pub(crate) fn pack_notice_surface(&self) -> Option<Surface> {
        let n = self.live_pack_notice()?;
        let key = match n.relation {
            Relation::Ancestor => "pack-notice-older",
            Relation::Descendant => "pack-notice-newer",
            Relation::Same | Relation::Sibling => "pack-notice-parallel",
        };
        let l = &self.localizer;
        let message = l.tr_with(key, &[("name", n.name.as_str().into())]);
        let library_has_theirs = self.library.version(n.pin.id, &n.pin.version).is_some();
        Some(pack_notice_banner(message, library_has_theirs, l))
    }

    /// « Passer à ma version » : le deck épingle ma version courante (`AddPack`), en une étape annulable.
    pub fn pack_use_mine(&mut self) -> Result<()> {
        let Some(n) = self.live_pack_notice().cloned() else { return Ok(()) };
        let id = n.pin.id;
        let current = self.library.current_version(id).context(UnknownPackSnafu { id })?;
        let pin = PackPin { id, version: current.version, requires: current.manifest.requires };
        let label = self.localizer.tr("pack-notice-use-mine");
        self.apply(&label, vec![Op::AddPack { kind: n.kind, namespace: n.namespace, pin }], None)?;
        self.pack_notice_close();
        Ok(())
    }

    /// « Faire de la sienne ma version courante » : le deck ne change pas.
    pub fn pack_make_current(&mut self) -> Result<()> {
        let Some(n) = self.live_pack_notice().cloned() else { return Ok(()) };
        self.library.set_current(n.pin.id, &n.pin.version)?;
        self.pack_cache.clear();
        self.pack_notice_close();
        Ok(())
    }

    /// Ferme le bandeau ; il ne revient plus pour ce deck dans cette session.
    pub fn pack_notice_close(&mut self) {
        if let Some(n) = self.pack_notice.take() {
            self.pack_notices_closed.insert(n.pin);
        }
    }
}

#[cfg(test)]
#[path = "pack_notice_tests.rs"]
mod pack_notice_tests;
