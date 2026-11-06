//! SSIP, the Speech Dispatcher protocol, over any byte stream.
//!
//! Sections cited are those of the SSIP manual (`doc/ssip.texi` in brailcom/speechd):
//! - General Rules: UTF-8 commands, one CRLF line each; a reply is `NNN-data` lines closed by
//!   an `NNN text` line, whose code gives the result. The protocol is synchronous: one command,
//!   then its whole reply.
//! - Return Codes: `1xx` and `2xx` succeed, `3xx`, `4xx` and `5xx` fail; only their first digit
//!   is defined. `7xx` are event notifications, with well-defined codes.
//! - Events Notification in SSIP: `701` BEGIN, `702` END, `703` CANCELED, each with the message
//!   id then the client id as data lines; they arrive between replies, never inside one.
//!
//! The text is sent as data, never as an argument, and is never logged.

use std::collections::VecDeque;
use std::io::{self, ErrorKind, Read, Write};

use snafu::IntoError;

use super::VoiceInfo;
use crate::error::{SttError, VoiceFailedSnafu, VoiceIoSnafu, VoiceRefusedSnafu};

/// The application part of our client name.
const APPLICATION: &str = "oku-slides";

/// The largest reply buffered (bytes): a list of a few hundred voices takes a few KiB.
const MAX_REPLY_BYTES: usize = 1 << 20;

/// An SSIP connection: commands answered in order, events kept until read.
#[derive(Debug)]
pub(crate) struct Ssip<S> {
    stream: S,
    /// Bytes read but not yet parsed into a whole reply.
    buf: Vec<u8>,
    /// Events read while waiting for a command's reply.
    events: VecDeque<Event>,
}

/// What happened to a message (Types of Events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventKind {
    /// `701`: the message started to sound.
    Begin,
    /// `702`: the message was spoken to its end.
    End,
    /// `703`: the message was cancelled, spoken or not.
    Cancel,
}

/// One event notification about a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Event {
    /// The message id `SPEAK` returned.
    pub(crate) msg: u64,
    /// What happened to it.
    pub(crate) kind: EventKind,
}

/// One whole reply: its final code and its data lines (General Rules).
#[derive(Debug)]
struct Reply {
    code: u16,
    data: Vec<String>,
}

impl<S: Read + Write> Ssip<S> {
    /// Names the client `<user>:oku-slides:voice` and asks for END and CANCELED events.
    ///
    /// Parameter Setting: `SET self CLIENT_NAME` comes first, once, each part made only of
    /// alphanumerics, `-` and `_`. Switching Notifications On and Off: `SET self NOTIFICATION`.
    pub(crate) fn connect(stream: S, user: &str) -> Result<Self, SttError> {
        let mut ssip = Self { stream, buf: Vec::new(), events: VecDeque::new() };
        ssip.command(&format!("SET self CLIENT_NAME {}:{APPLICATION}:voice", name_part(user)), "CLIENT_NAME")?;
        ssip.command("SET self NOTIFICATION END on", "NOTIFICATION")?;
        ssip.command("SET self NOTIFICATION CANCEL on", "NOTIFICATION")?;
        Ok(ssip)
    }

