//! L'élément copié, gardé par l'hôte pour toutes les fenêtres (t-1182).
//!
//! L'interface l'envoie au ⌘C (`SetClip`), l'hôte le numérote, le diffuse aux
//! fenêtres (évènement `clip`) et le rend à une fenêtre qui s'ouvre (`Clip`). Le
//! numéro dit aux fenêtres quelle copie est la dernière, quel que soit l'ordre
//! d'arrivée des évènements. Ses images ne voyagent qu'au collage (`CarryAssets`) ;
//! il n'est jamais enregistré.

use oku_slides_core::Element;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::error::{ResponseSnafu, Result};

use super::Workspace;

/// Un élément copié, avec son onglet, son texte et sa fenêtre d'envoi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    /// Onglet où il a été copié.
    pub tab: String,
    /// Son texte au presse-papiers : encore là, c'est la dernière copie.
    pub text: String,
    /// L'élément tel que la vue de sa slide le donnait.
    pub element: Box<Element>,
    /// Label de la fenêtre qui l'a copié : elle ignore l'écho de sa diffusion.
    ///
    /// Un label de fenêtre, pas une provenance de l'agent.
    pub origin: String,
}

/// La réponse de `Clip` et `SetClip` : le dernier élément copié et son numéro.
#[derive(Serialize)]
struct ClipReply<'a> {
    /// Le dernier élément copié, s'il y en a un.
    clip: Option<&'a Clip>,
    /// Croît à chaque copie ; 0 sans copie.
    seq: u64,
}

impl Workspace {
    /// Le dernier élément copié et son numéro : `{ "clip": … | null, "seq": n }`.
    pub(super) fn clip_json(&self) -> Result<String> {
        let reply = ClipReply { clip: self.clip.as_ref(), seq: self.clip_seq };
        Ok(serde_json::to_string(&reply).context(ResponseSnafu)?)
    }

    /// Retient `clip` à la place du précédent, sous un nouveau numéro.
    pub(super) fn set_clip(&mut self, clip: Clip) {
        self.clip_seq = self.clip_seq.saturating_add(1);
        self.clip = Some(clip);
    }
}
