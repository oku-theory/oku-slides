//! Contrôles de la surface `host-theme` : les champs d'un jeton selon son
//! contrôle ([`token_fields`]), et la valeur du jeton qu'un champ envoie
//! ([`token_value`]).
//!
//! Un nombre qui renvoie à un autre jeton (`size.lead` → `subtitle`) se
//! montre en texte ; un texte numérique revient en nombre. Les familles de
//! police et la palette sont des tableaux à une colonne texte (une famille, ou
//! une couleur hex ou nommée) : leurs changements (`ListOp`, ou `replace` au
//! collage) s'appliquent à la liste du brouillon, adressée par index. Un
//! filet de titre a un champ par côté, envoyé sous `<chemin>.<côté>`.

use oku_slides_a2ui::{ChoiceOption, ColumnKind, ListColumn};
use oku_slides_core::ListOp;
use oku_slides_i18n::Localizer;
use serde_json::{Map, Value, json};

use crate::agent::{FieldKind, ParamField};
use crate::error::{InvalidTokenValueSnafu, Result};
use crate::theme_schema::{Control, RULE_MAX, TokenSpec, spec};

/// Côtés d'un filet de titre, dans l'ordre des champs.
const RULE_SIDES: [&str; 3] = ["width", "height", "color"];
/// Pas d'un curseur en em sans pas donné par le schéma.
const EM_STEP: f64 = 0.05;

/// Un champ d'une ligne : sa clé (composant et modèle de données), le chemin
/// envoyé par son action et le champ à rendre.
pub(crate) struct TokenField {
    pub key: String,
    pub path: Vec<String>,
    pub field: ParamField,
}

fn param(label: String, kind: FieldKind, value: Value) -> ParamField {
    ParamField {
        label,
        kind,
        value,
        op_template: Value::Null,
        min: None,
        max: None,
        step: None,
        options: Vec::new(),
        columns: Vec::new(),
        item_label: None,
    }
}

fn option(id: &str, label: String) -> ChoiceOption {
    ChoiceOption { id: id.to_owned(), label, hint: None }
}

/// Un nombre en pt, en em ou sans unité : compteur (pt entier), curseur, ou
/// texte s'il renvoie à un autre jeton (ou n'est pas entier, en pt).
fn number(label: String, control: &Control, value: Option<&Value>) -> ParamField {
    let (min, max, step, stepper) = match control {
        Control::Pt { min, max, .. } => (*min, *max, None, true),
        Control::Em { min, max, .. } => (*min, *max, Some(EM_STEP), false),
        Control::Number { min, max, step } => (*min, *max, Some(*step), false),
        _ => (0.0, 0.0, None, false),
    };
    let n = match value {
        None | Some(Value::Null) => Some(min),
        Some(v) => v.as_f64(),
    };
    let field = match n {
        Some(n) if !stepper || n.fract() == 0.0 => {
            let kind = if stepper { FieldKind::Stepper } else { FieldKind::Slider };
            param(label, kind, normalized(n))
        }
        _ => param(label, FieldKind::Text, json!(value.map(text).unwrap_or_default())),
    };
    ParamField { min: Some(min), max: Some(max), step, ..field }
}

/// Une valeur en texte : une chaîne telle quelle, le reste en JSON.
fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Les lignes d'un tableau à une colonne `column` : `{id: index, column: entrée}`.
fn list_rows(value: Option<&Value>, column: &str) -> Value {
    let entries = value.and_then(Value::as_array).cloned().unwrap_or_default();
    Value::Array(entries.iter().enumerate().map(|(i, e)| json!({ "id": i.to_string(), column: text(e) })).collect())
}

fn list(label: String, value: Option<&Value>, column: &str, l: &Localizer) -> ParamField {
    let columns = vec![ListColumn {
        key: column.to_owned(),
        label: l.tr(&format!("theme-column-{column}")),
        kind: ColumnKind::Text,
        options: Vec::new(),
        list: None,
    }];
    ParamField { columns, min: Some(1.0), ..param(label, FieldKind::List, list_rows(value, column)) }
}

