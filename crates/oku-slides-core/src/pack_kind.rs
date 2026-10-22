//! La sorte d'un pack : ses fichiers et la catégorie de ses widgets.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Category;

/// La sorte d'un pack de widgets : des éléments, ou des mises en page de slide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PackKind {
    /// Un pack d'éléments (`widgets.json`, dossier `widgets/`).
    Widgets,
    /// Un pack de layouts de slide (`layouts.json`, dossier `layouts/`).
    Layouts,
}

impl PackKind {
    /// Les deux sortes, dans l'ordre de déclaration.
    pub const ALL: [PackKind; 2] = [PackKind::Widgets, PackKind::Layouts];

    /// Le nom du manifeste du pack.
    pub fn manifest_file(self) -> &'static str {
        match self {
            PackKind::Widgets => "widgets.json",
            PackKind::Layouts => "layouts.json",
        }
    }

    /// Le dossier des fichiers de widgets du pack.
    pub fn files_dir(self) -> &'static str {
        match self {
            PackKind::Widgets => "widgets",
            PackKind::Layouts => "layouts",
        }
    }

    /// La catégorie que les widgets d'un pack de cette sorte doivent déclarer.
    pub fn category(self) -> Category {
        match self {
            PackKind::Widgets => Category::Element,
            PackKind::Layouts => Category::Slide,
        }
    }
}

#[cfg(test)]
#[path = "pack_kind_tests.rs"]
mod pack_kind_tests;
