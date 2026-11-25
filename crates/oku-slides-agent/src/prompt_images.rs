//! Les blocs du prompt pour les images collées : leur liste, puis celles que l'agent voit.

use std::fmt::Write as _;
use std::sync::Arc;

use agent_client_protocol::schema::v1::{ContentBlock, ImageContent, TextContent};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use oku_slides_platform::config::{ImageRefusal, PromptImageLimits};
use oku_slides_session::agent::AttachmentRef;

/// The blocks of the prompt's images, before its text; none without an image.
///
/// `see`: the agent announces `promptCapabilities.image`. An image the limits refuse, or one
/// the tab forgot (`None`), is only listed, with the reason. `PromptImageLimits::admit` measures
/// the base64-encoded size (what the API receives), per image (`max_bytes`, `max_side`) and for
/// the whole prompt (`total_bytes`).
pub(crate) fn image_blocks(
    images: &[(AttachmentRef, Option<Arc<[u8]>>)],
    see: bool,
    limits: PromptImageLimits,
) -> Vec<ContentBlock> {
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
    for (n, (r, bytes)) in images.iter().enumerate() {
        let note = match bytes {
            None => " (no longer available: ask the user to paste it again)",
            Some(_) if !see => " (not shown: this agent does not receive images)",
            Some(b) => match limits.admit(b.len(), r.width.max(r.height), total) {
                Err(ImageRefusal::TooLarge) => " (not shown: too large to show you)",
                Err(ImageRefusal::TooHeavyTogether) => {
                    " (not shown: the images of this message are too heavy together)"
                }
                Ok(encoded) => {
                    total += encoded;
                    shown.push(ContentBlock::Image(ImageContent::new(STANDARD.encode(b), r.mime.clone())));
                    ""
                }
            },
        };
        let _ = writeln!(list, "{}. {} {} {}×{}{note}", n + 1, r.id, r.mime, r.width, r.height);
    }
    let mut blocks = vec![ContentBlock::Text(TextContent::new(list))];
    blocks.extend(shown);
    blocks
}

#[cfg(test)]
#[path = "prompt_images_tests.rs"]
mod prompt_images_tests;
