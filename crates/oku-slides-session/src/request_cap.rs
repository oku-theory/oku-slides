//! The size cap on a request that may apply ops, the same over HTTP and in the app.

use std::io::{self, Write};

use serde_json::Value;
use snafu::ensure;

use crate::Result;
use crate::error::RequestTooLargeSnafu;

/// One mebibyte.
pub(crate) const MIB: usize = 1024 * 1024;

/// The longest request, in bytes of JSON, that may apply ops: 8 MiB.
pub const MAX_APPLY_BYTES: usize = 8 * MIB;

/// Refuses a request whose JSON is longer than [`MAX_APPLY_BYTES`].
///
/// # Errors
///
/// `RequestTooLarge` past the cap.
pub fn check_request_size(request: &Value) -> Result<()> {
    let mut counted = Counted(0);
    // Counting never fails, nor does writing a `Value`; a failure would read as too long.
    let written = serde_json::to_writer(&mut counted, request);
    ensure!(written.is_ok() && counted.0 <= MAX_APPLY_BYTES, RequestTooLargeSnafu { max: MAX_APPLY_BYTES });
    Ok(())
}

/// A writer that counts the bytes it is given.
struct Counted(usize);

impl Write for Counted {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "request_cap_tests.rs"]
mod request_cap_tests;
