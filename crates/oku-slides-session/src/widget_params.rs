//! Valeurs d'exemple d'un widget : contrôle, et réglages de l'hôte.
//!
//! `null` ou la valeur de l'aperçu effacent une valeur ; dans un cas d'exemple, un
//! texte vidé reste vide (son écriture : `widget_examples.rs`). Les réglages de l'hôte
//! (`widget_settings`) montrent un champ par paramètre réglable, à la valeur
//! du premier cas d'exemple.

use oku_slides_a2ui::{ChoiceOption, ColumnKind, ListColumn};
use oku_slides_core::{ParamSchema, ParamType, ParamValue, default_items, param_value};
use oku_slides_i18n::Localizer;
use serde_json::{Map, Value, json};
use snafu::{ResultExt, ensure};

use crate::agent::{FieldKind, ParamField, ShowParams};
use crate::error::{NotEditingTemplatesSnafu, TrialEncodeSnafu, TrialFixedSnafu, TrialTooLongSnafu, TrialValueSnafu};
use crate::panel::HostParams;
use crate::widget_preview::preview_params;
use crate::widgets_edit::parse_name;
use crate::{Result, Session};

/// Longueur d'un texte `plain` d'exemple, en octets.
const PLAIN_MAX: usize = 4096;
/// Paragraphes d'un `rich` d'exemple.
const RICH_PARAGRAPHS_MAX: usize = 64;
/// Taille JSON d'un `rich` d'exemple, en octets.
const RICH_BYTES_MAX: usize = 16384;

impl Session {
    /// Réglages du widget `name`, aux valeurs de son cas `example` (le premier
    /// pour `None`) ; `None` pour Commun.
    pub(crate) fn widget_settings(&self, name: &str, example: Option<&str>) -> Result<Option<HostParams>> {
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
        let shown = preview_params(&schema, &self.preview_inputs(&file, &schema, example, &Map::new())?, l);
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

fn label(p: &ParamSchema) -> String {
    p.label.clone().unwrap_or_else(|| p.name.clone())
}

/// Le champ d'un paramètre réglable ; sa valeur est celle de l'aperçu.
///
/// Un `rich` montré est une liste de paragraphes : `params_with` en fait le markdown.
/// Une liste montre ses items (ceux d'essai, ou ceux du `default`, ids `d0`…).
fn field(p: &ParamSchema, shown: Option<&ParamValue>, l: &Localizer) -> Option<ParamField> {
    if !settable(&p.ty) {
        return None;
    }
    if let ParamType::List { item, item_label, min, max } = &p.ty {
        let value = match shown {
            Some(v) => serde_json::to_value(v).unwrap_or(Value::Null),
            None => default_items(p).ok().and_then(|items| serde_json::to_value(items).ok()).unwrap_or(Value::Null),
        };
        return Some(ParamField {
            label: label(p),
            kind: FieldKind::List,
            value: if value.is_array() { value } else { json!([]) },
            op_template: Value::Null,
            min: Some(*min as f64),
            max: max.map(|m| m as f64),
            step: None,
            options: Vec::new(),
            columns: item.iter().filter_map(column).collect(),
            item_label: item_label.clone(),
        });
    }
    let option = |id: &str, label: &str| ChoiceOption { id: id.to_owned(), label: label.to_owned(), hint: None };
    let (kind, min, max, step, options) = match &p.ty {
        ParamType::Rich => (FieldKind::Rich, None, None, None, Vec::new()),
        // Un curseur pour une vraie étendue ; des bornes vides ou inversées se règlent au clavier.
        ParamType::Number { min: Some(a), max: Some(b) } if a < b => {
            (FieldKind::Slider, Some(*a), Some(*b), Some(slider_step(b - a)), Vec::new())
        }
        ParamType::Choice { values } => {
            (FieldKind::Select, None, None, None, values.iter().map(|v| option(v, v)).collect())
        }
        ParamType::Flag => (
            FieldKind::Select,
            None,
            None,
            None,
            vec![option("true", &l.tr("session-widget-param-yes")), option("false", &l.tr("session-widget-param-no"))],
        ),
        // `plain`, et un nombre sans bornes (lu comme un nombre, `checked_value`).
        _ => (FieldKind::Text, None, None, None, Vec::new()),
    };
    let value = match (&p.ty, shown) {
        (ParamType::Number { .. }, Some(ParamValue::Value(v))) if kind == FieldKind::Slider => v.clone(),
        (ParamType::Number { .. }, Some(ParamValue::Value(v))) => json!(v.to_string()),
        (_, Some(ParamValue::Value(Value::Bool(b)))) => json!(b.to_string()),
        (_, Some(v)) => serde_json::to_value(v).unwrap_or(Value::Null),
        (_, None) => Value::Null,
    };
    Some(ParamField {
        label: label(p),
        kind,
        value,
        op_template: Value::Null,
        min,
        max,
        step,
        options,
        columns: Vec::new(),
        item_label: None,
    })
}

/// La colonne du tableau d'une liste pour une colonne de son schéma ; `None`
/// pour un type qu'une colonne n'admet pas (refusé à la validation du schéma).
fn column(c: &ParamSchema) -> Option<ListColumn> {
    let (kind, options) = match &c.ty {
        ParamType::Plain => (ColumnKind::Text, Vec::new()),
        ParamType::Rich => (ColumnKind::Rich, Vec::new()),
        ParamType::Number { .. } => (ColumnKind::Number, Vec::new()),
        ParamType::Flag => (ColumnKind::Flag, Vec::new()),
        ParamType::Choice { values } => (
            ColumnKind::Select,
            values.iter().map(|v| ChoiceOption { id: v.clone(), label: v.clone(), hint: None }).collect(),
        ),
        ParamType::Asset => (ColumnKind::Asset, Vec::new()),
        ParamType::Slot { .. } | ParamType::Grid | ParamType::Host { .. } | ParamType::List { .. } => return None,
    };
    Some(ListColumn { key: c.name.clone(), label: label(c), kind, options })
}

/// Le pas d'un curseur sur une étendue `span > 0` : cent pas sur une petite étendue (opacité…),
/// sinon des unités. Arrondi à six chiffres significatifs : `0.7 / 100` donne `0.007`, pas
/// `0.006999999999999999`.
fn slider_step(span: f64) -> f64 {
    if span > 10.0 {
        return 1.0;
    }
    let raw = span / 100.0;
    // `raw` ≤ 0.1 : l'échelle est une puissance de dix positive, exacte en flottant.
    let scale = 10f64.powi(5 - raw.log10().floor() as i32);
    if !scale.is_finite() {
        return raw;
    }
    (raw * scale).round() / scale
}

/// Vrai pour un type réglable dans l'aperçu.
pub(crate) fn settable(ty: &ParamType) -> bool {
    !matches!(ty, ParamType::Asset | ParamType::Slot { .. } | ParamType::Grid | ParamType::Host { .. })
}

/// La valeur d'exemple retenue pour `p` ; `None` efface la valeur.
pub(crate) fn checked_value(p: &ParamSchema, value: Value) -> Result<Option<Value>> {
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
    // Des paragraphes vides : `param_value` ne donne rien, la valeur s'efface.
    let Some(checked) = param_value(&p.ty, param, value).context(TrialValueSnafu { param })? else { return Ok(None) };
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
