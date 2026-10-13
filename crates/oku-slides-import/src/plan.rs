//! Le plan d'import : les opérations à appliquer en une transaction, les
//! assets qu'elles référencent, les demandes de widget et le rapport ; en
//! hi-fi, aussi le thème et le pack générés, les schémas déclarés de ses
//! widgets et les polices à embarquer.
//!
//! Le plan est valide par construction : `oku_slides_core::apply_all` sur un
//! deck vide portant le pack et le thème cibles l'accepte (en hi-fi, ses ops
//! désignent elles-mêmes le thème et le pack générés, et le registre est la
//! base plus les schémas déclarés), et le deck obtenu ne lève aucune `Issue`
//! une fois les assets ajoutés au bundle.

use std::collections::BTreeMap;

use oku_slides_core::{Op, PackFiles, WidgetSchema};
use oku_slides_i18n::Localizer;
use serde::Serialize;
use serde_json::Value;

use crate::report::ImportReport;
use crate::source::{Asset, Item};

/// Un motif récurrent sans widget dédié, rendu par un repli.
///
/// Le lot 5 en fera un widget de pack enfant ; au lot 2, la demande est
/// seulement émise (plan et rapport).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetRequest {
    /// Signature du motif (`crate::pattern::signature`).
    pub signature: String,
    /// Ids des slides Oku où il apparaît, dans l'ordre.
    pub slides: Vec<String>,
    /// Les items source de sa première occurrence.
    pub sample: Vec<Item>,
    /// Sorte du widget de repli employé (`stack`…).
    pub fallback: String,
}

/// Le résultat d'un projecteur.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportPlan {
    /// Opérations, à appliquer dans l'ordre en une transaction.
    pub ops: Vec<Op>,
    /// Les assets référencés par `ops`, par identifiant (`asset_id` de leurs octets).
    pub assets: BTreeMap<String, Asset>,
    /// Demandes de widget, une par signature.
    pub widget_requests: Vec<WidgetRequest>,
    /// Rapport.
    pub report: ImportReport,
    /// Thème généré (hi-fi) : la version (`theme.json`) que désigne l'op
    /// `set_theme`, à ajouter aux thèmes du bundle (`add_theme`).
    #[serde(skip)]
    pub theme: Option<PackFiles>,
    /// Pack généré (hi-fi) : la version (`pack.json` et widgets) que désigne
    /// l'op `set_templates`, à ajouter aux packs du bundle (`add_pack`).
    #[serde(skip)]
    pub pack: Option<PackFiles>,
    /// Schémas déclarés des widgets du pack généré : la sonde du rendu doit
    /// lire exactement ceux-là (un écart est un bogue du générateur).
    #[serde(skip)]
    pub schemas: Vec<WidgetSchema>,
    /// Polices à embarquer dans `fonts/` du deck (hi-fi) : nom de fichier → octets.
    #[serde(skip)]
    pub fonts: BTreeMap<String, Vec<u8>>,
}

impl ImportPlan {
    /// Plan vide portant `report`.
    pub(crate) fn new(report: ImportReport) -> Self {
        Self {
            ops: Vec::new(),
            assets: BTreeMap::new(),
            widget_requests: Vec::new(),
            report,
            theme: None,
            pack: None,
            schemas: Vec::new(),
            fonts: BTreeMap::new(),
        }
    }

    /// Le rapport écrit par `--report` : [`ImportReport::to_json`] (messages
    /// localisés), plus les demandes de widget.
    pub fn report_json(&self, l: &Localizer) -> Value {
        let mut v = self.report.to_json(l);
        if let Some(o) = v.as_object_mut() {
            // Une demande se sérialise toujours : clés textuelles, nombres finis.
            o.insert("widget_requests".to_owned(), serde_json::to_value(&self.widget_requests).unwrap_or_default());
        }
        v
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod plan_tests;
