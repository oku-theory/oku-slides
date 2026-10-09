//! Trames du tube : longueur de l'en-tête (u32 big-endian), en-tête JSON,
//! longueur de la charge (u32 big-endian), charge brute.
//!
//! Bornes par partie et par sorte ([`FrameLimits`]), tirées de `max_frame_mib` :
//! un en-tête (JSON : deck, pack, mise en page) au plus un huitième, un SVG au
//! plus un quart, une police, un asset, un PNG ou un PDF la taille entière ;
//! les autres réponses n'ont pas de charge.

use std::io::{Read, Write};

use serde::Serialize;
use snafu::{ResultExt, ensure};

use crate::error::{FrameError, IoSnafu, JsonSnafu, TooLargeSnafu, TruncatedSnafu};
use crate::protocol::Request;

/// Un en-tête de trame : au plus `max / HEADER_SHARE`.
const HEADER_SHARE: usize = 8;
/// La charge d'un SVG : au plus `max / SVG_SHARE`.
const SVG_SHARE: usize = 4;

/// Bornes d'une trame, en octets : l'en-tête et la charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameLimits {
    pub(crate) header: usize,
    pub(crate) payload: usize,
}

impl FrameLimits {
    /// La même borne `max` pour les deux parties.
    pub(crate) fn uniform(max: usize) -> Self {
        Self { header: max, payload: max }
    }

    /// Une requête de l'hôte : l'en-tête au plus `max / 8`, la charge (police, asset) `max`.
    pub(crate) fn requests(max: usize) -> Self {
        Self { header: max / HEADER_SHARE, payload: max }
    }

    /// La réponse à `req` : l'en-tête au plus `max / 8` ; la charge au plus
    /// `max / 4` pour un SVG, `max` pour un PNG ou un PDF, rien sinon.
    pub(crate) fn reply_to(req: &Request, max: usize) -> Self {
        let payload = match req {
            Request::Svg { .. } => max / SVG_SHARE,
            Request::Png { .. } | Request::Pdf { .. } => max,
            _ => 0,
        };
        Self { header: max / HEADER_SHARE, payload }
    }
}

/// Une trame lue : en-tête JSON non interprété et charge brute.
#[derive(Debug)]
pub(crate) struct Frame {
    pub(crate) header: Vec<u8>,
    pub(crate) payload: Vec<u8>,
}

/// Écrit une trame (en-tête sérialisé en JSON, puis charge) et vide le tampon.
///
/// Chaque partie est bornée par `limits` : au-delà, `TooLarge` avant tout
/// octet écrit (le flux reste utilisable).
pub(crate) fn write_frame(
    w: &mut impl Write,
    header: &impl Serialize,
    payload: &[u8],
    limits: FrameLimits,
) -> Result<(), FrameError> {
    let header = serde_json::to_vec(header).context(JsonSnafu)?;
    let mut lens = [0u32; 2];
    let parts = [(&header[..], limits.header, false), (payload, limits.payload, true)];
    for (len, (part, max, payload)) in lens.iter_mut().zip(parts) {
        let max = max.min(u32::MAX as usize);
        ensure!(part.len() <= max, TooLargeSnafu { len: part.len(), max, payload });
        *len = u32::try_from(part.len()).map_err(|_| TooLargeSnafu { len: part.len(), max, payload }.build())?;
    }
    for (len, part) in lens.iter().zip([&header[..], payload]) {
        w.write_all(&len.to_be_bytes()).context(IoSnafu)?;
        w.write_all(part).context(IoSnafu)?;
    }
    w.flush().context(IoSnafu)
}

/// Lit une trame ; `None` à la fin propre du flux, entre deux trames.
///
/// Chaque partie est bornée par `limits` avant toute allocation.
pub(crate) fn read_frame(r: &mut impl Read, limits: FrameLimits) -> Result<Option<Frame>, FrameError> {
    read_frame_with(r, || limits)
}

/// [`read_frame`], les bornes demandées à `limits` après le premier octet :
/// celles de la requête envoyée entre-temps, pour un lecteur qui attendait déjà.
pub(crate) fn read_frame_with(
    r: &mut impl Read,
    limits: impl FnOnce() -> FrameLimits,
) -> Result<Option<Frame>, FrameError> {
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
    let limits = limits();
    let header = read_part(r, u32::from_be_bytes(len) as usize, limits.header, false)?;
    read_exact(r, &mut len)?;
    let payload = read_part(r, u32::from_be_bytes(len) as usize, limits.payload, true)?;
    Ok(Some(Frame { header, payload }))
}

/// `payload` : la partie est la charge (sinon l'en-tête).
fn read_part(r: &mut impl Read, len: usize, max: usize, payload: bool) -> Result<Vec<u8>, FrameError> {
    ensure!(len <= max, TooLargeSnafu { len, max, payload });
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
