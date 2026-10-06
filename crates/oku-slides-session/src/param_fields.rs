//! Champs de réglage d'un paramètre de widget, partagés par l'atelier (valeurs d'exemple) et le
//! panneau de la cible (valeurs du deck).

use oku_slides_a2ui::{ChoiceOption, ColumnKind, ListColumn};
use oku_slides_core::{LIST_ITEMS_MAX, ParamSchema, ParamType, ParamValue, default_items};
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};

use crate::agent::{FieldKind, ParamField};

pub(crate) fn label(p: &ParamSchema) -> String {
    p.label.clone().unwrap_or_else(|| p.name.clone())
}

/// Le nom montré du widget `kind` : son `label`, sinon le nom traduit d'un widget de base
/// (`session-widget-kind-<kind>`), sinon `kind` tel quel.
pub(crate) fn kind_name(kind: &str, label: Option<&str>, l: &Localizer) -> String {
    match label {
        Some(label) => label.to_owned(),
        None => l.try_tr(&format!("session-widget-kind-{kind}")).unwrap_or_else(|| kind.to_owned()),
    }
}

/// Le nom montré du paramètre `p` du widget `kind` : son `label`, sinon le nom traduit d'un
/// paramètre de widget de base (`session-param-<kind>-<nom>`, propre à chaque widget : `layout`
/// n'a pas les mêmes valeurs dans `content` et `stack`), sinon son nom tel quel.
pub(crate) fn param_name(kind: &str, p: &ParamSchema, l: &Localizer) -> String {
    match &p.label {
        Some(label) => label.clone(),
        None => l.try_tr(&format!("session-param-{kind}-{}", p.name)).unwrap_or_else(|| p.name.clone()),
    }
}

/// Le champ d'un paramètre réglable ; sa valeur est celle de l'aperçu.
///
/// Un `rich` montré est une liste de paragraphes : `params_with` en fait le markdown.
/// Une liste montre ses items (ceux d'essai, ou ceux du `default`, ids `d0`…).
pub(crate) fn field(p: &ParamSchema, shown: Option<&ParamValue>, l: &Localizer) -> Option<ParamField> {
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
            max: Some(max.unwrap_or(LIST_ITEMS_MAX) as f64),
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

/// La valeur saisie, ramenée à celle du schéma : `None` efface la valeur.
///
/// `null` ou un texte blanc effacent ; une sélection de `flag` renvoie `"true"`/`"false"` ; un
/// nombre se saisit en texte (s'il n'en est pas un, le texte reste, refusé plus loin).
pub(crate) fn coerce(p: &ParamSchema, value: Value) -> Option<Value> {
    Some(match (&p.ty, value) {
        (_, Value::Null) => return None,
        (_, Value::String(s)) if s.trim().is_empty() => return None,
        // Un champ de sélection renvoie le texte de l'option.
        (ParamType::Flag, Value::String(s)) if s == "true" || s == "false" => Value::Bool(s == "true"),
        // Un nombre sans bornes se règle dans un champ texte.
        (ParamType::Number { .. }, Value::String(s)) => {
            s.trim().parse::<f64>().ok().and_then(serde_json::Number::from_f64).map_or(Value::String(s), Value::Number)
        }
        (_, v) => v,
    })
}

/// Le `default` du schéma, lu en valeur ; `None` pour une liste (ses items sont amorcés à part,
/// `default_items`) ou sans défaut.
pub(crate) fn schema_default(p: &ParamSchema) -> Option<ParamValue> {
    match &p.ty {
        ParamType::List { .. } => None,
        _ => p.default.as_ref().filter(|v| !v.is_null()).and_then(|v| serde_json::from_value(v.clone()).ok()),
    }
}

#[cfg(test)]
#[path = "param_fields_tests.rs"]
mod param_fields_tests;
