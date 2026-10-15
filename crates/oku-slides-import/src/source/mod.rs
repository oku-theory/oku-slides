//! La représentation intermédiaire : un deck source résolu, neutre vis-à-vis du format.
//!
//! Tout est concret (plus d'héritage à calculer) ; les valeurs stylées gardent
//! leur provenance. Les `Frame` sont en pt source, absolus. Le vocabulaire
//! OOXML y reste : c'est la langue commune des lecteurs, et un futur lecteur
//! PDF s'y ramène. Où il paraît :
//! - [`Geometry::Preset`] : noms de `prstGeom` (un lecteur PDF émet `Geometry::Path`) ;
//! - [`Origin`] : emplacements de thème (`accent1`, `+mn-lt`…) ; un PDF, `Origin::Explicit` ;
//! - [`LayoutKind`] : types de layout ; un PDF, `LayoutKind::Custom` ;
//! - [`Role`] : tiré des types de placeholder (`title`, `body`…) ; un PDF n'en a pas ;
//! - [`Run::field`] : types de champ (`slidenum`, `datetime1`…) ;
//! - [`Bullet::AutoNum`] : schémas de numérotation (`arabicPeriod`…).
//!
//! Les projecteurs `match`ent exhaustivement : pas de `#[non_exhaustive]` ici.

mod chart;
mod font;
mod shape;
mod style;
mod table;
mod text;

use std::collections::BTreeMap;

use serde::Serialize;

pub use chart::{Chart, ChartKind, Grouping, Series};
pub use font::{Asset, FaceStyle, FontFace, SourceFont};
pub(crate) use font::{family_weight, is_drawable};
pub use shape::{Crop, Geometry, PathCmd, Picture, Shape, SubPath};
pub use style::{Color, ColorMod, Dash, Fill, GradientStop, Origin, Rgba, Stroke, Styled};
pub use table::{Borders, Cell, Table, TableFlags};
pub(crate) use text::SLIDE_NUMBER;
pub use text::{
    Align, Anchor, Autofit, Bullet, Caps, Insets, LevelStyle, ParaStyle, Paragraph, Run, RunStyle, Spacing, TextBox,
};

/// Taille de page, en pt.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Size {
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
}

/// Boîte d'un item : position et taille en pt (page), rotation, miroirs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Frame {
    /// Bord gauche.
    pub x: f64,
    /// Bord haut.
    pub y: f64,
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
    /// Rotation en degrés, sens horaire, dans `[0, 360)`.
    pub rot: f64,
    /// Miroir horizontal.
    pub flip_h: bool,
    /// Miroir vertical.
    pub flip_v: bool,
}

impl Frame {
    /// Boîte sans rotation ni miroir.
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h, rot: 0.0, flip_h: false, flip_v: false }
    }
}

/// Rôle d'un placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Titre (`title`, `ctrTitle`).
    Title,
    /// Sous-titre.
    Subtitle,
    /// Corps de texte.
    Body,
    /// Contenu quelconque (`obj`, défaut).
    Object,
    /// Image.
    Picture,
    /// Graphique.
    Chart,
    /// Tableau.
    Table,
    /// Média.
    Media,
    /// Date.
    Date,
    /// Pied de page.
    Footer,
    /// Numéro de slide.
    SlideNum,
}

/// Thème source.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceTheme {
    /// Nom du thème.
    pub name: String,
    /// Couleurs par emplacement : `dk1 lt1 dk2 lt2 accent1..accent6 hlink folHlink`.
    pub colors: BTreeMap<String, Rgba>,
    /// Polices.
    pub fonts: ThemeFonts,
}

/// Polices du thème (écriture latine).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ThemeFonts {
    /// Titres.
    pub major: String,
    /// Corps.
    pub minor: String,
}

/// Placeholder d'un master ou d'un layout.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Placeholder {
    /// Identifiant d'item.
    pub id: String,
    /// Rôle.
    pub role: Role,
    /// Index (`ph@idx`).
    pub idx: Option<u32>,
    /// Boîte.
    pub frame: Frame,
    /// Style d'un texte saisi à chaque niveau 0..8.
    pub levels: Vec<LevelStyle>,
    /// Texte d'invite.
    pub text: TextBox,
}

/// Master.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Master {
    /// Identifiant (`m1`…).
    pub id: String,
    /// Nom.
    pub name: String,
    /// Index de son thème dans `SourceDeck.themes`.
    pub theme: usize,
    /// Fond, résolu avec les couleurs du master.
    pub background: Fill,
    /// Items hors placeholders.
    pub decor: Vec<Item>,
    /// Placeholders.
    pub placeholders: Vec<Placeholder>,
    /// Styles de texte du master (`txStyles`), premier niveau.
    pub text_styles: MasterTextStyles,
}

