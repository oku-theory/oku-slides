//! Le rapport d'import : par slide, le modèle choisi, le placement, les pertes
//! et les replis ; les avertissements du lecteur ; la couverture du contenu.
//! Une perte n'est jamais une erreur : l'import continue, le rapport la dit.
//! [`ImportReport::to_json`] donne la forme écrite par `--report`, avec le
//! message localisé de chaque perte et de chaque avertissement du lecteur.
//! En hi-fi s'y ajoutent le rendu de référence ([`ReferenceReport`]) et l'écart
//! visuel de chaque slide (`SlideReport::score`, [`crate::metrics`]).

mod fonts;
mod kind;
mod reference;

use fonts::font_notice;
use reference::suspect_messages;
pub use reference::{ReferenceCause, ReferenceReport};

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;
use serde_json::Value;

use crate::fonts::FontChoice;
use crate::metrics::SlideScore;
use crate::source::{OpaqueReason, Warning, WarningKind};

/// Mode d'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Contenu projeté sur un pack et un thème existants.
    Retrofit,
    /// Copie fidèle, sur un pack et un thème générés.
    Hifi,
}

/// Où vont les contenus d'une slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    /// Blocs du slot `flow`, dans l'agencement du modèle.
    Flow,
    /// Éléments du slot libre, placés en cellules de la grille du modèle.
    Grid,
    /// Éléments du slot libre, chacun en cadre à sa place source (hi-fi).
    Free,
}

/// Effet de texte que le texte riche Oku n'exprime pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunEffect {
    /// Souligné.
    Underline,
    /// Barré.
    Strike,
    /// Exposant ou indice.
    Baseline,
    /// Capitales ou petites capitales.
    Caps,
    /// Surlignage.
    Highlight,
    /// Police propre au passage (hi-fi : l'élément n'a qu'une police).
    Font,
    /// Espacement des lettres propre au passage (hi-fi : l'élément n'en a
    /// qu'un).
    Tracking,
    /// Passage maigre rendu gras : le niveau du placeholder de layout est
    /// gras et un run du cœur n'ôte pas le gras (hi-fi).
    Weight,
    /// Passage droit rendu en italique : le niveau du placeholder de layout
    /// est en italique et un run du cœur n'ôte pas l'italique (hi-fi).
    Slant,
}

/// Rendu d'une forme que la base n'exprime pas (hi-fi).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeEffect {
    /// Dégradé rendu en couleur unie.
    Gradient,
    /// Image de remplissage étirée au cadre, hors du contour de la forme.
    Picture,
    /// Tirets rendus en trait continu.
    Dash,
    /// Géométrie inconnue rendue en rectangle.
    Geometry,
    /// Pointes de flèche d'un trait non dessinées.
    Arrow,
}

