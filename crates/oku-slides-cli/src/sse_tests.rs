use std::io::Read;

use speculoos::prelude::*;

use super::{Clients, MAX_SSE_CLIENTS};

#[test]
fn reader_gets_formatted_event_then_eof_when_clients_drop() {
    let clients = Clients::default();
    let mut reader = clients.subscribe().unwrap();
    clients.send("agent", r#"{"kind":"chunk","text":"a"}"#);
    drop(clients);
    let mut out = String::new();
    reader.read_to_string(&mut out).unwrap();
    assert_that!(out.as_str()).is_equal_to("event: agent\ndata: {\"kind\":\"chunk\",\"text\":\"a\"}\n\n");
}

#[test]
fn send_forgets_closed_readers() {
    let clients = Clients::default();
    drop(clients.subscribe().unwrap());
    clients.send("mcp", "{}");
    assert_that!(clients.len()).is_equal_to(0);
}

#[test]
fn subscribe_refuses_past_the_limit_and_frees_a_slot_when_a_reader_drops() {
    let clients = Clients::default();
    let mut readers: Vec<_> = (0..MAX_SSE_CLIENTS).map(|_| clients.subscribe().unwrap()).collect();
    assert_that!(clients.subscribe()).is_none();
    readers.pop();
    assert_that!(clients.subscribe()).is_some();
}
