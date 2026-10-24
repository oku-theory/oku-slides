//! Images collées dans le brouillon de l'agent : gardées par onglet, en mémoire.
//!
//! Une image est validée au collage (type réel, octets, pixels de l'en-tête), rangée sous
//! son identifiant d'asset `sha256:…`, et n'entre dans le deck que quand une op la cite
//! (`Session::embed_op_assets`).

use std::collections::VecDeque;
use std::sync::Arc;

use oku_slides_core::media::{dimensions, sniff};
use oku_slides_core::{Deck, WidgetRegistry};
use serde::{Deserialize, Serialize};

use crate::error::{PasteSnafu, UnknownAssetSnafu};
use crate::{Result, Session};

/// Les types qu'un collage accepte : ceux que Typst dessine tels quels.
const PASTE_TYPES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Une image collée : son identifiant d'asset, son type réel et sa taille en pixels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentRef {
    /// `sha256:<hex>` de ses octets, le même que son asset une fois posée.
    pub id: String,
    /// Son type réel, lu dans ses octets.
    pub mime: String,
    /// Sa largeur en pixels.
    pub width: u32,
    /// Sa hauteur en pixels.
    pub height: u32,
}

impl AttachmentRef {
    /// Ce que la bulle du fil montre de l'image.
    pub fn shown(&self) -> oku_slides_a2ui::PromptImage {
        oku_slides_a2ui::PromptImage { mime: self.mime.clone(), width: self.width, height: self.height }
    }
}

/// Pourquoi un collage est refusé (erreur [`crate::SessionErrorKind::Paste`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PasteReason {
    /// Pas un PNG, JPEG, GIF ni WebP.
    Unsupported,
    /// Au-delà de `[paste] max_image_mib`.
    TooLarge,
    /// Au-delà de `[paste] max_image_pixels`.
    TooManyPixels,
    /// En-tête illisible.
    Unreadable,
}

impl PasteReason {
    /// La clé Fluent du refus.
    pub fn key(self) -> &'static str {
        match self {
            Self::Unsupported => "session-paste-unsupported",
            Self::TooLarge => "session-paste-too-large",
            Self::TooManyPixels => "session-paste-too-many-pixels",
            Self::Unreadable => "session-paste-unreadable",
        }
    }
}

/// Les images collées d'un onglet, de la plus ancienne à la plus récente.
#[derive(Debug)]
pub(crate) struct Attachments {
    limits: oku_slides_platform::config::PasteSettings,
    items: VecDeque<(AttachmentRef, Arc<[u8]>)>,
    bytes: usize,
}

impl Attachments {
    pub(crate) fn new(limits: oku_slides_platform::config::PasteSettings) -> Self {
        Self { limits, items: VecDeque::new(), bytes: 0 }
    }

    /// Valide et garde `bytes` ; au-delà de `max_kept_mib`, oublie les plus anciennes hors de `keep`.
    fn add(&mut self, bytes: Vec<u8>, keep: &[String]) -> std::result::Result<AttachmentRef, PasteReason> {
        let mime = sniff(&bytes);
        if !PASTE_TYPES.contains(&mime) {
            return Err(PasteReason::Unsupported);
        }
        if bytes.len() > self.limits.max_image_bytes() {
            return Err(PasteReason::TooLarge);
        }
        let (width, height) = dimensions(&bytes, mime).ok_or(PasteReason::Unreadable)?;
        if u64::from(width) * u64::from(height) > self.limits.max_image_pixels {
            return Err(PasteReason::TooManyPixels);
        }
        let id = oku_slides_core::asset_id(&bytes);
        // Déjà là : elle redevient la plus récente.
        if let Some(i) = self.items.iter().position(|(r, _)| r.id == id)
            && let Some(item) = self.items.remove(i)
        {
            let r = item.0.clone();
            self.items.push_back(item);
            return Ok(r);
        }
        let r = AttachmentRef { id, mime: mime.to_owned(), width, height };
        self.bytes += bytes.len();
        self.items.push_back((r.clone(), bytes.into()));
        self.evict(keep, &r.id);
        Ok(r)
    }

    /// Oublie les plus anciennes, hors de `keep` et de `fresh`, tant que le total dépasse la borne.
    fn evict(&mut self, keep: &[String], fresh: &str) {
        while self.bytes > self.limits.max_kept_bytes() {
            let Some(i) = self.items.iter().position(|(r, _)| r.id != fresh && !keep.contains(&r.id)) else {
                return;
            };
            if let Some((_, b)) = self.items.remove(i) {
                self.bytes -= b.len();
            }
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<(AttachmentRef, Arc<[u8]>)> {
        self.items.iter().find(|(r, _)| r.id == id).map(|(r, b)| (r.clone(), b.clone()))
    }
}

impl Session {
    /// Valide une image collée et la garde pour l'onglet ; `keep` : les ids du brouillon.
    pub fn paste_image(&mut self, bytes: Vec<u8>, keep: &[String]) -> Result<AttachmentRef> {
        self.attachments.add(bytes, keep).map_err(|reason| PasteSnafu { reason }.build().into())
    }

    /// Une image collée gardée par l'onglet, et ses octets.
    pub fn pasted_image(&self, id: &str) -> Option<(AttachmentRef, Arc<[u8]>)> {
        self.attachments.get(id)
    }

    /// Ajoute au bundle les images collées que `next` cite et que le deck ne citait pas.
    ///
    /// Un autre asset nouvellement cité, absent du bundle, est refusé ; un asset déjà
    /// manquant avant l'op (deck importé) reste au `validate` de le signaler.
    pub(crate) fn embed_op_assets(&mut self, next: &Deck, registry: &WidgetRegistry) -> Result<()> {
        let before = oku_slides_core::asset_refs(self.doc.sources.deck(), registry);
        let mut found = Vec::new();
        for id in oku_slides_core::asset_refs(next, registry) {
            if before.contains(&id) || self.doc.sources.assets.contains_key(&id) {
                continue;
            }
            match self.attachments.get(&id) {
                Some((_, bytes)) => found.push(bytes),
                None => return UnknownAssetSnafu { asset: id }.fail()?,
            }
        }
        // Tout est vérifié avant d'ajouter : un refus n'ajoute rien.
        for bytes in found {
            self.doc.sources.add_asset(bytes.to_vec());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "attachments_tests.rs"]
mod attachments_tests;
