//! The prompt's attachment blocks: attached documents, pasted images and those the agent sees.

use std::fmt::Write as _;
use std::sync::Arc;

use agent_client_protocol::schema::v1::{ContentBlock, ImageContent, TextContent};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oku_slides_platform::config::{ImageRefusal, PromptImageLimits};
use oku_slides_session::agent::AttachmentRef;
use oku_slides_session::agent::bounds::label;

/// The blocks of the prompt's images, before its text; none without an image.
///
/// Attached documents are not images: skipped here ([`document_block`] lists them).
/// `see`: the agent announces `promptCapabilities.image`. An image the limits refuse, or one
/// the tab forgot (`None`), is only listed, with the reason. `PromptImageLimits::admit` measures
/// the base64-encoded size (what the API receives), per image (`max_bytes`, `max_side`) and for
/// the whole prompt (`total_bytes`).
pub(crate) fn image_blocks(
    images: &[(AttachmentRef, Option<Arc<[u8]>>)],
    see: bool,
    limits: PromptImageLimits,
) -> Vec<ContentBlock> {
    let images: Vec<_> = images
        .iter()
        .filter_map(|(r, bytes)| match r {
            AttachmentRef::Image { id, mime, width, height } => Some((id, mime, *width, *height, bytes)),
            AttachmentRef::Document { .. } => None,
        })
        .collect();
    if images.is_empty() {
        return Vec::new();
    }
    let mut list = String::from(
        "Pasted images (pasted by the user with this message; place one with apply: \
         add_element {\"kind\": \"base/image\", \"params\": {\"asset\": \"<id>\"}}, or set_param asset \
         on an existing image; never write an asset id you were not given or did not read from the deck):\n",
    );
    let mut shown = Vec::new();
    let mut total = 0usize;
    for (n, (id, mime, width, height, bytes)) in images.into_iter().enumerate() {
        let note = match bytes {
            None => " (no longer available: ask the user to paste it again)",
            Some(_) if !see => " (not shown: this agent does not receive images)",
            Some(b) => match limits.admit(b.len(), width.max(height), total) {
                Err(ImageRefusal::TooLarge) => " (not shown: too large to show you)",
                Err(ImageRefusal::TooHeavyTogether) => {
                    " (not shown: the images of this message are too heavy together)"
                }
                Ok(encoded) => {
                    total += encoded;
                    shown.push(ContentBlock::Image(ImageContent::new(STANDARD.encode(b), mime.clone())));
                    ""
                }
            },
        };
        let _ = writeln!(list, "{}. {id} {mime} {width}×{height}{note}", n + 1);
    }
    let mut blocks = vec![ContentBlock::Text(TextContent::new(list))];
    blocks.extend(shown);
    blocks
}

/// The block listing the prompt's attached documents, before its text; `None` without one.
///
/// Each line gives the id to import with `knowledge_import`, the name (through [`label`]: no
/// quote nor control can close it early, and at most `LABEL_MAX` characters), the type and the size in KB rounded up. A document
/// the tab forgot (`None`) is listed with a note asking to attach it again.
pub(crate) fn document_block(attachments: &[(AttachmentRef, Option<Arc<[u8]>>)]) -> Option<ContentBlock> {
    let mut list = String::from(
        "Attached documents (attached by the user with this message; import one with knowledge_import \
         {\"attachment\": \"<id>\"} before quoting it):\n",
    );
    let mut any = false;
    for (r, bytes) in attachments {
        let AttachmentRef::Document { id, name, kind, bytes: size } = r else { continue };
        any = true;
        let note = if bytes.is_none() { " (no longer available: ask the user to attach it again)" } else { "" };
        let _ = writeln!(list, "- {id} \"{}\" ({}, {} KB){note}", label(name), kind.label(), size.div_ceil(1024));
    }
    any.then(|| ContentBlock::Text(TextContent::new(list)))
}

#[cfg(test)]
#[path = "prompt_images_tests.rs"]
mod prompt_images_tests;