/// Premier niveau des styles de texte d'un master (`txStyles`), résolus sur les
/// défauts de la présentation, sans placeholder : ce que prend un texte de ce
/// rôle quand le master n'a pas de placeholder pour lui.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MasterTextStyles {
    /// Titres (`titleStyle`).
    pub title: LevelStyle,
    /// Corps (`bodyStyle`).
    pub body: LevelStyle,
    /// Autres textes : boîtes de texte, date, pied, numéro (`otherStyle`).
    pub other: LevelStyle,
}

/// Type d'un layout, dans le vocabulaire OOXML (`ST_SlideLayoutType`).
///
/// Les types que lisent les projecteurs sont nommés ; tout autre reste brut dans
/// [`LayoutKind::Other`]. Un lecteur sans types de layout (PDF) émet [`LayoutKind::Custom`].
/// Sérialisé sous son nom OOXML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutKind {
    /// Titre (`title`).
    Title,
    /// En-tête de section (`secHead`).
    SectionHeader,
    /// Titre et contenu (`obj`).
    TitleAndContent,
    /// Deux contenus (`twoObj`).
    TwoContent,
    /// Titre seul (`titleOnly`).
    TitleOnly,
    /// Vide (`blank`).
    Blank,
    /// Personnalisé (`cust`), le défaut OOXML.
    Custom,
    /// Un autre type OOXML, brut (32 caractères au plus).
    Other(String),
}

impl LayoutKind {
    /// Le type de nom OOXML `name`.
    pub fn from_ooxml(name: &str) -> Self {
        match name {
            "title" => Self::Title,
            "secHead" => Self::SectionHeader,
            "obj" => Self::TitleAndContent,
            "twoObj" => Self::TwoContent,
            "titleOnly" => Self::TitleOnly,
            "blank" => Self::Blank,
            "cust" => Self::Custom,
            other => Self::Other(other.to_owned()),
        }
    }

    /// Son nom OOXML.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Title => "title",
            Self::SectionHeader => "secHead",
            Self::TitleAndContent => "obj",
            Self::TwoContent => "twoObj",
            Self::TitleOnly => "titleOnly",
            Self::Blank => "blank",
            Self::Custom => "cust",
            Self::Other(name) => name,
        }
    }
}

impl Serialize for LayoutKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

/// Layout.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Layout {
    /// Identifiant (`l1`…).
    pub id: String,
    /// Nom.
    pub name: String,
    /// Type (`type` de `p:sldLayout`).
    pub kind: LayoutKind,
    /// Identifiant de son master.
    pub master: String,
    /// Fond propre, résolu avec les couleurs du layout ; `None` = celui du master.
    pub background: Option<Fill>,
    /// Le décor du master est-il dessiné sous ce layout (`showMasterSp`) ?
    pub show_master_decor: bool,
    /// Items hors placeholders.
    pub decor: Vec<Item>,
    /// Placeholders.
    pub placeholders: Vec<Placeholder>,
}

/// Slide.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceSlide {
    /// Identifiant (`s1`…).
    pub id: String,
    /// Numéro 1.. dans l'ordre de la présentation.
    pub number: u32,
    /// Identifiant de son layout, s'il est lisible.
    pub layout: Option<String>,
    /// Fond résolu avec les couleurs de la slide (fond hérité compris).
    pub background: Fill,
    /// Le décor du layout (et du master) est-il dessiné sous la slide ?
    pub show_layout_decor: bool,
    /// Masquée en diaporama.
    pub hidden: bool,
    /// Items, dans l'ordre de dessin.
    pub items: Vec<Item>,
    /// Notes, en texte brut.
    pub notes: String,
}

/// Placeholders parents d'un item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlaceholderLink {
    /// Index du placeholder (`ph@idx`).
    pub idx: Option<u32>,
    /// Id du placeholder de layout rempli.
    pub layout: Option<String>,
    /// Id du placeholder de master correspondant.
    pub master: Option<String>,
}

/// Pourquoi un item n'a pas été traduit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OpaqueReason {
    /// Vidéo ou son.
    Media,
    /// Objet OLE.
    Ole,
    /// Équation.
    Math,
    /// SmartArt sans version dessinée lisible.
    SmartArt,
    /// Autre (URI du `graphicData`, `nesting`…).
    Unknown(String),
}

/// Item intraduisible.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Opaque {
    /// Raison.
    pub reason: OpaqueReason,
    /// Asset de l'image de repli fournie par le fichier.
    pub fallback: Option<String>,
}

