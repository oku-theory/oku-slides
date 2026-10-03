//! Messages échangés entre l'hôte et le travailleur, dans l'en-tête des trames.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Identifiant d'un emplacement (un rendu du travailleur).
pub(crate) type SlotId = u32;

/// Un pack de widgets : son identifiant et ses fichiers (nom → source).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WirePack {
    pub(crate) id: String,
    pub(crate) files: BTreeMap<String, String>,
}

/// Une requête de l'hôte.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub(crate) enum Request {
    SetLanguage {
        locale: String,
    },
    /// Charge : octets de la police.
    PutFont {
        id: String,
    },
    /// Charge : octets de l'asset.
    PutAsset {
        id: String,
    },
    /// Sonde un fichier de widget seul ; `common` : la source de `_commun`.
    ProbeFile {
        name: String,
        source: String,
        #[serde(default)]
        common: Option<String>,
    },
    OpenSlot {
        slot: SlotId,
        fonts: Vec<String>,
    },
    CloseSlot {
        slot: SlotId,
    },
    SetWidgets {
        slot: SlotId,
        pack: Option<WirePack>,
    },
    Render {
        slot: SlotId,
        deck: Box<oku_slides_core::Deck>,
        assets: Vec<String>,
    },
    Svg {
        slot: SlotId,
        generation: u64,
        page: usize,
    },
    Png {
        slot: SlotId,
        generation: u64,
        page: usize,
        pixel_per_pt: f32,
    },
    Pdf {
        slot: SlotId,
        generation: u64,
    },
    Release {
        slot: SlotId,
        generations: Vec<u64>,
    },
}

/// Ce que le travailleur dit d'un rendu réussi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CompiledHeader {
    pub(crate) generation: u64,
    pub(crate) page_count: usize,
    /// `u128` en hexadécimal (32 chiffres) : JSON ne porte pas d'entier de 128 bits.
    pub(crate) page_hashes: Vec<String>,
    pub(crate) layout: oku_slides_render::Layout,
    pub(crate) tokens: Option<serde_json::Value>,
    pub(crate) warnings: Vec<oku_slides_render::Diagnostic>,
    pub(crate) chart_palette: Vec<String>,
}

/// La réponse du travailleur à une requête.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "reply", rename_all = "kebab-case")]
pub(crate) enum Response {
    Ok,
    FileProbe {
        probe: oku_slides_render::FileProbe,
    },
    Compiled(Box<CompiledHeader>),
    /// Charge : octets du résultat.
    Bytes,
    Error {
        error: oku_slides_render::WireError,
    },
}
