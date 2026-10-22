//! Un deck déjà ouvert : une autre copie du même deck (même `deck_uid`) se
//! reconnaît d'après les infos des onglets, sans réveiller ceux qui dorment.
//!
//! Le fichier se lit hors du verrou de l'espace de travail : pris le temps
//! de copier de quoi lire, rendu, puis repris pour chercher l'onglet.

use std::path::Path;
use std::sync::{Arc, Mutex};

use snafu::ResultExt;

use oku_slides_core::{SourceLimits, WidgetRegistry};
use oku_slides_platform::storage::{Storage, resolve};

use crate::error::{RegistrySnafu, ResponseSnafu, Result};
use crate::files::{deck_uid_of, source_limits};

use super::{Slot, Workspace, WorkspaceOutcome, number, relock};

impl Workspace {
    /// `SameDeck` : l'onglet qui tient déjà le deck de `path`, sous un autre
    /// fichier, et son titre ; `null` sinon. Ne change rien.
    ///
    /// Le fichier déjà ouvert tel quel n'est pas « un autre » : `OpenTab`
    /// active son onglet, sans question. Un fichier illisible est une
    /// erreur (l'interface l'affiche et n'ouvre rien).
    pub(super) fn same_deck(ws: &Mutex<Self>, path: &Path) -> Result<WorkspaceOutcome> {
        let path = resolve(path);
        let found = match Self::reader(ws, &path)? {
            Some((storage, limits, registry)) => {
                // Hors de tout verrou : lecture et décodage du fichier.
                let uid = deck_uid_of(&storage, &path, limits, &registry)?;
                uid.and_then(|uid| Self::lock(ws).tab_of_deck(&uid, &path))
            }
            None => None,
        };
        let (tab, title) = found.map_or((None, None), |(id, title)| (Some(id), Some(title)));
        let reply = serde_json::json!({ "tab": tab, "title": title });
        Ok(WorkspaceOutcome::Applied(serde_json::to_string(&reply).context(ResponseSnafu)?))
    }

    /// De quoi lire `path` hors du verrou ; `None` : déjà ouvert tel quel,
    /// rien à lire.
    /// Le registre intégré suffit : seule l'identité du deck est lue, d'un replica qui connaît `base`.
    fn reader(ws: &Mutex<Self>, path: &Path) -> Result<Option<(Storage, SourceLimits, Arc<WidgetRegistry>)>> {
        let (storage, limits, fonts) = {
            let w = Self::lock(ws);
            if w.tab_with_path(path).is_some() {
                return Ok(None);
            }
            (w.platform.storage.clone(), source_limits(&w.platform.settings), w.fonts.clone())
        };
        // Hors du verrou : son premier appel sonde les widgets intégrés.
        let registry = oku_slides_render::builtin_registry(&fonts).context(RegistrySnafu)?;
        Ok(Some((storage, limits, registry)))
    }

    /// L'onglet non vierge qui tient le deck `uid` sous un autre fichier que
    /// `path`, et son titre ; le plus ancien s'il y en a plusieurs. Un onglet
    /// en mode pack ne compte pas : il refuserait la fusion.
    fn tab_of_deck(&self, uid: &str, path: &Path) -> Option<(String, String)> {
        self.tabs
            .iter()
            .filter(|(_, t)| {
                let i = &t.info;
                !i.pristine && !i.editing_pack && i.path.as_deref() != Some(path) && i.deck_uid.as_deref() == Some(uid)
            })
            .min_by_key(|(id, _)| number(id))
            .map(|(id, t)| (id.clone(), t.info.title.clone()))
    }

    /// Après l'ouverture de `path` dans un nouvel onglet (fichier remis par
    /// le système, sans question possible) : si son deck est déjà ouvert
    /// dans un autre onglet, l'avis `notice-same-deck-open` le désigne.
    ///
    /// L'onglet et l'avis se trouvent sous le verrou de l'espace de travail,
    /// l'avis se pose sous celui de la session seule, verrou de l'espace de
    /// travail rendu.
    pub(super) fn note_same_deck(ws: &Mutex<Self>, path: &Path) {
        let target = {
            let w = Self::lock(ws);
            let Some(id) = w.tab_with_path(path) else { return };
            let uid = w.tabs.get(&id).and_then(|t| t.info.deck_uid.clone());
            let Some((_, title)) = uid.and_then(|uid| w.tab_of_deck(&uid, path)) else { return };
            let text = w.platform.localizer.tr_with("notice-same-deck-open", &[("title", title.into())]);
            w.session(&id).ok().map(|session| (id, session, text))
        };
        if let Some((id, session, text)) = target {
            Self::post_notice(ws, &id, &session, text);
        }
    }

    /// Pose un avis sur la session d'un onglet (relue si elle dormait), puis
    /// met ses infos à jour ; jamais sous le verrou de l'espace de travail.
    pub(super) fn post_notice(ws: &Mutex<Self>, id: &str, session: &Arc<Mutex<Slot>>, text: String) {
        let info = relock(session).ready().map(|s| {
            s.set_notice(text);
            s.tab_info()
        });
        match info {
            Ok(info) => {
                Self::refresh(ws, id, info);
            }
            Err(e) => tracing::warn!(tab = %id, error = %e.one_line(), "avis non posé"),
        }
    }
}
