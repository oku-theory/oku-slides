use std::sync::{Arc, Mutex, PoisonError};

use futures::{SinkExt, StreamExt};
use oku_slides_platform::telemetry::ACP_TARGET;
use rstest::rstest;
use speculoos::prelude::*;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use super::{incoming, masked, outgoing};

/// Records the message of each event on [`ACP_TARGET`].
#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<String>>>);

impl Visit for Seen {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push(format!("{value:?}"));
        }
    }
}

impl<S: tracing::Subscriber> Layer<S> for Seen {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        if event.metadata().target() == ACP_TARGET {
            event.record(&mut self.clone());
        }
    }
}

impl Seen {
    fn messages(&self) -> Vec<String> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

/// A `session/new` request carrying the MCP tools' bearer token.
const SESSION_NEW: &str = r#"{"jsonrpc":"2.0","id":1,"method":"session/new","params":{"cwd":"/tmp","mcpServers":[{"type":"http","name":"oku","url":"http://127.0.0.1:1/mcp","headers":[{"name":"Authorization","value":"Bearer s3cr3t"}]}]}}"#;

#[test]
fn masked_session_new_hides_the_authorization_value() {
    let line = masked(SESSION_NEW);
    assert_that!(line).does_not_contain("s3cr3t");
    assert_that!(line).contains(r#""value":"***""#);
}

#[test]
fn masked_session_new_keeps_the_other_fields() {
    assert_that!(masked(SESSION_NEW)).contains(r#""url":"http://127.0.0.1:1/mcp""#);
}

#[test]
fn masked_two_servers_hides_both_authorization_values() {
    let line = r#"{"headers":[{"name":"authorization","value":"one"}],"next":{"headers":[{"name":"Authorization","value":"two"}]}}"#;
    let line = masked(line);
    assert_that!(line).does_not_contain("one");
    assert_that!(line).does_not_contain("two");
}

#[rstest]
#[case(r#"{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"s"}}"#)]
#[case("not json at all")]
fn masked_line_without_header_is_unchanged(#[case] line: &str) {
    assert_that!(masked(line)).is_equal_to(line.to_owned());
}

#[tokio::test]
async fn outgoing_line_written_with_newline_and_logged_masked() {
    let seen = Seen::default();
    let _guard = tracing::subscriber::set_default(tracing_subscriber::registry().with(seen.clone()));
    let (ours, theirs) = tokio::io::duplex(4096);
    let mut sink = Box::pin(outgoing(ours.compat_write(), 7));
    sink.send(SESSION_NEW.to_owned()).await.expect("send");
    let mut read = String::new();
    BufReader::new(theirs).read_line(&mut read).await.expect("read");
    assert_that!(read).is_equal_to(format!("{SESSION_NEW}\n"));
    assert_that!(seen.messages()).is_equal_to(vec![format!("→ {}", masked(SESSION_NEW))]);
}

#[tokio::test]
async fn outgoing_line_logged_without_the_token() {
    let seen = Seen::default();
    let _guard = tracing::subscriber::set_default(tracing_subscriber::registry().with(seen.clone()));
    let (ours, _theirs) = tokio::io::duplex(4096);
    let mut sink = Box::pin(outgoing(ours.compat_write(), 7));
    sink.send(SESSION_NEW.to_owned()).await.expect("send");
    assert_that!(seen.messages().join("\n")).does_not_contain("s3cr3t");
}

#[tokio::test]
async fn incoming_line_read_and_logged() {
    let seen = Seen::default();
    let _guard = tracing::subscriber::set_default(tracing_subscriber::registry().with(seen.clone()));
    let (ours, mut theirs) = tokio::io::duplex(4096);
    theirs.write_all(b"{\"id\":1,\"result\":{}}\n").await.expect("write");
    let mut lines = incoming(ours.compat(), 7);
    let line = lines.next().await.expect("a line").expect("ok");
    assert_that!(line).is_equal_to(r#"{"id":1,"result":{}}"#.to_owned());
    assert_that!(seen.messages()).is_equal_to(vec![r#"← {"id":1,"result":{}}"#.to_owned()]);
}