/// Nature d'une perte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "detail")]
pub enum LossKind {
    /// Niveaux de liste aplatis.
    ListLevels,
    /// Fusions de cellules aplaties.
    MergedCells,
    /// Empilement à 100 % rendu en empilement simple.
    PercentStacked,
    /// Format des nombres d'un graphique (`formatCode` : `0%`, monnaie…) non gardé.
    ChartFormat(String),
    /// Effet de texte perdu.
    RunStyle(RunEffect),
    /// Rotation, miroir ou rognage perdus.
    Transform,
    /// Couleur sans jeton proche dans le thème cible (`#rrggbb`).
    ColorDropped(String),
    /// Texte coupé aux bornes du cœur.
    TextTruncated,
    /// Tableau coupé aux bornes du cœur.
    TableTruncated,
    /// Données d'un graphique coupées aux bornes de relecture du cœur (lignes, cellules).
    ChartTruncated,
    /// Item intraduisible sans image de repli.
    Opaque(OpaqueReason),
    /// SmartArt sans dessin : ses nœuds posés en cartes, sa disposition perdue (retrofit).
    SmartArtLayout,
    /// Image dans un format que le rendu ne dessine pas (type MIME).
    UnsupportedImage(String),
    /// Slide masquée en diaporama : importée visible.
    HiddenSlide,
    /// Effets (ombres, reflets, 3D) perdus.
    Effects,
    /// Séries écartées : un graphique en secteurs n'en montre qu'une.
    SeriesDropped {
        /// Séries gardées.
        kept: u32,
        /// Séries écartées.
        dropped: u32,
    },
    /// En-tête ou pied répété sur les slides (texte, chiffres ôtés, ou nom de
    /// l'image), omis : le thème dessine les siens. Dit une fois, à sa première slide.
    RepeatedDecor {
        /// Texte du motif, ou nom de l'image.
        text: String,
        /// Nombre de slides où il apparaît.
        slides: u32,
    },
    /// Contenu sans place : le modèle choisi n'a aucun slot.
    NoSlot,
    /// Texte sans place sur une couverture : mis dans les notes de la slide (retrofit).
    MovedToNotes,
    /// Items de la slide au-delà de la borne du retrofit, omis (sur la slide).
    ItemsDropped {
        /// Items omis.
        dropped: u32,
    },
    /// Page source d'un autre format que la page Oku (16:9) : contenu mis à
    /// l'échelle uniformément et centré (hi-fi). Dit une fois, à la première slide.
    PageFitted {
        /// Largeur de la page source, en pt arrondis.
        width: u32,
        /// Hauteur de la page source, en pt arrondis.
        height: u32,
    },
    /// Alignement, interligne, espacement ou retrait d'un paragraphe ramené à
    /// celui de sa boîte (hi-fi : l'élément texte n'en a qu'un), ou marge du
    /// texte ramenée dans les bornes du cœur.
    ParagraphStyle,
    /// Rendu de forme approché (hi-fi).
    ShapeStyle(ShapeEffect),
    /// Layout sans widget généré : le pack a atteint le nombre de fichiers que
    /// la sonde charge (hi-fi). Ses slides prennent le widget `blank`, son
    /// fond et son décor y sont copiés en éléments. Dit une fois, à sa première slide.
    LayoutDropped {
        /// Nom du layout.
        layout: String,
    },
    /// Décor d'un master ou d'un layout coupé : son code dépasserait la taille
    /// d'un fichier de pack que la sonde charge (hi-fi). Sur le premier item omis.
    DecorTruncated {
        /// Items omis, celui-ci compris.
        omitted: u32,
    },
    /// Style d'un tableau approché (hi-fi) : fonds par cellule, bordures,
    /// largeurs de colonnes, hauteurs de lignes ou tailles de texte que le
    /// widget `table` ne rend pas, ou un style autre que celui du thème généré
    /// (le premier tableau).
    TableStyle,
    /// Tableau ou graphique sans données : rien n'est posé (hi-fi).
    EmptyData,
    /// Champ de texte rendu au-delà du bord de sa slide (relevé sur le rendu,
    /// dans les deux modes) : un contenu plus haut que sa place (t-816).
    Overflow {
        /// Le champ : `<id de l'élément ou de la slide Oku>/<paramètre>`.
        element: String,
    },
}

/// Une perte, sur un item source ou sur la slide entière.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Loss {
    /// Id de l'item source ; `None` pour la slide.
    pub item: Option<String>,
    /// Nature.
    pub kind: LossKind,
}

impl Loss {
    /// Perte sur l'item `item`.
    pub fn on(item: &str, kind: LossKind) -> Self {
        Self { item: Some(item.to_owned()), kind }
    }

    /// Perte sur la slide entière.
    pub fn slide(kind: LossKind) -> Self {
        Self { item: None, kind }
    }
}

/// Argument `effect` du message `import-loss-run-style`.
fn effect_name(e: RunEffect) -> &'static str {
    match e {
        RunEffect::Underline => "underline",
        RunEffect::Strike => "strike",
        RunEffect::Baseline => "baseline",
        RunEffect::Caps => "caps",
        RunEffect::Highlight => "highlight",
        RunEffect::Font => "font",
        RunEffect::Tracking => "tracking",
        RunEffect::Weight => "weight",
        RunEffect::Slant => "slant",
    }
}

/// Argument `effect` du message `import-loss-shape-style`.
fn shape_effect_name(e: ShapeEffect) -> &'static str {
    match e {
        ShapeEffect::Gradient => "gradient",
        ShapeEffect::Picture => "picture",
        ShapeEffect::Dash => "dash",
        ShapeEffect::Geometry => "geometry",
        ShapeEffect::Arrow => "arrow",
    }
}