    /// Speaks `text` with `voice` at `rate` (-100..=100) and returns the queued message id.
    ///
    /// `text` must hold something to say: the daemon queues even a lone empty line, and only a
    /// data part that is a lone `.` line queues nothing (and answers without an id).
    ///
    /// Parameter Setting: `SET self SYNTHESIS_VOICE` takes a name from `LIST SYNTHESIS_VOICES`,
    /// `SET self RATE` an integer in -100..=100. Speech Synthesis: `SPEAK` answers `230`, then
    /// takes the text closed by `CR LF . CR LF`, a line starting with `.` getting an extra `.`,
    /// and answers `225-<msg_id>` then `225 OK MESSAGE QUEUED`.
    pub(crate) fn speak(&mut self, text: &str, voice: &str, rate: i32) -> Result<u64, SttError> {
        // A voice is one argument on one line: no control character may end or split it.
        if voice.trim().is_empty() || voice.chars().any(char::is_control) {
            return Err(VoiceFailedSnafu { step: "SYNTHESIS_VOICE" }.build().into());
        }
        self.command(&format!("SET self SYNTHESIS_VOICE {voice}"), "SYNTHESIS_VOICE")?;
        self.command(&format!("SET self RATE {}", rate.clamp(-100, 100)), "RATE")?;
        self.command("SPEAK", "SPEAK")?;
        self.send(&body(text), "SPEAK")?;
        let reply = self.reply("SPEAK")?;
        let id = reply.data.first().and_then(|id| id.trim().parse().ok());
        id.ok_or_else(|| VoiceFailedSnafu { step: "SPEAK reply" }.build().into())
    }

    /// The voices of the current synthesizer.
    ///
    /// Retrieving Information: `LIST SYNTHESIS_VOICES` lists one voice per data line, its name,
    /// language and variant separated by tabs.
    pub(crate) fn voices(&mut self) -> Result<Vec<VoiceInfo>, SttError> {
        let reply = self.command("LIST SYNTHESIS_VOICES", "LIST SYNTHESIS_VOICES")?;
        Ok(reply
            .data
            .iter()
            .filter_map(|line| {
                let mut columns = line.split('\t');
                let name = columns.next().filter(|n| !n.trim().is_empty())?;
                let lang = columns.next().filter(|l| !l.trim().is_empty())?;
                Some(VoiceInfo { id: name.to_owned(), name: name.to_owned(), lang: lang.trim().to_owned() })
            })
            .collect())
    }

    /// The next event, or `None` when the stream's read timeout passed first.
    ///
    /// Command replies (such as the one to `CANCEL`) and other events are skipped; a partial
    /// notification is kept for the next call.
    pub(crate) fn read_event(&mut self) -> Result<Option<Event>, SttError> {
        loop {
            if let Some(event) = self.events.pop_front() {
                return Ok(Some(event));
            }
            let Some(reply) = self.next_reply()? else { return Ok(None) };
            if let Some(event) = event(&reply)? {
                return Ok(Some(event));
            }
        }
    }

    /// Closes the connection politely (Other Commands: `QUIT`); a failure changes nothing.
    pub(crate) fn quit(&mut self) {
        if let Err(e) = self.command("QUIT", "QUIT") {
            tracing::debug!(error = %e, "speech daemon QUIT");
        }
    }

    /// Sends `line` and returns its successful reply.
    fn command(&mut self, line: &str, step: &'static str) -> Result<Reply, SttError> {
        self.send(&format!("{line}\r\n"), step)?;
        self.reply(step)
    }

    fn send(&mut self, bytes: &str, step: &'static str) -> Result<(), SttError> {
        self.stream.write_all(bytes.as_bytes()).map_err(|e| VoiceIoSnafu { step }.into_error(e))?;
        self.stream.flush().map_err(|e| VoiceIoSnafu { step }.into_error(e).into())
    }

    /// The reply to the command just sent, events before it kept for [`Ssip::read_event`].
    fn reply(&mut self, step: &'static str) -> Result<Reply, SttError> {
        loop {
            let Some(reply) = self.next_reply()? else {
                return Err(VoiceIoSnafu { step }.into_error(ErrorKind::TimedOut.into()).into());
            };
            if let Some(event) = event(&reply)? {
                self.events.push_back(event);
            } else if (100..300).contains(&reply.code) {
                return Ok(reply);
            } else if !(700..800).contains(&reply.code) {
                return Err(VoiceRefusedSnafu { step, code: reply.code }.build().into());
            }
        }
    }

