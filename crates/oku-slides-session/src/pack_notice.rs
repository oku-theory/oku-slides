//! Versions de pack reçues avec un fichier, et le bandeau qui les signale.
//!
//! À l'ouverture, chaque version embarquée absente de la bibliothèque y est
//! ajoutée en silence (un id inconnu crée le pack). Si la version du deck
//! n'est pas la courante de son pack, un bandeau le dit, avec deux actions :
//! passer le deck à ma version (annulable), ou faire de la sienne ma version
//! courante. Il ne bloque jamais l'ouverture ; fermé, il ne revient plus
//! pour ce deck dans cette session.

use serde::Serialize;
use snafu::OptionExt;
use uuid::Uuid;

use oku_slides_core::{Op, PackRef};

use crate::error::UnknownPackSnafu;
use crate::pack::{PackManifest, Relation};
use crate::{Result, Session};

/// Le bandeau : la version du deck n'est pas la courante de son pack.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PackNotice {
    /// Le pack du deck, dans sa version embarquée.
    pub(crate) pack: PackRef,
    pub(crate) name: String,
    /// La version du deck par rapport à ma version courante.
    pub(crate) relation: Relation,
}

/// Le bandeau tel que l'interface le montre, dans la langue de l'utilisateur.
#[derive(Debug, Clone, Serialize)]
pub struct PackNoticeView {
    pub pack: Uuid,
    pub name: String,
    pub relation: Relation,
    pub message: String,
    /// Libellé de « Passer à ma version ».
    pub use_mine: String,
    /// Libellé de « Faire de la sienne ma version courante » ; `None` si la
    /// bibliothèque n'a pas la version du deck (l'action échouerait).
    pub make_current: Option<String>,
}

impl Session {
    /// Ajoute en silence à la bibliothèque les versions embarquées qu'elle n'a pas.
    ///
    /// Une version déjà connue ou nouvelle d'un pack connu ne change jamais
    /// sa version courante ; un pack inconnu est créé, celle-ci courante.
    pub(crate) fn receive_packs(&self) {
        for files in self.doc.sources.packs.values() {
            let id = match PackManifest::from_files(files) {
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

    /// Le bandeau du deck ouvert, s'il en faut un.
    pub(crate) fn pack_notice_for_deck(&self) -> Option<PackNotice> {
        let r = self.doc.sources.deck().templates.clone()?;
        if self.pack_notices_closed.contains(&r) {
            return None;
        }
        let current = self.library.current(r.id)?;
        let relation = self.library.relation(r.id, &r.version, &current);
        if relation == Relation::Same {
            return None;
        }
        let name = self
            .doc
            .sources
            .packs
            .get(&r.version)
            .and_then(|f| PackManifest::from_files(f).ok())
            .map(|m| m.name)
            .unwrap_or_default();
        Some(PackNotice { pack: r, name, relation })
    }

    /// Le bandeau, s'il vaut encore pour le pack du deck.
    ///
    /// Le deck a pu changer de pack depuis l'ouverture (modification,
    /// fusion, restauration) : le bandeau ne parle plus de lui.
    fn live_pack_notice(&self) -> Option<&PackNotice> {
        self.pack_notice.as_ref().filter(|n| self.doc.sources.deck().templates.as_ref() == Some(&n.pack))
    }

    /// Le bandeau à montrer, s'il y en a un.
    pub(crate) fn pack_notice_view(&self) -> Option<PackNoticeView> {
        let n = self.live_pack_notice()?;
        let key = match n.relation {
            Relation::Ancestor => "pack-notice-older",
            Relation::Descendant => "pack-notice-newer",
            Relation::Same | Relation::Sibling => "pack-notice-parallel",
        };
        let l = &self.localizer;
        Some(PackNoticeView {
            pack: n.pack.id,
            name: n.name.clone(),
            relation: n.relation,
            message: l.tr_with(key, &[("name", n.name.as_str().into())]),
            use_mine: l.tr("pack-notice-use-mine"),
            make_current: self
                .library
                .version(n.pack.id, &n.pack.version)
                .is_some()
                .then(|| l.tr("pack-notice-make-current")),
        })
    }

    /// « Passer à ma version » : le deck passe à ma version courante, en une étape annulable.
    pub fn pack_use_mine(&mut self) -> Result<()> {
        let Some(n) = self.live_pack_notice().cloned() else { return Ok(()) };
        let id = n.pack.id;
        let version = self.library.current(id).context(UnknownPackSnafu { id })?;
        let label = self.localizer.tr("pack-notice-use-mine");
        self.apply(&label, vec![Op::SetTemplates { templates: Some(PackRef { id, version }) }], None)?;
        self.pack_notice_close();
        Ok(())
    }

    /// « Faire de la sienne ma version courante » : le deck ne change pas.
    pub fn pack_make_current(&mut self) -> Result<()> {
        let Some(n) = self.live_pack_notice().cloned() else { return Ok(()) };
        self.library.set_current(n.pack.id, &n.pack.version)?;
        self.template_cache = None;
        self.pack_notice_close();
        Ok(())
    }

    /// Ferme le bandeau ; il ne revient plus pour ce deck dans cette session.
    pub fn pack_notice_close(&mut self) {
        if let Some(n) = self.pack_notice.take() {
            self.pack_notices_closed.insert(n.pack);
        }
    }
}

#[cfg(test)]
#[path = "pack_notice_tests.rs"]
mod pack_notice_tests;