/// Argument `reason` du message `import-loss-opaque`.
fn reason_name(r: &OpaqueReason) -> &'static str {
    match r {
        OpaqueReason::Media => "media",
        OpaqueReason::Ole => "ole",
        OpaqueReason::Math => "math",
        OpaqueReason::SmartArt => "smartart",
        OpaqueReason::Unknown(_) => "unknown",
    }
}

impl LossKind {
    /// Clé Fluent du message.
    pub(crate) fn message_id(&self) -> &'static str {
        match self {
            LossKind::ListLevels => "import-loss-list-levels",
            LossKind::MergedCells => "import-loss-merged-cells",
            LossKind::PercentStacked => "import-loss-percent-stacked",
            LossKind::ChartFormat(_) => "import-loss-chart-format",
            LossKind::RunStyle(_) => "import-loss-run-style",
            LossKind::Transform => "import-loss-transform",
            LossKind::ColorDropped(_) => "import-loss-color-dropped",
            LossKind::TextTruncated => "import-loss-text-truncated",
            LossKind::TableTruncated => "import-loss-table-truncated",
            LossKind::ChartTruncated => "import-loss-chart-truncated",
            LossKind::Opaque(_) => "import-loss-opaque",
            LossKind::SmartArtLayout => "import-loss-smartart-layout",
            LossKind::UnsupportedImage(_) => "import-loss-unsupported-image",
            LossKind::HiddenSlide => "import-loss-hidden-slide",
            LossKind::Effects => "import-loss-effects",
            LossKind::SeriesDropped { .. } => "import-loss-series-dropped",
            LossKind::RepeatedDecor { .. } => "import-loss-repeated-decor",
            LossKind::NoSlot => "import-loss-no-slot",
            LossKind::MovedToNotes => "import-loss-moved-to-notes",
            LossKind::ItemsDropped { .. } => "import-loss-items-dropped",
            LossKind::PageFitted { .. } => "import-loss-page-fitted",
            LossKind::ParagraphStyle => "import-loss-paragraph-style",
            LossKind::ShapeStyle(_) => "import-loss-shape-style",
            LossKind::LayoutDropped { .. } => "import-loss-layout-dropped",
            LossKind::DecorTruncated { .. } => "import-loss-decor-truncated",
            LossKind::TableStyle => "import-loss-table-style",
            LossKind::Overflow { .. } => "import-loss-overflow",
            LossKind::EmptyData => "import-loss-empty-data",
        }
    }
}

impl Localize for Loss {
    fn localize(&self, l: &Localizer) -> String {
        let id = self.kind.message_id();
        match &self.kind {
            LossKind::RunStyle(e) => l.tr_with(id, &[("effect", effect_name(*e).into())]),
            LossKind::ShapeStyle(e) => l.tr_with(id, &[("effect", shape_effect_name(*e).into())]),
            LossKind::ColorDropped(hex) => l.tr_with(id, &[("color", hex.as_str().into())]),
            LossKind::Opaque(r) => l.tr_with(id, &[("reason", reason_name(r).into())]),
            LossKind::UnsupportedImage(mime) => l.tr_with(id, &[("format", mime.as_str().into())]),
            LossKind::ChartFormat(code) => l.tr_with(id, &[("format", code.as_str().into())]),
            LossKind::SeriesDropped { kept, dropped } => {
                l.tr_with(id, &[("kept", (*kept).into()), ("dropped", (*dropped).into())])
            }
            LossKind::RepeatedDecor { text, slides } => {
                l.tr_with(id, &[("text", text.as_str().into()), ("slides", (*slides).into())])
            }
            LossKind::PageFitted { width, height } => {
                l.tr_with(id, &[("width", (*width).into()), ("height", (*height).into())])
            }
            LossKind::LayoutDropped { layout } => l.tr_with(id, &[("layout", layout.as_str().into())]),
            LossKind::DecorTruncated { omitted } => l.tr_with(id, &[("omitted", (*omitted).into())]),
            LossKind::Overflow { element } => l.tr_with(id, &[("element", element.as_str().into())]),
            LossKind::ItemsDropped { dropped } => l.tr_with(id, &[("dropped", (*dropped).into())]),
            LossKind::ListLevels
            | LossKind::MergedCells
            | LossKind::PercentStacked
            | LossKind::Transform
            | LossKind::TextTruncated
            | LossKind::TableTruncated
            | LossKind::ChartTruncated
            | LossKind::MovedToNotes
            | LossKind::HiddenSlide
            | LossKind::Effects
            | LossKind::NoSlot
            | LossKind::ParagraphStyle
            | LossKind::SmartArtLayout
            | LossKind::TableStyle
            | LossKind::EmptyData => l.tr(id),
        }
    }
}

