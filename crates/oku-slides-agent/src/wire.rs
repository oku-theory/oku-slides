//! The ACP transport over the agent's stdio, which logs every message on request.
//!
//! Each JSON-RPC line goes to [`ACP_TARGET`] at `debug` (turned on by
//! `OKU_SLIDES_DEBUG_ACP`): `→` to the agent, `←` from it, with the MCP tools'
//! `Authorization` header masked. Off, a line costs one filter check.

use std::io;

use agent_client_protocol::Lines;
use futures::io::BufReader;
use futures::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, Sink, Stream, TryStreamExt};
use oku_slides_platform::telemetry::ACP_TARGET;
use serde_json::Value;

/// What replaces a secret header value in the logs.
const MASK: &str = "***";

/// Newline-delimited JSON-RPC over the agent's stdio, as `ByteStreams` does, logged.
pub(crate) fn transport(
    outgoing_bytes: impl AsyncWrite + Send + 'static,
    incoming_bytes: impl AsyncRead + Send + 'static,
    generation: u64,
) -> Lines<impl Sink<String, Error = io::Error> + Send + 'static, impl Stream<Item = io::Result<String>> + Send + 'static>
{
    Lines::new(outgoing(outgoing_bytes, generation), incoming(incoming_bytes, generation))
}

/// Writes each line to the agent, newline-terminated and flushed, after logging it.
fn outgoing(
    writer: impl AsyncWrite + Send + 'static,
    generation: u64,
) -> impl Sink<String, Error = io::Error> + Send + 'static {
    futures::sink::unfold(Box::pin(writer), move |mut writer, line: String| async move {
        tracing::debug!(target: ACP_TARGET, generation, "→ {}", masked(&line));
        let mut bytes = line.into_bytes();
        bytes.push(b'\n');
        writer.write_all(&bytes).await?;
        writer.flush().await?;
        Ok(writer)
    })
}

/// The agent's lines, each logged as it arrives.
fn incoming(
    reader: impl AsyncRead + Send + 'static,
    generation: u64,
) -> impl Stream<Item = io::Result<String>> + Send + Unpin + 'static {
    Box::pin(BufReader::new(reader).lines().inspect_ok(move |line| {
        tracing::debug!(target: ACP_TARGET, generation, "← {}", masked(line));
    }))
}

/// `line` with every `Authorization` header value replaced by [`MASK`].
///
/// A line that is not JSON, or has no such header, is returned as is.
fn masked(line: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(line) else {
        return line.to_owned();
    };
    if mask(&mut value) { value.to_string() } else { line.to_owned() }
}

/// Masks `{"name": "Authorization", "value": …}` objects in `value`; whether it found one.
fn mask(value: &mut Value) -> bool {
    match value {
        Value::Object(object) => {
            let secret = object
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name.eq_ignore_ascii_case("authorization"));
            if secret && let Some(header) = object.get_mut("value") {
                *header = Value::String(MASK.to_owned());
                return true;
            }
            mask_all(object.values_mut())
        }
        Value::Array(items) => mask_all(items.iter_mut()),
        _ => false,
    }
}

/// Masks every value of `values`, not stopping at the first secret found.
fn mask_all<'a>(values: impl Iterator<Item = &'a mut Value>) -> bool {
    let mut found = false;
    for value in values {
        found |= mask(value);
    }
    found
}

#[cfg(test)]
#[path = "wire_tests.rs"]
mod wire_tests;
