//! La session d'un onglet, relue seulement quand on s'en sert.
//!
//! Reprise, un onglet dort ([`Slot::Dormant`]) : sa barre vient de
//! `workspace.json`, sa session n'est relue qu'à sa première requête, sous
//! son seul verrou (jamais celui de l'espace de travail).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::error::{Result, UnknownTabSnafu};
use crate::resume::Resumed;
use crate::views::TabInfo;
use crate::{Fonts, Platform, Session, TabDocument};

/// Ce qu'il faut pour relire la session d'un onglet endormi.
pub(super) struct Dormant {
    /// Dossier de reprise de l'onglet (`tabs/<id>`).
    pub(super) dir: PathBuf,
    /// Son titre enregistré, que nomme l'avis d'un état mis de côté.
    pub(super) title: String,
    /// The document `workspace.json` cached for it: a theme tab must hold it.
    pub(super) document: TabDocument,
    pub(super) fonts: Fonts,
    pub(super) platform: Platform,
}

impl Dormant {
    /// Relit la session de l'onglet.
    ///
    /// Un état illisible est mis de côté par [`Session::resume`] ; l'onglet
    /// repart d'un deck vierge avec une reprise neuve, et le dit. Un état d'une
    /// version plus ancienne qui avait du travail non enregistré le dit en nommant l'onglet.
    fn wake(&self) -> Result<Session> {
        let mut session = super::tab_session(&self.fonts, &self.platform)?;
        let notice = match session.resume_checked(self.dir.clone(), Some(&self.document)) {
            Resumed::Restored => return Ok(session),
            Resumed::Unreadable => self.platform.localizer.tr("notice-tab-resume-failed"),
            Resumed::Outdated => {
                self.platform.localizer.tr_with("notice-tab-set-aside", &[("name", self.title.as_str().into())])
            }
        };
        if !session.resume_fresh(self.dir.clone()) {
            tracing::warn!(path = %self.dir.display(), "reprise de l'onglet inactive");
        }
        session.set_notice(notice);
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
    /// La session, relue d'abord si l'onglet dort (voir [`Dormant::wake`]),
    /// et rendue si elle ne l'a jamais été (démarrage paresseux, `render_fonts`).
    ///
    /// Un état illisible ne retire pas l'onglet. Seul l'échec de
    /// [`Session::unrendered`] le laisse endormi : la requête échoue, la suivante
    /// réessaie. Un premier rendu qui échoue laisse l'onglet prêt mais sans
    /// rendu : la requête échoue, et chacune des suivantes refait ce rendu.
    /// Fermé : `UnknownTab`, comme s'il n'existait plus.
    pub(super) fn ready(&mut self) -> Result<&mut Session> {
        match self {
            Self::Ready(session) => {
                session.ensure_rendered()?;
                Ok(session)
            }
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

    /// La session si elle est prête, sans la relire ni la rendre ; `None` : endormie ou fermée.
    pub(super) fn ready_session(&self) -> Option<&Session> {
        match self {
            Self::Ready(session) => Some(session),
            Self::Dormant(_) | Self::Closed(_) => None,
        }
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

    /// Nothing more is written for the resume of tab `id`: it is closed for good.
    ///
    /// A ready session stops its resume, then goes (its lease with it): a stale
    /// `with_session` holding this slot then gets `UnknownTab` instead of writing.
    pub(super) fn stop_resume(&mut self, id: &str) {
        match self {
            Self::Ready(session) => {
                session.stop_resume();
                *self = Self::Closed(id.to_owned());
            }
            Self::Dormant(_) => *self = Self::Closed(id.to_owned()),
            Self::Closed(_) => {}
        }
    }
}

pub(super) struct Tab {
    pub(super) session: Arc<Mutex<Slot>>,
    /// Dernières infos connues : la vue se construit sans verrouiller les sessions.
    pub(super) info: TabInfo,
    /// The deck tab that opened this pack tab (memory only).
    pub(super) opener: Option<String>,
}

impl Tab {
    pub(super) fn new(session: Session) -> Self {
        let info = session.tab_info();
        Self { info, session: Arc::new(Mutex::new(Slot::Ready(Box::new(session)))), opener: None }
    }

    /// Un onglet repris qui dort : sa session sera relue à sa première requête.
    pub(super) fn dormant(info: TabInfo, dormant: Dormant) -> Self {
        Self { info, session: Arc::new(Mutex::new(Slot::Dormant(Box::new(dormant)))), opener: None }
    }
}