/// Clé Fluent et détail du message d'un avertissement du lecteur.
fn warning_message(kind: &WarningKind) -> (&'static str, Option<&str>) {
    match kind {
        WarningKind::EffectIgnored(d) => ("import-warning-effect-ignored", Some(d)),
        WarningKind::Approximated(d) => ("import-warning-approximated", Some(d)),
        WarningKind::UnreadableItem(d) => ("import-warning-unreadable-item", Some(d)),
        WarningKind::UnreadablePart(d) => ("import-warning-unreadable-part", Some(d)),
        WarningKind::MissingPart(d) => ("import-warning-missing-part", Some(d)),
        WarningKind::UnsupportedChart(d) => ("import-warning-unsupported-chart", Some(d)),
        WarningKind::UnsupportedImage(d) => ("import-warning-unsupported-image", Some(d)),
        WarningKind::OversizedImage(d) => ("import-warning-oversized-image", Some(d)),
        WarningKind::ExternalImage(d) => ("import-warning-external-image", Some(d)),
        WarningKind::UnsafeSvg(d) => ("import-warning-unsafe-svg", Some(d)),
        WarningKind::UnknownTableStyle(d) => ("import-warning-unknown-table-style", Some(d)),
        WarningKind::UnsafeLink(d) => ("import-warning-unsafe-link", Some(d)),
        WarningKind::NestingTooDeep(d) => ("import-warning-nesting-too-deep", Some(d)),
        WarningKind::FontEmbeddedUnreadable(d) => ("import-warning-font-embedded-unreadable", Some(d)),
        WarningKind::Truncated => ("import-warning-truncated", None),
    }
}

impl Localize for Warning {
    fn localize(&self, l: &Localizer) -> String {
        match warning_message(&self.kind) {
            (id, Some(detail)) => l.tr_with(id, &[("detail", detail.into())]),
            (id, None) => l.tr(id),
        }
    }
}

/// Ajoute à chaque objet de `written` (le tableau `key` de `v`) le message localisé de l'entrée de même rang.
fn add_messages<T: Localize>(v: &mut Value, key: &str, entries: &[T], l: &Localizer) {
    let Some(written) = v.get_mut(key).and_then(Value::as_array_mut) else { return };
    for (w, entry) in written.iter_mut().zip(entries) {
        if let Some(o) = w.as_object_mut() {
            o.insert("message".to_owned(), Value::String(entry.localize(l)));
        }
    }
}

/// Ce que devient une slide source.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlideReport {
    /// Numéro de la slide source (1..).
    pub source: u32,
    /// Id de la slide Oku produite.
    pub slide: String,
    /// Id du modèle choisi.
    pub template: String,
    /// Placement des contenus.
    pub placement: Placement,
    /// Pertes.
    pub losses: Vec<Loss>,
    /// Sortes de widget de repli employées (`stack`…).
    pub fallbacks: Vec<String>,
    /// Écart visuel au rendu de référence (hi-fi, référence disponible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<SlideScore>,
}

impl SlideReport {
    /// Sa pire région non suspecte dépasse-t-elle `threshold` ?
    pub fn is_above(&self, threshold: f64) -> bool {
        self.score.as_ref().and_then(SlideScore::worst).is_some_and(|w| w > threshold)
    }
}

/// Une proportion : `found` sur `total`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Ratio {
    /// Retrouvés.
    pub found: u32,
    /// Attendus.
    pub total: u32,
}

impl Ratio {
    /// Vrai si tout est retrouvé (vrai aussi quand rien n'est attendu).
    pub fn is_complete(&self) -> bool {
        self.found >= self.total
    }
}

/// Un contenu source absent du deck obtenu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Missing {
    /// Texte d'un run.
    Run {
        /// Numéro de slide source.
        slide: u32,
        /// Texte du run, espaces normalisés.
        text: String,
    },
    /// Image.
    Image {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
    /// Tableau.
    Table {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
    /// Graphique.
    Chart {
        /// Numéro de slide source.
        slide: u32,
        /// Id de l'item source.
        item: String,
    },
}

/// Couverture du contenu, mesurée sur le deck obtenu.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Coverage {
    /// Runs de texte retrouvés.
    pub runs: Ratio,
    /// Images retrouvées.
    pub images: Ratio,
    /// Tableaux aux données égales.
    pub tables: Ratio,
    /// Graphiques aux données égales.
    pub charts: Ratio,
    /// Ce qui manque.
    pub missing: Vec<Missing>,
}

