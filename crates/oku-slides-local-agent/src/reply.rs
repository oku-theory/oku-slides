//! Lecture de la réponse du modèle : texte libre ou appel `<tool_call>`.

use serde_json::{Map, Value};

use crate::engine::Reply;

const OPEN: &str = "<tool_call>";
const CLOSE: &str = "</tool_call>";

/// Texte libre, appel d'outil, ou appel mal formé avec la raison.
pub(crate) fn parse(text: &str) -> Reply {
    let Some(start) = text.find(OPEN) else {
        return Reply::Text(text.trim().to_owned());
    };
    let rest = &text[start + OPEN.len()..];
    let body = rest.find(CLOSE).map_or(rest, |end| &rest[..end]);
    match call(body.trim()) {
        Ok((name, args)) => Reply::Call { name, args },
        Err(reason) => Reply::Invalid(reason),
    }
}

fn call(body: &str) -> Result<(String, Value), String> {
    let value: Value = serde_json::from_str(body).map_err(|e| format!("not valid JSON: {e}"))?;
    let Value::Object(mut object) = value else {
        return Err("the tool call must be a JSON object".to_owned());
    };
    let Some(Value::String(name)) = object.remove("name") else {
        return Err("the tool call needs a string \"name\"".to_owned());
    };
    let args = match object.remove("arguments") {
        None | Some(Value::Null) => Value::Object(Map::new()),
        Some(args @ Value::Object(_)) => args,
        Some(_) => return Err("\"arguments\" must be a JSON object".to_owned()),
    };
    Ok((name, args))
}

#[cfg(test)]
#[path = "reply_tests.rs"]
mod reply_tests;
