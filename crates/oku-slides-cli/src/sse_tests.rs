use std::pin::Pin;

use axum::body::HttpBody;
use speculoos::prelude::*;

use super::{Clients, MAX_SSE_CLIENTS, SseBody};

/// Tout le flux, jusqu'à sa fin.
fn read_to_end(mut body: SseBody) -> String {
    let rt = tokio::runtime::Builder::new_current_thread().build().unwrap_or_else(|e| panic!("runtime : {e}"));
    rt.block_on(async {
        let mut out = Vec::new();
        while let Some(frame) = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await {
            let Ok(frame) = frame;
            if let Ok(data) = frame.into_data() {
                out.extend_from_slice(&data);
            }
        }
        String::from_utf8(out).unwrap_or_else(|e| panic!("utf-8 : {e}"))
    })
}

#[test]
fn reader_gets_formatted_event_then_eof_when_clients_drop() {
    let clients = Clients::default();
    let reader = clients.subscribe().unwrap();
    clients.send("agent", r#"{"kind":"chunk","text":"a"}"#);
    drop(clients);
    assert_that!(read_to_end(reader).as_str())
        .is_equal_to("event: agent\ndata: {\"kind\":\"chunk\",\"text\":\"a\"}\n\n");
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

#[test]
fn close_ends_every_stream_and_refuses_new_ones() {
    // L'arrêt gracieux du serveur attend la fin de chaque flux.
    let clients = Clients::default();
    let reader = clients.subscribe().unwrap();
    clients.send("agent", "{}");
    clients.close();
    assert_that!(read_to_end(reader).as_str()).is_equal_to("event: agent\ndata: {}\n\n");
    assert_that!(clients.subscribe()).is_none();
}

#[test]
fn send_drops_a_client_that_stops_reading() {
    // Sans limite, un client qui ne lit plus ferait grossir la mémoire ; l'EventSource se reconnecte.
    let clients = Clients::default();
    let _stalled = clients.subscribe().unwrap();
    for _ in 0..=super::CHANNEL_CAPACITY {
        clients.send("agent", "{}");
    }
    assert_that!(clients.len()).is_equal_to(0);
}
