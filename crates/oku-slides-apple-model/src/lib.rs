//! Apple's on-device language model (Foundation Models, macOS 26+) through a Swift bridge.
//!
//! - [`availability`]: whether the model can answer on this Mac, and why not.
//! - [`respond`]: one guided reply, blocking, under a JSON Schema.
//!
//! `FoundationModels` is weak-linked: binaries still start on macOS 12, where the model
//! reports [`AppleModel::MacOsTooOld`]. Off macOS nothing is built from Swift.

mod bridge;

use snafu::{Backtrace, Snafu};
use tokio_util::sync::CancellationToken;

/// The state of Apple's on-device model on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppleModel {
    /// The model is ready to answer.
    Ready,
    /// This Mac cannot run Apple Intelligence.
    DeviceNotEligible,
    /// Apple Intelligence is turned off in System Settings.
    IntelligenceOff,
    /// The model is still downloading or preparing.
    ModelNotReady,
    /// macOS is older than 26, or this is not macOS.
    MacOsTooOld,
    /// The system gave a reason this build does not know.
    Unknown,
}

/// The state of Apple's on-device model, asked of the system (cheap, synchronous).
#[must_use]
pub fn availability() -> AppleModel {
    bridge::availability()
}

/// One guided reply: instructions, a prompt and the JSON Schema the reply must follow.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// The session's instructions (system prompt).
    pub instructions: &'a str,
    /// The whole conversation, as text.
    pub prompt: &'a str,
    /// The JSON Schema of the reply, in the form `GenerationSchema` decodes.
    pub schema: &'a str,
    /// Sampling temperature.
    pub temperature: f64,
    /// Longest reply, in tokens; 0 leaves the model's own limit.
    pub max_tokens: u32,
}

/// Runs one reply and blocks until it ends; `cancel` stops it early.
///
/// # Errors
///
/// When the prompt overflows the context, the model refuses, the reply fails or is cancelled.
pub fn respond(request: &Request<'_>, cancel: &CancellationToken) -> Result<String, AppleModelError> {
    bridge::respond(request, cancel)
}

/// What kind of failure an [`AppleModelError`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Instructions, prompt and schema do not fit the model's context.
    ContextOverflow,
    /// A guardrail or the model refused the request.
    Refused,
    /// Any other failure (invalid schema, model unavailable, system error).
    Failed,
    /// The caller cancelled the reply.
    Cancelled,
}

/// Apple's model could not reply; the message is English, for logs only.
#[derive(Debug, Snafu)]
pub struct AppleModelError(Inner);

impl AppleModelError {
    /// The kind of failure.
    #[must_use]
    pub fn kind(&self) -> ErrorKind {
        match self.0 {
            Inner::ContextOverflow { .. } => ErrorKind::ContextOverflow,
            Inner::Refused { .. } => ErrorKind::Refused,
            Inner::Failed { .. } => ErrorKind::Failed,
            Inner::Cancelled { .. } => ErrorKind::Cancelled,
        }
    }

    /// The error of `kind` with the system's `message`.
    fn of(kind: ErrorKind, message: String) -> Self {
        Self(match kind {
            ErrorKind::ContextOverflow => ContextOverflowSnafu { message }.build(),
            ErrorKind::Refused => RefusedSnafu { message }.build(),
            ErrorKind::Failed => FailedSnafu { message }.build(),
            ErrorKind::Cancelled => CancelledSnafu.build(),
        })
    }
}

#[derive(Debug, Snafu)]
enum Inner {
    #[snafu(display("the request overflows the model's context: {message}"))]
    ContextOverflow { message: String, backtrace: Backtrace },
    #[snafu(display("the model refused the request: {message}"))]
    Refused { message: String, backtrace: Backtrace },
    #[snafu(display("the model failed: {message}"))]
    Failed { message: String, backtrace: Backtrace },
    #[snafu(display("the reply was cancelled"))]
    Cancelled { backtrace: Backtrace },
}
