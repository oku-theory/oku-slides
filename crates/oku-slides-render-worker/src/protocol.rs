//! Messages échangés entre l'hôte et le travailleur, dans l'en-tête des trames.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Version du protocole : à changer à toute modification de [`Request`], de
/// [`Response`] ou des trames. L'hôte refuse un travailleur d'une autre version.
pub(crate) const PROTOCOL_VERSION: u32 = 2;

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
    /// Première requête d'une connexion : la version du protocole de l'hôte.
    Hello {
        version: u32,
    },
    SetLanguage {
        locale: String,
    },
    /// Charge : octets de la police.
    PutFont {
        id: String,
    },
    /// Oublie ces polices : aucun emplacement ouvert ne s'en sert plus.
    DropFonts {
        ids: Vec<String>,
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
    /// Les jetons du thème du slot (`None` : les jetons par défaut).
    SetTheme {
        slot: SlotId,
        tokens: Option<serde_json::Map<String, serde_json::Value>>,
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
    /// Rend la présentation de la charge (`.pptx`) par le témoin `witness`, avec ces
    /// polices ; `slides` : numéros source de la première et de la dernière slide.
    Reference {
        witness: crate::ReferenceWitness,
        fonts: Vec<String>,
        #[serde(default)]
        slides: Option<(u32, u32)>,
    },
    /// La page `page` d'une référence, en PNG de `width` pixels de large.
    ReferencePng {
        reference: u64,
        page: usize,
        width: u32,
    },
    ReleaseReference {
        reference: u64,
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
    /// La version du protocole du travailleur, en réponse à `Hello`.
    Hello {
        version: u32,
    },
    FileProbe {
        probe: oku_slides_render::FileProbe,
    },
    Compiled(Box<CompiledHeader>),
    /// Charge : octets du résultat.
    Bytes,
    /// Une référence rendue : son identifiant, le témoin et son nombre de pages.
    Reference {
        reference: u64,
        witness: String,
        pages: usize,
    },
    Error {
        error: oku_slides_render::WireError,
    },
}
