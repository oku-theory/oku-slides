//! Valeurs d'exemple d'un widget : contrôle, et réglages de l'hôte.
//!
//! `null` ou la valeur de l'aperçu effacent une valeur ; un texte ou une liste vidés
//! restent vides (leur écriture : `widget_examples.rs`). Les cellules d'une liste ont
//! les bornes de taille du cœur (`oku_slides_core::ListSpec`). Les réglages de l'hôte
//! (`widget_settings`) montrent un champ par paramètre réglable, à sa valeur d'exemple
//! (les champs : `param_fields.rs`).

use oku_slides_core::{ParamSchema, ParamValue, param_value, text_fits};
use serde_json::{Map, Value};
use snafu::{ResultExt, ensure};

use crate::agent::ShowParams;
use crate::error::{NotEditingTemplatesSnafu, TrialEncodeSnafu, TrialFixedSnafu, TrialTooLongSnafu, TrialValueSnafu};
use crate::panel::HostParams;
use crate::param_fields::{coerce, field, label, settable};
use crate::widget_preview::preview_params;
use crate::widgets_edit::parse_name;
use crate::{Result, Session};

impl Session {
    /// Réglages du widget `name`, à ses valeurs d'exemple ; `None` pour Commun.
    pub(crate) fn widget_settings(&self, name: &str) -> Result<Option<HostParams>> {
        let file = parse_name(name)?;
        ensure!(self.in_templates(), NotEditingTemplatesSnafu);
        if file.is_common() {
            return Ok(None);
        }
        let l = &self.localizer;
        let title = l.tr_with("session-widget-params", &[("name", file.as_str().into())]);
        let Some(schema) = self.valid_schema(&file)? else {
            let spec = ShowParams { title, slide: String::new(), fields: Vec::new() };
            return Ok(Some(HostParams { spec, keys: Vec::new(), notes: vec![l.tr("session-widget-invalid-file")] }));
        };
        let shown = preview_params(&schema, &self.preview_inputs(None, &file, &schema, &Map::new())?, l);
        let (mut fields, mut keys, mut notes) = (Vec::new(), Vec::new(), Vec::new());
        for p in &schema.params {
            match field(p, shown.get(&p.name), l) {
                Some(f) => {
                    keys.push(p.name.clone());
                    fields.push(f);
                }
                None => notes.push(l.tr_with("session-widget-param-fixed", &[("name", label(p).into())])),
            }
        }
        if schema.params.is_empty() {
            notes.push(l.tr("session-widget-no-params"));
        }
        Ok(Some(HostParams { spec: ShowParams { title, slide: String::new(), fields }, keys, notes }))
    }
}

/// La valeur d'exemple retenue pour `p` ; `None` efface la valeur.
pub(crate) fn checked_value(p: &ParamSchema, value: Value) -> Result<Option<Value>> {
    let param = p.name.as_str();
    ensure!(settable(&p.ty), TrialFixedSnafu { param });
    let Some(value) = coerce(p, value) else { return Ok(None) };
    // Des paragraphes vides : `param_value` ne donne rien, la valeur s'efface.
    let Some(checked) = param_value(&p.ty, param, value).context(TrialValueSnafu { param })? else { return Ok(None) };
    // Comme le cœur, la taille se mesure sur la valeur convertie.
    ensure!(text_fits(&p.ty, &checked), TrialTooLongSnafu { param });
    Ok(serde_json::to_value(checked).map(Some).context(TrialEncodeSnafu { param })?)
}

/// La valeur d'exemple est-elle celle de l'aperçu ? Les nombres se comparent comme nombres.
pub(crate) fn same_as(neutral: Option<&ParamValue>, v: &Value) -> bool {
    let Some(n) = neutral.and_then(|n| serde_json::to_value(n).ok()) else { return false };
    match (n.as_f64(), v.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => &n == v,
    }
}

#[cfg(test)]
#[path = "widget_params_tests.rs"]
mod widget_params_tests;
