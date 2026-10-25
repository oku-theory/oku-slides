//! La sorte d'un pack : ses fichiers et la catégorie de ses widgets.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Category;

/// The kind of a widget pack: elements, or slide layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PackKind {
    /// A pack of elements (`widgets.json`, folder `widgets/`).
    Widgets,
    /// A pack of slide layouts (`layouts.json`, folder `layouts/`).
    Layouts,
}

impl PackKind {
    /// Les deux sortes, dans l'ordre de déclaration.
    pub const ALL: [PackKind; 2] = [PackKind::Widgets, PackKind::Layouts];

    /// Le nom du manifeste du pack.
    #[must_use]
    pub fn manifest_file(self) -> &'static str {
        match self {
            PackKind::Widgets => "widgets.json",
            PackKind::Layouts => "layouts.json",
        }
    }

    /// Le dossier des fichiers de widgets du pack.
    #[must_use]
    pub fn files_dir(self) -> &'static str {
        match self {
            PackKind::Widgets => "widgets",
            PackKind::Layouts => "layouts",
        }
    }

    /// La catégorie que les widgets d'un pack de cette sorte doivent déclarer.
    #[must_use]
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
