//! The draft's attachments: kept by the tab's session, then added to its draft.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oku_slides_platform::config::PasteSettings;
use oku_slides_session::agent::{AttachmentRef, Upload};
use std::collections::HashSet;
use std::sync::PoisonError;

use snafu::ResultExt;
use tokio::sync::mpsc;

use crate::command_error::{
    AttachmentGoneSnafu, AttachmentNotBase64Snafu, AttachmentTooLargeSnafu, CommandError, PasteRefusedSnafu,
    TooManyAttachmentsSnafu, TooManyDocumentsSnafu, TooManyImagesSnafu,
};
use crate::tab_entry::{Out, TabLink};

/// Decodes, checks and keeps the attachment `data` (base64) named `name`, then adds it to `t`'s draft.
///
/// `data` longer than the base64 of the larger of `[paste] max_image_mib` and `max_document_mib`
/// is refused before decoding. An image counts toward `max_draft_images`, a document toward
/// `max_draft_documents` (by its magic only: the session then checks it); bytes that are neither
/// pass no cap and get the session's own refusal. Blocking (session lock): from `Stack::handle`,
/// never under the draft's lock.
pub(crate) fn attach(
    t: &TabLink,
    out: &mpsc::UnboundedSender<Out>,
    settings: &PasteSettings,
    name: &str,
    data: &str,
) -> Result<(), CommandError> {
    // One attach at a time per tab: otherwise two attaches pass the cap on the same snapshot.
    let _one_at_a_time = t.paste.lock().unwrap_or_else(PoisonError::into_inner);
    let max = encoded_max(settings);
    if data.len() > max {
        return Err(AttachmentTooLargeSnafu { len: data.len(), max }.build().into());
    }
    let bytes = STANDARD.decode(data).context(AttachmentNotBase64Snafu).map_err(CommandError::from)?;
    // Hashed and sniffed once, here, then checked by the session without reading them again.
    let upload = Upload::new(bytes, name);
    let draft = t.draft_attachments();
    let keep: Vec<String> = draft.iter().map(|i| i.id().to_owned()).collect();
    // Attaching again an item already in the draft does not grow it: allowed even at the cap.
    if !keep.iter().any(|k| k == upload.id()) {
        let image = |i: &&AttachmentRef| matches!(i, AttachmentRef::Image { .. });
        if upload.is_image() {
            let max = settings.max_draft_images;
            if draft.iter().filter(image).count() >= max {
                return Err(TooManyImagesSnafu { max }.build().into());
            }
        } else if upload.is_document() {
            let max = settings.max_draft_documents.get();
            if draft.iter().filter(|i| !image(i)).count() >= max {
                return Err(TooManyDocumentsSnafu { max }.build().into());
            }
        }
    }
    let attached = t
        .session
        .with(|s| s.attach(upload, &keep))
        .and_then(|r| r)
        .context(PasteRefusedSnafu)
        .map_err(CommandError::from)?;
    t.add_attachment(out, attached);
    Ok(())
}

/// The instruction's attachment `ids` (without duplicates), all kept by the tab, else an error.
///
/// More than `max` distinct ids: refused before any lookup.
pub(crate) fn resolve_attachments(t: &TabLink, ids: &[String], max: usize) -> Result<Vec<AttachmentRef>, CommandError> {
    distinct_within(ids, max)?
        .into_iter()
        .map(|id| match t.session.with(|s| s.attached(id).map(|(r, _)| r)) {
            Ok(Some(r)) => Ok(r),
            Ok(None) | Err(_) => Err(CommandError::from(AttachmentGoneSnafu { id: id.clone() }.build())),
        })
        .collect()
}

/// `ids` without duplicates, in their order; refused as soon as more than `max` are distinct.
fn distinct_within(ids: &[String], max: usize) -> Result<Vec<&String>, CommandError> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for id in ids {
        if seen.insert(id) {
            if unique.len() == max {
                return Err(TooManyAttachmentsSnafu { max }.build().into());
            }
            unique.push(id);
        }
    }
    Ok(unique)
}

/// The base64 length of the largest attachment the session keeps.
///
/// A length past `usize` bounds nothing (no string is that long): `usize::MAX` then.
fn encoded_max(settings: &PasteSettings) -> usize {
    settings.max_image_bytes().max(settings.max_document_bytes()).div_ceil(3).saturating_mul(4)
}

#[cfg(test)]
#[path = "attach_tests.rs"]
mod attach_tests;
