//! Les assets d'un élément collé passent d'un onglet à l'autre (t-1178).
//!
//! Les deux sessions sont prises l'une après l'autre, jamais ensemble, et
//! jamais sous le verrou de l'espace de travail ([`Workspace::with_session`]).

use std::sync::Mutex;

use oku_slides_core::Element;
use snafu::ResultExt;

use crate::error::{ResponseSnafu, Result};

use super::{Workspace, WorkspaceOutcome, relock};

impl Workspace {
    /// `CarryAssets` : les assets que cite `element` passent de l'onglet `from` à `to`.
    ///
    /// `from` n'est lu que s'il est prêt : jamais réveillé ni rendu pour lire des
    /// octets, un rendu en échec n'y bloque donc pas le collage. Fermé, inconnu ou
    /// endormi, il n'apporte rien : le collage dans `to` refusera alors un asset qui
    /// lui manque (`UnknownAsset`). `to` inconnu est une erreur. Répond `{ "carried": n }`.
    pub(super) fn carry_assets(ws: &Mutex<Self>, from: &str, to: &str, element: &Element) -> Result<WorkspaceOutcome> {
        let assets = Self::ready_assets(ws, from, element);
        // `to` est l'onglet actif, déjà prêt, et `carry_assets` ne change pas ses infos : rien à redessiner.
        let carried = Self::with_session(ws, to, |s| s.carry_assets(assets)).0?;
        let reply = serde_json::json!({ "carried": carried });
        Ok(WorkspaceOutcome::Applied(serde_json::to_string(&reply).context(ResponseSnafu)?))
    }

    /// Les octets des assets de `element` dans l'onglet `tab`, s'il est prêt ; rien sinon.
    fn ready_assets(ws: &Mutex<Self>, tab: &str, element: &Element) -> Vec<Vec<u8>> {
        // Le verrou de l'espace de travail tombe avant celui de la session.
        let Ok(session) = Self::lock(ws).session(tab) else {
            return Vec::new();
        };
        // Un onglet ne s'endort qu'à la reprise : celui où l'élément vient d'être copié est prêt.
        relock(&session).ready_session().map(|s| s.element_assets(element)).unwrap_or_default()
    }
}
