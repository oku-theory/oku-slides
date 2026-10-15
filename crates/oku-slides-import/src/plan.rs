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
use std::io;
use std::ops::Deref;

use oku_slides_core::{Op, PackFiles, WidgetSchema};
use oku_slides_i18n::Localizer;
use serde::Serialize;
use serde_json::Value;

use crate::error::{ImportError, output_too_large};
use crate::limits::ReadLimits;
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

    /// Vérifie que la sortie tient dans les budgets d'import de `limits`.
    ///
    /// Borne précoce et approximative de ce que l'import produira, pas une garantie que
    /// le deck se rouvrira : assets (uniques par contenu), polices embarquées, ops
    /// sérialisées en JSON, thème et pack générés (`pack.json` et widgets) tiennent dans
    /// [`ReadLimits::max_output_bytes`] ; les ops seules, qui finissent dans l'historique
    /// Loro, tiennent aussi dans [`ReadLimits::max_replica_bytes`].
    ///
    /// Les projecteurs sont infaillibles : l'appelant vérifie le plan qu'ils rendent,
    /// avant de l'appliquer, et n'applique ou n'écrit que le [`CheckedPlan`] rendu. Un
    /// dépassement est fatal, de nature `TooLarge`.
    pub fn check_budget(self, limits: &ReadLimits) -> Result<CheckedPlan, ImportError> {
        self.fits(limits)?;
        Ok(CheckedPlan(self))
    }

    /// Vérifie les budgets de [`Self::check_budget`].
    fn fits(&self, limits: &ReadLimits) -> Result<(), ImportError> {
        let mut ops = Budget { left: limits.max_replica_bytes };
        if !ops.take_json(&self.ops) {
            return Err(output_too_large(limits.max_replica_bytes));
        }
        let ops_len = limits.max_replica_bytes - ops.left;
        let mut budget = Budget { left: limits.max_output_bytes };
        let fits = budget.take_u64(ops_len)
            && self.assets.values().all(|a| budget.take(a.bytes.len()))
            && self.fonts.values().all(|f| budget.take(f.len()))
            && self
                .theme
                .iter()
                .chain(&self.pack)
                .all(|f| budget.take_json(f.manifest()) && f.widgets().values().all(|w| budget.take(w.len())));
        if fits { Ok(()) } else { Err(output_too_large(limits.max_output_bytes)) }
    }
}

/// Un plan dont la sortie tient dans les budgets : seul [`ImportPlan::check_budget`] en rend un.
///
/// Il se lit comme un [`ImportPlan`] ; on peut en retirer les ops, le thème, le pack et les
/// polices, ou annoter le rapport, jamais y ajouter : il reste dans les budgets vérifiés.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckedPlan(ImportPlan);

impl Deref for CheckedPlan {
    type Target = ImportPlan;

    fn deref(&self) -> &ImportPlan {
        &self.0
    }
}

impl CheckedPlan {
    /// Retire les opérations, à appliquer dans l'ordre en une transaction.
    pub fn take_ops(&mut self) -> Vec<Op> {
        std::mem::take(&mut self.0.ops)
    }

    /// Retire le thème généré (hi-fi).
    pub fn take_theme(&mut self) -> Option<PackFiles> {
        self.0.theme.take()
    }

    /// Retire le pack généré (hi-fi).
    pub fn take_pack(&mut self) -> Option<PackFiles> {
        self.0.pack.take()
    }

    /// Retire les polices à embarquer (hi-fi).
    pub fn take_fonts(&mut self) -> BTreeMap<String, Vec<u8>> {
        std::mem::take(&mut self.0.fonts)
    }

    /// Le rapport, à compléter après le rendu (débordements, écart à la référence).
    pub fn report_mut(&mut self) -> &mut ImportReport {
        &mut self.0.report
    }
}

/// Ce qui reste du budget de sortie ; une écriture au-delà échoue (arrêt de la sérialisation).
struct Budget {
    left: u64,
}

impl Budget {
    /// Retire `n` octets ; faux s'il n'en reste pas assez.
    fn take(&mut self, n: usize) -> bool {
        u64::try_from(n).is_ok_and(|n| self.take_u64(n))
    }

    /// Retire `n` octets ; faux s'il n'en reste pas assez.
    fn take_u64(&mut self, n: u64) -> bool {
        match self.left.checked_sub(n) {
            Some(left) => {
                self.left = left;
                true
            }
            None => false,
        }
    }

    /// Retire la taille de `v` en JSON compact, sans le matérialiser ; faux au-delà, ou si
    /// `v` ne se sérialise pas : un plan dont on ne peut mesurer la sortie est refusé.
    fn take_json(&mut self, v: &impl Serialize) -> bool {
        serde_json::to_writer(&mut *self, v).is_ok()
    }
}

impl io::Write for Budget {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.take(buf.len()) { Ok(buf.len()) } else { Err(io::ErrorKind::FileTooLarge.into()) }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod plan_tests;