/// Sorte d'item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ItemKind {
    /// Boîte de texte sans remplissage ni trait.
    Text(TextBox),
    /// Forme dessinée (en boîte : de loin la plus grosse variante).
    Shape(Box<Shape>),
    /// Image.
    Picture(Picture),
    /// Tableau.
    Table(Table),
    /// Graphique.
    Chart(Chart),
    /// Groupe (ou SmartArt dessiné).
    Group {
        /// Enfants, dans l'ordre de dessin.
        items: Vec<Item>,
        /// Version dessinée d'un SmartArt (`dsp:drawing`), plutôt qu'un groupe de l'auteur.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        smartart: bool,
    },
    /// Intraduisible.
    Opaque(Opaque),
}

/// Élément posé sur une slide, un layout ou un master.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    /// Identifiant unique dans le deck (`s3/12`, `s3/12/{GUID}`, `s3/12~2`).
    pub id: String,
    /// Nom donné dans l'éditeur (`cNvPr@name`).
    pub name: String,
    /// Texte alternatif (`cNvPr@descr`).
    pub alt: Option<String>,
    /// Boîte.
    pub frame: Frame,
    /// Rang de dessin dans son parent (0 = au fond).
    pub z: u32,
    /// Rôle, si c'est un placeholder.
    pub role: Option<Role>,
    /// Placeholders parents.
    pub placeholder: Option<PlaceholderLink>,
    /// Contenu.
    pub kind: ItemKind,
}

impl Item {
    /// Le texte de l'item : celui d'une boîte de texte ou d'une forme.
    pub fn text(&self) -> Option<&TextBox> {
        match &self.kind {
            ItemKind::Text(tb) => Some(tb),
            ItemKind::Shape(s) => s.text.as_ref(),
            _ => None,
        }
    }

    /// L'item porte-t-il un texte non vide (autre que des espaces) ?
    pub fn has_text(&self) -> bool {
        self.text().is_some_and(|tb| tb.paragraphs.iter().flat_map(|p| &p.runs).any(|r| !r.text.trim().is_empty()))
    }
}

/// Ce que le lecteur a ignoré, approché ou dû replier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum WarningKind {
    /// Effet non traduit (`effectLst`, `scene3d`, `sp3d`, rotation ou miroir de groupe).
    EffectIgnored(String),
    /// Rendu approché (`pattFill`, dégradé non linéaire, `txXfrm`, valeurs tronquées).
    Approximated(String),
    /// Item illisible, retiré.
    UnreadableItem(String),
    /// Part de structure illisible (slide omise, layout absent, thème par défaut).
    UnreadablePart(String),
    /// Part référencée absente du paquet.
    MissingPart(String),
    /// Graphique non géré, ou combiné (seul le premier type est lu).
    UnsupportedChart(String),
    /// Image dont le type réel n'est pas dessiné par Typst : son chemin.
    ///
    /// `emf`, `bmp`, type inconnu… : une indication pour les projecteurs.
    UnsupportedImage(String),
    /// Image liée hors du fichier (`r:link`), non importée : sa cible.
    ExternalImage(String),
    /// SVG assaini (script, lien externe… retirés) ou rejeté (DTD, illisible) : son chemin.
    UnsafeSvg(String),
    /// Style de tableau référencé mais absent de `tableStyles.xml`.
    UnknownTableStyle(String),
    /// Lien refusé (schéma, longueur, caractères).
    UnsafeLink(String),
    /// Imbrication trop profonde, coupée.
    NestingTooDeep(String),
    /// Police embarquée illisible (EOT compressé…).
    FontEmbeddedUnreadable(String),
    /// Trop d'avertissements : les suivants sont omis.
    Truncated,
}

/// Avertissement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    /// Numéro de slide concernée.
    pub slide: Option<u32>,
    /// Id de l'item concerné.
    pub item: Option<String>,
    /// Nature.
    pub kind: WarningKind,
}

/// Le deck source résolu.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceDeck {
    /// Taille de page.
    pub size: Size,
    /// Un thème par master déclaré (même index).
    pub themes: Vec<SourceTheme>,
    /// Masters lisibles.
    pub masters: Vec<Master>,
    /// Layouts lisibles.
    pub layouts: Vec<Layout>,
    /// Slides lisibles.
    pub slides: Vec<SourceSlide>,
    /// Assets par identifiant.
    pub assets: BTreeMap<String, Asset>,
    /// Familles utilisées ou embarquées, triées.
    pub fonts: Vec<SourceFont>,
    /// Avertissements (1 000 au plus, puis `Truncated`).
    pub warnings: Vec<Warning>,
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
