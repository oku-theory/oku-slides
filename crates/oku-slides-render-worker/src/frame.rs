//! Trames du tube : longueur de l'en-tête (u32 big-endian), en-tête JSON,
//! longueur de la charge (u32 big-endian), charge brute.

use std::io::{Read, Write};

use serde::Serialize;
use snafu::{ResultExt, ensure};

use crate::error::{FrameError, IoSnafu, JsonSnafu, TooLargeSnafu, TruncatedSnafu};

/// Une trame lue : en-tête JSON non interprété et charge brute.
#[derive(Debug)]
pub(crate) struct Frame {
    pub(crate) header: Vec<u8>,
    pub(crate) payload: Vec<u8>,
}

/// Écrit une trame (en-tête sérialisé en JSON, puis charge) et vide le tampon.
///
/// Chaque partie est bornée à `max` octets : au-delà, `TooLarge` avant tout
/// octet écrit (le flux reste utilisable).
pub(crate) fn write_frame(
    w: &mut impl Write,
    header: &impl Serialize,
    payload: &[u8],
    max: usize,
) -> Result<(), FrameError> {
    let header = serde_json::to_vec(header).context(JsonSnafu)?;
    let max = max.min(u32::MAX as usize);
    let mut lens = [0u32; 2];
    for (len, part) in lens.iter_mut().zip([&header[..], payload]) {
        ensure!(part.len() <= max, TooLargeSnafu { len: part.len(), max });
        *len = u32::try_from(part.len()).map_err(|_| TooLargeSnafu { len: part.len(), max }.build())?;
    }
    for (len, part) in lens.iter().zip([&header[..], payload]) {
        w.write_all(&len.to_be_bytes()).context(IoSnafu)?;
        w.write_all(part).context(IoSnafu)?;
    }
    w.flush().context(IoSnafu)
}

/// Lit une trame ; `None` à la fin propre du flux, entre deux trames.
///
/// Chaque partie est bornée à `max` octets avant toute allocation.
pub(crate) fn read_frame(r: &mut impl Read, max: usize) -> Result<Option<Frame>, FrameError> {
    let mut len = [0u8; 4];
    // Fin propre : aucun octet avant la trame. Un signal (EINTR) n'en est pas une.
    let first = loop {
        match r.read(&mut len[..1]) {
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            other => break other.context(IoSnafu)?,
        }
    };
    match first {
        0 => return Ok(None),
        _ => read_exact(r, &mut len[1..])?,
    }
    let header = read_part(r, u32::from_be_bytes(len) as usize, max)?;
    read_exact(r, &mut len)?;
    let payload = read_part(r, u32::from_be_bytes(len) as usize, max)?;
    Ok(Some(Frame { header, payload }))
}

fn read_part(r: &mut impl Read, len: usize, max: usize) -> Result<Vec<u8>, FrameError> {
    ensure!(len <= max, TooLargeSnafu { len, max });
    let mut buf = vec![0u8; len];
    read_exact(r, &mut buf)?;
    Ok(buf)
}

fn read_exact(r: &mut impl Read, buf: &mut [u8]) -> Result<(), FrameError> {
    match r.read_exact(buf) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Err(TruncatedSnafu.build()),
        Err(e) => Err(e).context(IoSnafu),
    }
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod frame_tests;
