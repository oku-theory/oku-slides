use serde::ser::{Error as _, Serialize, Serializer};
use speculoos::prelude::*;
use tokio::sync::mpsc;

use super::emit;

/// Un évènement qui ne se sérialise jamais.
struct Broken;

impl Serialize for Broken {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(S::Error::custom("cassé"))
    }
}

#[test]
fn emit_skips_an_event_that_does_not_serialize() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    emit(&tx, "agent", "t-1", &Broken);
    assert_that!(rx.try_recv().is_err()).is_true();
}

#[test]
fn emit_wraps_the_event_with_its_tab() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    emit(
        &tx,
        "agent",
        "t-1",
        &crate::AgentEvent::Conversation { scope: oku_slides_session::agent::AgentScope::Slides },
    );
    let (channel, json) = rx.try_recv().unwrap();
    assert_that!(channel).is_equal_to("agent");
    assert_that!(json.as_str())
        .is_equal_to(r#"{"tab":"t-1","event":{"kind":"conversation","scope":{"kind":"slides"}}}"#);
}