/// Le filet par défaut, quand on règle un côté d'un filet vidé.
fn default_rule() -> Value {
    json!({ "width": 80, "height": 4, "color": "accent" })
}

/// Un jeton à montrer : clé de base de ses champs, chemin, entrée du schéma,
/// libellé et valeur effective (`None` : absent du brouillon comme des défauts).
pub(crate) struct Shown<'a> {
    pub key: &'a str,
    pub path: &'a [String],
    pub spec: &'a TokenSpec,
    pub label: &'a str,
    pub value: Option<&'a Value>,
}

/// Les champs du jeton `t` ; `tokens` : les jetons effectifs, pour les noms de graisses.
pub(crate) fn token_fields(t: &Shown<'_>, tokens: &Map<String, Value>, l: &Localizer) -> Vec<TokenField> {
    let Shown { key, path, spec, value, .. } = *t;
    let label = t.label.to_owned();
    let single = |field: ParamField| vec![TokenField { key: key.to_owned(), path: path.to_vec(), field }];
    let shown = || json!(value.map(text).unwrap_or_default());
    match &spec.control {
        Control::Color => single(param(label, FieldKind::Color, shown())),
        Control::Text => single(param(label, FieldKind::Text, shown())),
        Control::Pt { .. } | Control::Em { .. } | Control::Number { .. } => single(number(label, &spec.control, value)),
        Control::Weight => {
            let mut options: Vec<ChoiceOption> =
                (1..=9).map(|w| (w * 100).to_string()).map(|w| option(&w, w.clone())).collect();
            if path.first().is_some_and(|g| g != "weight") {
                let names = tokens.get("weight").and_then(Value::as_object).into_iter().flatten();
                options.extend(names.map(|(name, _)| option(name, name.clone())));
            }
            single(ParamField { options, ..param(label, FieldKind::Select, shown()) })
        }
        Control::Choice { options } => {
            let options = options.iter().map(|o| option(o, l.tr(&format!("theme-option-{o}")))).collect();
            single(ParamField { options, ..param(label, FieldKind::Select, shown()) })
        }
        Control::Bool => {
            let options = ["true", "false"].map(|o| option(o, l.tr(&format!("theme-option-{o}")))).to_vec();
            let v = value.and_then(Value::as_bool).unwrap_or_default();
            single(ParamField { options, ..param(label, FieldKind::Select, json!(v.to_string())) })
        }
        Control::Fonts => single(list(label, value, "family", l)),
        Control::Palette => single(list(label, value, "color", l)),
        Control::Rule => {
            let rule = value.filter(|v| v.is_object()).cloned().unwrap_or_else(default_rule);
            RULE_SIDES
                .iter()
                .map(|side| {
                    let label = format!("{label} · {}", l.tr(&format!("theme-rule-{side}")));
                    let v = rule.get(*side);
                    let field = match *side {
                        "color" => param(label, FieldKind::Color, json!(v.map(text).unwrap_or_default())),
                        _ => number(label, &Control::Pt { min: 0.0, max: RULE_MAX, refs: None }, v),
                    };
                    let mut sub = path.to_vec();
                    sub.push((*side).to_owned());
                    TokenField { key: format!("{key}:{side}"), path: sub, field }
                })
                .collect()
        }
    }
}

/// Un nombre entier en entier JSON (`30`, pas `30.0`) : les comparaisons
/// avec la base restent justes.
fn normalized(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 1e15 { json!(n as i64) } else { json!(n) }
}

/// Un nombre, ou un texte numérique, en nombre ; le reste tel quel.
fn numeric(value: Value) -> Value {
    let n = match &value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok().filter(|n| n.is_finite()),
        _ => None,
    };
    n.map_or(value, normalized)
}

fn malformed(path: &[String]) -> crate::error::SessionError {
    InvalidTokenValueSnafu { path: path.join("."), reason: "wrong-type" }.build().into()
}