    /// The next whole reply, or `None` when the read timeout passed first.
    fn next_reply(&mut self) -> Result<Option<Reply>, SttError> {
        loop {
            if let Some(reply) = self.parse_buffered()? {
                return Ok(Some(reply));
            }
            let mut chunk = [0_u8; 4096];
            match self.stream.read(&mut chunk) {
                Ok(0) => return Err(VoiceIoSnafu { step: "read" }.into_error(ErrorKind::UnexpectedEof.into()).into()),
                Ok(n) => self.buf.extend_from_slice(chunk.get(..n).unwrap_or_default()),
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => return Ok(None),
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(VoiceIoSnafu { step: "read" }.into_error(e).into()),
            }
            if self.buf.len() > MAX_REPLY_BYTES {
                return Err(VoiceFailedSnafu { step: "SSIP reply size" }.build().into());
            }
        }
    }

    /// Takes the first whole reply out of the buffer, if it holds one.
    fn parse_buffered(&mut self) -> Result<Option<Reply>, SttError> {
        let mut data = Vec::new();
        let mut start = 0;
        while let Some(len) = self.buf.get(start..).and_then(|rest| rest.iter().position(|&b| b == b'\n')) {
            let raw = self.buf.get(start..start + len).unwrap_or_default();
            let line = raw.strip_suffix(b"\r").unwrap_or(raw);
            start += len + 1;
            let (code, last, text) = parse_line(line)?;
            if last {
                self.buf.drain(..start);
                return Ok(Some(Reply { code, data }));
            }
            data.push(text);
        }
        Ok(None)
    }
}

/// Sends `CANCEL self` (Controlling Speech Output): our queued and sounding messages stop.
///
/// Written on a clone of a connection whose reply and CANCELED event its reader takes.
pub(crate) fn cancel(mut stream: impl Write) -> io::Result<()> {
    stream.write_all(b"CANCEL self\r\n")?;
    stream.flush()
}

/// One reply line: its code, whether it is the last one, and its text.
fn parse_line(line: &[u8]) -> Result<(u16, bool, String), SttError> {
    let malformed = || VoiceFailedSnafu { step: "SSIP reply" }.build().into();
    let digits = line.get(..3).filter(|d| d.iter().all(u8::is_ascii_digit)).ok_or_else(malformed)?;
    let code = digits.iter().fold(0_u16, |n, d| n * 10 + u16::from(d - b'0'));
    let text = String::from_utf8_lossy(line.get(4..).unwrap_or_default()).into_owned();
    match line.get(3) {
        Some(b'-') => Ok((code, false, text)),
        Some(b' ') | None => Ok((code, true, text)),
        Some(_) => Err(malformed()),
    }
}

/// The event a `701`, `702` or `703` notification reports; `None` for any other reply.
fn event(reply: &Reply) -> Result<Option<Event>, SttError> {
    let kind = match reply.code {
        701 => EventKind::Begin,
        702 => EventKind::End,
        703 => EventKind::Cancel,
        _ => return Ok(None),
    };
    let msg = reply.data.first().and_then(|id| id.trim().parse().ok());
    let msg = msg.ok_or_else(|| SttError::from(VoiceFailedSnafu { step: "SSIP event" }.build()))?;
    Ok(Some(Event { msg, kind }))
}

/// One part of a client name: alphanumerics, `-` and `_`, anything else as `_`.
fn name_part(raw: &str) -> String {
    if raw.is_empty() {
        return "unknown".to_owned();
    }
    raw.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

/// The `SPEAK` data: CRLF lines, dot-stuffed, closed by `.` alone, a NUL spoken as a space.
fn body(text: &str) -> String {
    let text = text.replace('\0', " ").replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(text.len() + 8);
    for line in text.split('\n') {
        if line.starts_with('.') {
            out.push('.');
        }
        out.push_str(line);
        out.push_str("\r\n");
    }
    out.push_str(".\r\n");
    out
}

#[cfg(test)]
#[path = "ssip_tests.rs"]
mod ssip_tests;
