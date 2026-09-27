//! La session d'un onglet, relue seulement quand on s'en sert.
//!
//! Reprise, un onglet dort ([`Slot::Dormant`]) : sa barre vient de
//! `workspace.json`, sa session n'est relue qu'à sa première requête, sous
//! son seul verrou (jamais celui de l'espace de travail).

use std::path::PathBuf;

use crate::error::{Result, UnknownTabSnafu};
use crate::views::TabInfo;
use crate::{Fonts, Platform, Session};

/// Ce qu'il faut pour relire la session d'un onglet endormi.
pub(super) struct Dormant {
    /// Dossier de reprise de l'onglet (`tabs/<id>`).
    pub(super) dir: PathBuf,
    pub(super) fonts: Fonts,
    pub(super) platform: Platform,
}

impl Dormant {
    /// Relit la session de l'onglet.
    ///
    /// Un état illisible est mis de côté par [`Session::resume`] ; l'onglet
    /// repart d'un deck vierge avec une reprise neuve, et le dit.
    fn wake(&self) -> Result<Session> {
        let mut session = Session::new(self.fonts.clone(), &self.platform)?;
        if !session.resume(self.dir.clone()) {
            if !session.resume_fresh(self.dir.clone()) {
                tracing::warn!(path = %self.dir.display(), "reprise de l'onglet inactive");
            }
            session.set_notice(self.platform.localizer.tr("notice-tab-resume-failed"));
        }
        Ok(session)
    }
}

/// La session d'un onglet : prête, endormie, ou fermée pour de bon.
pub(super) enum Slot {
    Ready(Box<Session>),
    Dormant(Box<Dormant>),
    /// Onglet fermé avant d'avoir été relu : plus jamais réveillé, pour ne
    /// pas réécrire le dossier qu'on vient de supprimer.
    Closed(String),
}

impl Slot {
    /// La session, relue d'abord si l'onglet dort (voir [`Dormant::wake`]).
    ///
    /// Un état illisible ne retire pas l'onglet. Seul l'échec de
    /// [`Session::new`] le laisse endormi : la requête échoue, la suivante
    /// réessaie. Fermé : `UnknownTab`, comme s'il n'existait plus.
    pub(super) fn ready(&mut self) -> Result<&mut Session> {
        match self {
            Self::Ready(session) => Ok(session),
            Self::Dormant(dormant) => {
                let session = dormant.wake()?;
                *self = Self::Ready(Box::new(session));
                self.ready()
            }
            Self::Closed(id) => Err(UnknownTabSnafu { id: id.clone() }.build().into()),
        }
    }

    pub(super) fn is_dormant(&self) -> bool {
        matches!(self, Self::Dormant(_))
    }

    /// Infos à jour d'une session relue ; `None` : endormie ou fermée.
    pub(super) fn info(&self) -> Option<TabInfo> {
        match self {
            Self::Ready(session) => Some(session.tab_info()),
            Self::Dormant(_) | Self::Closed(_) => None,
        }
    }

    /// Attend que la reprise envoyée par la session soit écrite.
    pub(super) fn flush(&self) {
        if let Self::Ready(session) = self {
            session.flush_resume();
        }
    }

    /// Plus rien n'est écrit pour la reprise de l'onglet `id` (il est fermé).
    pub(super) fn stop_resume(&mut self, id: &str) {
        match self {
            Self::Ready(session) => session.stop_resume(),
            Self::Dormant(_) => *self = Self::Closed(id.to_owned()),
            Self::Closed(_) => {}
        }
    }
}