/// L'entrée d'une ligne de tableau : une chaîne, ou la cellule `column` d'un objet.
fn cell(entry: &Value, column: &str) -> Value {
    match entry {
        Value::Object(row) => row.get(column).cloned().unwrap_or(Value::Null),
        other => other.clone(),
    }
}

/// La liste du jeton après le changement `change` d'un tableau à une colonne.
fn list_value(path: &[String], change: Value, column: &str, current: Option<Value>) -> Result<Value> {
    let rows = match change {
        Value::Array(rows) => rows,
        Value::Object(ref o) if o.get("op").and_then(Value::as_str) == Some("replace") => {
            o.get("items").and_then(Value::as_array).cloned().ok_or_else(|| malformed(path))?
        }
        Value::Object(_) => {
            let op: ListOp = serde_json::from_value(change).map_err(|_| malformed(path))?;
            let list = current.and_then(|v| v.as_array().cloned()).unwrap_or_default();
            return apply_list_op(path, list, op, column).map(Value::Array);
        }
        other => return Ok(other),
    };
    Ok(Value::Array(rows.iter().map(|r| cell(r, column)).collect()))
}

/// Applique `op` à la liste `list`, dont les items ont pour id leur index.
fn apply_list_op(path: &[String], mut list: Vec<Value>, op: ListOp, column: &str) -> Result<Vec<Value>> {
    let index = |item: &str, len: usize| item.parse::<usize>().ok().filter(|i| *i < len).ok_or_else(|| malformed(path));
    match op {
        ListOp::Insert { index: at, values, .. } => {
            let at = at.unwrap_or(list.len()).min(list.len());
            // Une ligne neuve sans valeur reprend sa voisine : la liste reste valide.
            let neighbour = list.get(at.saturating_sub(1)).or(list.first()).cloned().unwrap_or(Value::Null);
            let value = values.get(column).cloned().filter(|v| !v.is_null()).unwrap_or(neighbour);
            list.insert(at, value);
        }
        ListOp::Remove { item } => {
            list.remove(index(&item, list.len())?);
        }
        ListOp::Move { item, to } => {
            let value = list.remove(index(&item, list.len())?);
            list.insert(to.min(list.len()), value);
        }
        ListOp::SetField { item, value, .. } => {
            let i = index(&item, list.len())?;
            match value {
                Value::Null => {
                    list.remove(i);
                }
                v => list[i] = v,
            }
        }
    }
    Ok(list)
}

/// Le jeton et sa valeur qu'envoie un champ au chemin `path` du schéma
/// `schema`, valeur `value` ; `current` lit la valeur effective d'un jeton (liste, filet).
pub(crate) fn token_value(
    schema: &[TokenSpec],
    path: Vec<String>,
    value: Value,
    current: impl Fn(&[String]) -> Option<Value>,
) -> Result<(Vec<String>, Value)> {
    if spec(schema, &path).is_none()
        && let Some((side, parent)) = path.split_last()
        && spec(schema, parent).is_some_and(|s| s.control == Control::Rule)
    {
        if !RULE_SIDES.contains(&side.as_str()) {
            return Err(malformed(&path));
        }
        let mut rule = current(parent).filter(Value::is_object).unwrap_or_else(default_rule);
        let v = if side == "color" { value } else { numeric(value) };
        rule[side.as_str()] = v;
        return Ok((parent.to_vec(), rule));
    }
    let Some(s) = spec(schema, &path) else { return Ok((path, value)) };
    let value = match &s.control {
        Control::Pt { .. } | Control::Em { .. } | Control::Number { .. } | Control::Weight => numeric(value),
        Control::Bool => match value.as_str() {
            Some("true") => Value::Bool(true),
            Some("false") => Value::Bool(false),
            _ => value,
        },
        Control::Fonts => list_value(&path, value, "family", current(&path))?,
        Control::Palette => list_value(&path, value, "color", current(&path))?,
        _ => value,
    };
    Ok((path, value))
}
