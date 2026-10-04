//! Valeurs d'essai d'un widget : contrôle et écriture.
//!
//! Tout ou rien ; `null`, un texte vide ou la valeur de l'aperçu effacent
//! l'essai. Les réglages de l'hôte (phase B) s'y ajoutent.

use oku_slides_core::{ParamSchema, ParamType, ParamValue, param_value};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use crate::agent::context::clipped;
use crate::error::{
    NotEditingTemplatesSnafu, TrialEncodeSnafu, TrialFixedSnafu, TrialTooLongSnafu, TrialUndeclaredSnafu,
    TrialValueSnafu, UnknownWidgetSnafu, WidgetInvalidSnafu,
};
use crate::widget_preview::preview_params;
use crate::widgets_edit::parse_name;
use crate::{Result, Session};

/// Longueur d'un texte `plain` d'essai, en octets.
const PLAIN_MAX: usize = 4096;
/// Paragraphes d'un `rich` d'essai.
const RICH_PARAGRAPHS_MAX: usize = 64;
/// Taille JSON d'un `rich` d'essai, en octets.
const RICH_BYTES_MAX: usize = 16384;

impl Session {
    /// Règle des essais, tout ou rien ; les essais retenus de `name`.
    pub fn set_widget_trials(&mut self, name: &str, values: Map<String, Value>) -> Result<Map<String, Value>> {
        let file = parse_name(name)?;
        ensure!(self.scratch().is_some(), NotEditingTemplatesSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        let neutral = preview_params(&schema, &Map::new(), &self.localizer);
        let mut planned = Vec::with_capacity(values.len());
        for (param, value) in values {
            let p = schema.param(&param).context(TrialUndeclaredSnafu { param: clipped(&param, 80) })?;
            let kept = trial_value(p, value)?.filter(|v| !same_as(neutral.get(&param), v));
            planned.push((param, kept));
        }
        let scratch = self.scratch_mut().context(NotEditingTemplatesSnafu)?;
        for (param, kept) in planned {
            scratch.set_trial(&file, &param, kept);
        }
        Ok(scratch.trials_of(&file).cloned().unwrap_or_default())
    }
}

/// Vrai pour un type réglable dans l'aperçu.
pub(crate) fn settable(ty: &ParamType) -> bool {
    !matches!(ty, ParamType::Asset | ParamType::Slot { .. } | ParamType::Grid | ParamType::Host { .. })
}

/// La valeur d'essai retenue pour `p` ; `None` efface l'essai.
fn trial_value(p: &ParamSchema, value: Value) -> Result<Option<Value>> {
    let param = p.name.as_str();
    ensure!(settable(&p.ty), TrialFixedSnafu { param });
    let value = match (&p.ty, value) {
        (_, Value::Null) => return Ok(None),
        (_, Value::String(s)) if s.trim().is_empty() => return Ok(None),
        // Un champ de sélection renvoie le texte de l'option.
        (ParamType::Flag, Value::String(s)) if s == "true" || s == "false" => Value::Bool(s == "true"),
        // Un nombre sans bornes se règle dans un champ texte.
        (ParamType::Number { .. }, Value::String(s)) => {
            s.trim().parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(Value::String(s), Value::Number)
        }
        (_, v) => v,
    };
    let too_long = match (&p.ty, &value) {
        (ParamType::Plain, Value::String(s)) => s.len() > PLAIN_MAX,
        (ParamType::Rich, Value::Array(ps)) => {
            ps.len() > RICH_PARAGRAPHS_MAX || serde_json::to_vec(&value).map_or(true, |b| b.len() > RICH_BYTES_MAX)
        }
        _ => false,
    };
    ensure!(!too_long, TrialTooLongSnafu { param });
    // Des paragraphes vides : `param_value` ne donne rien, l'essai s'efface.
    let Some(checked) = param_value(&p.ty, param, value).context(TrialValueSnafu { param })? else { return Ok(None) };
    Ok(serde_json::to_value(checked).map(Some).context(TrialEncodeSnafu { param })?)
}

/// La valeur d'essai est-elle celle de l'aperçu ? Les nombres se comparent comme nombres.
fn same_as(neutral: Option<&ParamValue>, v: &Value) -> bool {
    let Some(n) = neutral.and_then(|n| serde_json::to_value(n).ok()) else { return false };
    match (n.as_f64(), v.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => &n == v,
    }
}

#[cfg(test)]
#[path = "widget_params_tests.rs"]
mod widget_params_tests;
