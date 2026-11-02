//! Reading Apple's guided reply: `{tool, arguments}` is a call, `{text}` is free text.

use serde_json::{Map, Value};

use crate::engine::Reply;

/// A call, a text, or an invalid reply with the reason in English.
pub(crate) fn parse(json: &str) -> Reply {
    let value: Value = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(e) => return Reply::Invalid(format!("not valid JSON: {e}")),
    };
    let Value::Object(mut object) = value else {
        return Reply::Invalid("the reply must be a JSON object".to_owned());
    };
    if let Some(tool) = object.remove("tool") {
        let Value::String(name) = tool else {
            return Reply::Invalid("\"tool\" must be a string".to_owned());
        };
        return match object.remove("arguments") {
            None | Some(Value::Null) => Reply::Call { name, args: Value::Object(Map::new()) },
            Some(args @ Value::Object(_)) => Reply::Call { name, args },
            Some(_) => Reply::Invalid("\"arguments\" must be a JSON object".to_owned()),
        };
    }
    match object.remove("text") {
        Some(Value::String(text)) => Reply::Text(text.trim().to_owned()),
        _ => Reply::Invalid("the reply needs a \"tool\" or a \"text\"".to_owned()),
    }
}

#[cfg(test)]
#[path = "apple_reply_tests.rs"]
mod apple_reply_tests;
