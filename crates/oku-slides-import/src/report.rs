//! Le rapport d'import : par slide, le modèle choisi, le placement, les pertes
//! et les replis ; les avertissements du lecteur ; la couverture du contenu.
//!
//! Une perte n'est jamais une erreur : l'import continue, le rapport la dit.
//! [`ImportReport::to_json`] donne la forme écrite par `--report`, avec le
//! message localisé de chaque perte et de chaque avertissement du lecteur.

use oku_slides_i18n::{Localize, Localizer};
use serde::Serialize;
use serde_json::Value;

use crate::source::{OpaqueReason, Warning, WarningKind};

/// Mode d'import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Contenu projeté sur un pack et un thème existants.
    Retrofit,
}

/// Où vont les contenus d'une slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    /// Blocs du slot `flow`, dans l'agencement du modèle.
    Flow,
    /// Éléments du slot libre, placés en cellules de la grille du modèle.
    Grid,
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
    /// Item intraduisible sans image de repli.
    Opaque(OpaqueReason),
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
            LossKind::RunStyle(_) => "import-loss-run-style",
            LossKind::Transform => "import-loss-transform",
            LossKind::ColorDropped(_) => "import-loss-color-dropped",
            LossKind::TextTruncated => "import-loss-text-truncated",
            LossKind::TableTruncated => "import-loss-table-truncated",
            LossKind::Opaque(_) => "import-loss-opaque",
            LossKind::UnsupportedImage(_) => "import-loss-unsupported-image",
            LossKind::HiddenSlide => "import-loss-hidden-slide",
            LossKind::Effects => "import-loss-effects",
            LossKind::SeriesDropped { .. } => "import-loss-series-dropped",
            LossKind::RepeatedDecor { .. } => "import-loss-repeated-decor",
            LossKind::NoSlot => "import-loss-no-slot",
        }
    }
}

impl Localize for Loss {
    fn localize(&self, l: &Localizer) -> String {
        let id = self.kind.message_id();
        match &self.kind {
            LossKind::RunStyle(e) => l.tr_with(id, &[("effect", effect_name(*e).into())]),
            LossKind::ColorDropped(hex) => l.tr_with(id, &[("color", hex.as_str().into())]),
            LossKind::Opaque(r) => l.tr_with(id, &[("reason", reason_name(r).into())]),
            LossKind::UnsupportedImage(mime) => l.tr_with(id, &[("format", mime.as_str().into())]),
            LossKind::SeriesDropped { kept, dropped } => {
                l.tr_with(id, &[("kept", (*kept).into()), ("dropped", (*dropped).into())])
            }
            LossKind::RepeatedDecor { text, slides } => {
                l.tr_with(id, &[("text", text.as_str().into()), ("slides", (*slides).into())])
            }
            LossKind::ListLevels
            | LossKind::MergedCells
            | LossKind::PercentStacked
            | LossKind::Transform
            | LossKind::TextTruncated
            | LossKind::TableTruncated
            | LossKind::HiddenSlide
            | LossKind::Effects
            | LossKind::NoSlot => l.tr(id),
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
}

impl ImportReport {
    /// Rapport vide du mode `mode`.
    pub fn new(mode: Mode) -> Self {
        Self { mode, slides: Vec::new(), warnings: Vec::new(), coverage: None }
    }

    /// Nombre total de pertes.
    pub fn loss_count(&self) -> usize {
        self.slides.iter().map(|s| s.losses.len()).sum()
    }

    /// La forme écrite : la sérialisation, plus un `message` localisé à côté
    /// de chaque perte et de chaque avertissement.
    pub fn to_json(&self, l: &Localizer) -> Value {
        // Ces types se sérialisent toujours : clés textuelles, nombres finis.
        let mut v = serde_json::to_value(self).unwrap_or_default();
        add_messages(&mut v, "warnings", &self.warnings, l);
        if let Some(slides) = v.get_mut("slides").and_then(Value::as_array_mut) {
            for (slide, report) in slides.iter_mut().zip(&self.slides) {
                add_messages(slide, "losses", &report.losses, l);
            }
        }
        v
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod report_tests;
