//! Messages de Typst : erreurs et avertissements, avec leur place dans les
//! sources et leurs indices. Ils se disent dans la langue de l'utilisateur.

use oku_slides_core::{PackKind, WidgetFile};
use oku_slides_i18n::{Localize, Localizer};

use crate::widget_probe::PROBE_PACK;

/// Gravité d'un message de Typst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

/// Position d'un message dans un fichier Typst, à partir de 1.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceAt {
    /// Chemin dans le monde Typst, sans `/` initial (`packs/<id>/widgets/kpi.typ`).
    pub path: String,
    pub line: usize,
    /// Colonne en caractères.
    pub column: usize,
}

/// Un message de Typst.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Fichier et ligne (`oku-slides/src/data.typ:12`), s'ils sont connus.
    pub place: Option<String>,
    /// Le message de Typst, tel quel (en anglais).
    pub message: String,
    pub hints: Vec<String>,
    /// Fichier, ligne et colonne, s'ils sont connus.
    #[serde(default)]
    pub at: Option<SourceAt>,
}

impl Localize for Diagnostic {
    fn localize(&self, l: &Localizer) -> String {
        let level = l.tr(match self.severity {
            Severity::Error => "diagnostic-error",
            Severity::Warning => "diagnostic-warning",
        });
        let mut out = match &self.place {
            Some(place) => l.tr_with(
                "diagnostic",
                &[("level", level.into()), ("place", place.as_str().into()), ("message", self.message.as_str().into())],
            ),
            None => {
                l.tr_with("diagnostic-unplaced", &[("level", level.into()), ("message", self.message.as_str().into())])
            }
        };
        for hint in &self.hints {
            out.push_str("\n  ");
            out.push_str(&l.tr_with("diagnostic-hint", &[("hint", hint.as_str().into())]));
        }
        out
    }
}

impl Diagnostic {
    /// Le fichier de widget d'un pack où se trouve le message, s'il y en a un.
    pub fn widget_file(&self) -> Option<WidgetFile> {
        let path = match &self.at {
            Some(at) => at.path.as_str(),
            None => {
                let place = self.place.as_deref()?;
                place.split_once(':').map_or(place, |(path, _)| path)
            }
        };
        let rest = path.trim_start_matches('/').strip_prefix("packs/")?;
        let (_, rest) = rest.split_once('/')?;
        // Le dossier des fichiers dit la sorte du pack : `widgets/` ou `layouts/`.
        let file = PackKind::ALL.into_iter().find_map(|k| rest.strip_prefix(k.files_dir())?.strip_prefix('/'))?;
        WidgetFile::from_file_name(file)
    }

    /// Une erreur sur le fichier de widget `name`, sans position.
    pub fn widget_error(name: &WidgetFile, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            place: Some(format!("packs/{PROBE_PACK}/widgets/{}", name.file_name())),
            message: message.into(),
            hints: Vec::new(),
            at: None,
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.localize(oku_slides_i18n::english()))
    }
}
