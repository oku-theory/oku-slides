//! Le pack enfant d'un retrofit, créé à la première écriture d'un widget du deck.
//!
//! Un deck de retrofit garde le pack cible de la bibliothèque (atelier
//! ouvert, bandeau des versions) tant qu'aucun widget ne s'y écrit. La
//! première écriture en mode deck sur ce pack, de l'agent ou de
//! l'interface, va à sa place dans un pack du deck seul : les fichiers de
//! la version cible sous un id tiré du `.pptx` et de cette version
//! ([`child_id`]), de parente la version cible, nommé « <pack> pour
//! <deck> », avec le widget écrit. Une seule version et la séance de
//! widgets ordinaire (`widgets_deck.rs`) : une seule étape d'annulation, que
//! l'annulation défait en rendant le pack cible. La bibliothèque et le pack
//! cible ne changent jamais.
//!
//! L'état de l'import ([`super::ImportState`]) garde ce pack à venir tant
//! que vit l'onglet ; un deck repris ou relu est un deck ordinaire sur son pack.

use sha2::{Digest, Sha256};
use uuid::{Builder, Uuid};

use oku_slides_core::{PackRef, PackScope, PackVersion};
use oku_slides_i18n::Localizer;

use crate::Session;
use crate::pack::PackManifest;
use crate::widgets_edit::WidgetTarget;

/// Id du pack enfant du retrofit de `pptx` sur la version `target`.
///
/// La moitié haute du SHA-256 de l'empreinte du `.pptx` suivie de la
/// version cible, en UUID v8 : le même fichier sur la même version donne le
/// même pack, une autre cible un autre pack.
pub(crate) fn child_id(pptx: &[u8], target: &PackVersion) -> Uuid {
    let mut hasher = Sha256::new();
    hasher.update(Sha256::digest(pptx));
    hasher.update(target.as_str().as_bytes());
    let digest: [u8; 32] = hasher.finalize().into();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Builder::from_custom_bytes(bytes).into_uuid()
}

/// Le pack enfant d'un retrofit, pas encore créé.
///
/// Son id, son nom et la version cible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChildPack {
    id: Uuid,
    name: String,
    target: PackRef,
}

impl ChildPack {
    /// L'enfant du retrofit de `pptx` sur `target`, pour le deck `deck`.
    ///
    /// `target` est une version du pack nommé `pack`.
    ///
    /// Son nom est dans la langue `l`, celle de l'interface à l'import.
    pub(crate) fn new(l: &Localizer, pptx: &[u8], target: PackRef, pack: &str, deck: &str) -> Self {
        let name = l.tr_with("session-import-retrofit-pack", &[("pack", pack.into()), ("deck", deck.into())]);
        Self { id: child_id(pptx, &target.version), name, target }
    }

    /// La version cible, que le deck garde tant qu'aucun widget ne s'y écrit.
    #[cfg(test)]
    pub(crate) fn target(&self) -> &PackRef {
        &self.target
    }

    /// Fait d'une écriture sur `t`, la version cible, une écriture sur le pack enfant.
    ///
    /// `manifest` prend l'id, le nom et la portée du deck seul ; la cible
    /// rendue a les fichiers de la version cible, qui devient la parente.
    fn adopt(&self, t: &WidgetTarget, manifest: &mut PackManifest) -> WidgetTarget {
        manifest.id = self.id;
        manifest.name.clone_from(&self.name);
        manifest.scope = PackScope::Deck;
        WidgetTarget { id: self.id, version: t.version.clone(), files: t.files.clone() }
    }
}

impl Session {
    /// Le pack enfant qui reçoit une écriture sur `t`, si `t` est sa cible.
    ///
    /// `manifest` (celui de l'écriture) devient alors celui de l'enfant.
    /// `None` hors d'un onglet de retrofit, ou une fois le deck sur un autre pack.
    pub(crate) fn retrofit_child_target(&self, t: &WidgetTarget, manifest: &mut PackManifest) -> Option<WidgetTarget> {
        let child = self.import.as_ref()?.child()?;
        let on_target = t.id == child.target.id && t.version == child.target.version;
        on_target.then(|| child.adopt(t, manifest))
    }
}