/// Le rapport d'un import.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportReport {
    /// Mode.
    pub mode: Mode,
    /// Une entrée par slide source, dans l'ordre.
    pub slides: Vec<SlideReport>,
    /// Avertissements du lecteur.
    pub warnings: Vec<Warning>,
    /// Couverture du contenu ; `None` tant qu'elle n'est pas mesurée, ou si le cœur a refusé le plan (bug).
    pub coverage: Option<Coverage>,
    /// Polices : une décision par famille source (hi-fi ; vide et non écrit en retrofit).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fonts: Vec<FontChoice>,
    /// Rendu de référence (hi-fi, `--reference auto`) ; non écrit sans lui.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<ReferenceReport>,
}

impl ImportReport {
    /// Rapport vide du mode `mode`.
    pub fn new(mode: Mode) -> Self {
        Self { mode, slides: Vec::new(), warnings: Vec::new(), coverage: None, fonts: Vec::new(), reference: None }
    }

    /// Nombre total de pertes.
    pub fn loss_count(&self) -> usize {
        self.slides.iter().map(|s| s.losses.len()).sum()
    }

    /// Les slides dont la pire région non suspecte dépasse le seuil de la référence.
    pub fn above_threshold(&self) -> impl Iterator<Item = &SlideReport> {
        let threshold = self.reference.as_ref().map(|r| r.threshold);
        self.slides.iter().filter(move |s| threshold.is_some_and(|t| s.is_above(t)))
    }

    /// L'écart visuel moyen des slides qui en ont un.
    pub fn mean_score(&self) -> Option<f64> {
        let scores: Vec<f64> = self.slides.iter().filter_map(|s| s.score.as_ref().map(|sc| sc.score)).collect();
        (!scores.is_empty()).then(|| (scores.iter().sum::<f64>() / scores.len() as f64 * 100.0).round() / 100.0)
    }

    /// La forme écrite.
    ///
    /// La sérialisation, plus un `message` localisé à côté de chaque perte, de chaque
    /// avertissement, de chaque police remplacée, absente ou aux droits restreints, de la référence
    /// absente et de chaque région suspecte écrite ; `above_threshold` sur chaque slide dont la
    /// pire région non suspecte dépasse le seuil.
    pub fn to_json(&self, l: &Localizer) -> Value {
        // Ces types se sérialisent toujours : clés textuelles, nombres finis.
        let mut v = serde_json::to_value(self).unwrap_or_default();
        add_messages(&mut v, "warnings", &self.warnings, l);
        let threshold = self.reference.as_ref().map(|r| r.threshold);
        if let Some(slides) = v.get_mut("slides").and_then(Value::as_array_mut) {
            for (slide, report) in slides.iter_mut().zip(&self.slides) {
                add_messages(slide, "losses", &report.losses, l);
                if let (Some(written), Some(score)) =
                    (slide.pointer_mut("/score/regions").and_then(Value::as_array_mut), &report.score)
                {
                    suspect_messages(written, &score.regions, l);
                }
                let above = threshold.is_some_and(|t| report.is_above(t));
                if let (Some(o), true) = (slide.as_object_mut(), above) {
                    o.insert("above_threshold".to_owned(), Value::Bool(true));
                }
            }
        }
        if let (Some(o), Some(cause)) = (
            v.get_mut("reference").and_then(Value::as_object_mut),
            self.reference.as_ref().and_then(|r| r.unavailable.as_ref()),
        ) {
            o.insert("message".to_owned(), Value::String(cause.localize(l)));
        }
        if let Some(fonts) = v.get_mut("fonts").and_then(Value::as_array_mut) {
            for (w, choice) in fonts.iter_mut().zip(&self.fonts) {
                if let (Some(o), Some(msg)) = (w.as_object_mut(), font_notice(choice, l)) {
                    o.insert("message".to_owned(), Value::String(msg));
                }
            }
        }
        v
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod report_tests;
