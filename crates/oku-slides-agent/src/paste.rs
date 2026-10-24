//! Images collées : dépôt dans la session de l'onglet, puis dans son brouillon.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oku_slides_platform::config::PasteSettings;
use oku_slides_session::agent::AttachmentRef;
use std::sync::PoisonError;

use snafu::ResultExt;
use tokio::sync::mpsc;

use crate::command_error::{CommandError, ImageGoneSnafu, ImageNotBase64Snafu, PasteRefusedSnafu, TooManyImagesSnafu};
use crate::tab_entry::{Out, TabLink};

/// Décode, valide et garde l'image `data` (base64), puis l'ajoute au brouillon de `t`.
///
/// Bloquant (verrou de la session) : depuis `Stack::handle`, jamais sous le verrou du brouillon.
pub(crate) fn paste_image(
    t: &TabLink,
    out: &mpsc::UnboundedSender<Out>,
    settings: &PasteSettings,
    data: &str,
) -> Result<(), CommandError> {
    // Un collage à la fois par onglet : sans cela, deux collages passent le plafond sur le même instantané.
    let _one_at_a_time = t.paste.lock().unwrap_or_else(PoisonError::into_inner);
    let bytes = STANDARD.decode(data).context(ImageNotBase64Snafu).map_err(CommandError::from)?;
    let keep: Vec<String> = t.draft_images().into_iter().map(|i| i.id).collect();
    let max = settings.max_draft_images;
    // Recoller une image déjà au brouillon ne l'agrandit pas : permis même au plafond.
    if keep.len() >= max && !keep.contains(&oku_slides_core::asset_id(&bytes)) {
        return Err(TooManyImagesSnafu { max }.build().into());
    }
    let image = t
        .session
        .with(|s| s.paste_image(bytes, &keep))
        .and_then(|r| r)
        .context(PasteRefusedSnafu)
        .map_err(CommandError::from)?;
    t.add_image(out, image);
    Ok(())
}

/// Les images `ids` de la consigne (sans doublon), toutes gardées par l'onglet, sinon une erreur.
///
/// Plus de `max` identifiants distincts : refusée avant toute recherche.
pub(crate) fn resolve_images(t: &TabLink, ids: &[String], max: usize) -> Result<Vec<AttachmentRef>, CommandError> {
    let mut unique: Vec<&String> = Vec::with_capacity(ids.len());
    for id in ids {
        if !unique.contains(&id) {
            unique.push(id);
        }
    }
    if unique.len() > max {
        return Err(TooManyImagesSnafu { max }.build().into());
    }
    unique
        .into_iter()
        .map(|id| match t.session.with(|s| s.pasted_image(id).map(|(r, _)| r)) {
            Ok(Some(r)) => Ok(r),
            Ok(None) | Err(_) => Err(CommandError::from(ImageGoneSnafu { id: id.clone() }.build())),
        })
        .collect()
}
