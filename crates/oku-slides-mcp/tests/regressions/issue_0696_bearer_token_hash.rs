//! t-696 : le jeton du serveur d'outils est vérifié par comparaison de condensats.
//!
//! Tests de caractérisation (exception TDD déclarée) : verts avant et après le
//! changement, car le défaut est temporel et ne se teste pas. Ils figent le
//! contrat : seul l'en-tête exact est accepté.

use std::time::Duration;

use speculoos::prelude::*;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

use crate::common::{Harness, start};

fn port(h: &Harness) -> u16 {
    h.server
        .url()
        .trim_start_matches("http://127.0.0.1:")
        .trim_end_matches("/mcp")
        .parse()
        .unwrap_or_else(|e| panic!("port de {} : {e}", h.server.url()))
}

/// Code de statut de la réponse à un `GET /mcp` dont l'en-tête `Authorization`
/// vaut `auth` (octets bruts), ou sans cet en-tête si `None`.
async fn status(port: u16, auth: Option<&[u8]>) -> u16 {
    let mut request = format!("GET /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n").into_bytes();
    if let Some(value) = auth {
        request.extend_from_slice(b"Authorization: ");
        request.extend_from_slice(value);
        request.extend_from_slice(b"\r\n");
    }
    request.extend_from_slice(b"\r\n");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap_or_else(|e| panic!("connexion : {e}"));
    stream.write_all(&request).await.unwrap_or_else(|e| panic!("écriture : {e}"));
    let mut head = Vec::new();
    let mut buf = [0u8; 512];
    let read = tokio::time::timeout(Duration::from_secs(5), async {
        while !head.windows(2).any(|w| w == b"\r\n") {
            match stream.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => head.extend_from_slice(&buf[..n]),
            }
        }
    })
    .await;
    assert_that!(read.is_ok()).named("réponse reçue").is_true();
    let line = String::from_utf8_lossy(&head);
    line.split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("ligne de statut illisible : {line:?}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_server_exact_authorization_accepted() {
    let h = start().await;
    let auth = h.server.authorization();
    assert_that!(status(port(&h), Some(auth.as_bytes())).await).is_not_equal_to(401);
}

#[tokio::test(flavor = "multi_thread")]
async fn tool_server_wrong_authorization_refused() {
    let h = start().await;
    let port = port(&h);
    let auth = h.server.authorization();
    let last = auth.chars().last().unwrap_or('0');
    let flipped = if last == '0' { '1' } else { '0' };
    let same_length_wrong = format!("{}{flipped}", &auth[..auth.len() - 1]);
    let prefix = &auth[..auth.len() - 1];
    let longer = format!("{auth}0");
    let lowercase = auth.replacen("Bearer", "bearer", 1);
    let no_scheme = auth.trim_start_matches("Bearer ").to_owned();
    // hyper retire les espaces finaux d'une valeur d'en-tête avant le middleware :
    // « espace final » se teste donc par un espace en trop après le schéma.
    let extra_space = auth.replacen("Bearer ", "Bearer  ", 1);
    let mut non_ascii = auth.clone().into_bytes();
    non_ascii.extend_from_slice(&[0xC3, 0xA9]);
    let mut non_ascii_inside = auth.clone().into_bytes();
    non_ascii_inside[10] = 0xFF;

    let cases: [(&str, Option<&[u8]>); 9] = [
        ("même longueur, faux", Some(same_length_wrong.as_bytes())),
        ("préfixe du jeton", Some(prefix.as_bytes())),
        ("un caractère de trop", Some(longer.as_bytes())),
        ("valeur vide", Some(b"")),
        ("en-tête absent", None),
        ("bearer en minuscules", Some(lowercase.as_bytes())),
        ("sans schéma", Some(no_scheme.as_bytes())),
        ("espace en trop après le schéma", Some(extra_space.as_bytes())),
        ("octets non ASCII en fin", Some(&non_ascii)),
    ];
    for (name, value) in cases {
        assert_that!(status(port, value).await).named(name).is_equal_to(401);
    }
    assert_that!(status(port, Some(&non_ascii_inside)).await).named("octet non ASCII au milieu").is_equal_to(401);
}
