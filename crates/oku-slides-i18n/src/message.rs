//! Un message à dire plus tard : identifiant Fluent et paramètres. Il se
//! crée sans langue (dans une erreur, par exemple) et se dit dans celle de
//! l'utilisateur ([`Localize`]) ou en anglais ([`Display`](std::fmt::Display)).

use crate::{FluentValue, Localize, Localizer, english};

/// Un message Fluent et ses paramètres.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    id: &'static str,
    args: Vec<(&'static str, Arg)>,
}

/// Paramètre d'un [`Message`] : texte ou nombre.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    /// Text inserted into the message as is.
    Text(String),
    /// Number, which Fluent formats and agrees according to the language.
    Number(f64),
}

/// Numbers that `f64` represents exactly: lossless conversion.
macro_rules! number_arg_exact {
    ($($t:ty),*) => { $( impl From<$t> for Arg { fn from(n: $t) -> Self { Arg::Number(f64::from(n)) } } )* };
}
number_arg_exact!(u32, i32, f32, f64);

/// 64-bit integers, which have no `From` into `f64`.
///
/// Only numbers beyond 2^53 are rounded.
macro_rules! number_arg_rounded {
    ($($t:ty),*) => { $( impl From<$t> for Arg {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a displayed count never reaches 2^53, and Fluent only formats f64"
        )]
        fn from(n: $t) -> Self { Arg::Number(n as f64) }
    } )* };
}
number_arg_rounded!(usize, u64, i64);

impl From<String> for Arg {
    fn from(s: String) -> Self {
        Arg::Text(s)
    }
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Arg::Text(s.to_owned())
    }
}

impl Message {
    /// Creates a message without parameters, from its Fluent identifier.
    #[must_use]
    pub fn new(id: &'static str) -> Self {
        Self { id, args: Vec::new() }
    }

    /// Ajoute un paramètre.
    #[must_use]
    pub fn arg(mut self, name: &'static str, value: impl Into<Arg>) -> Self {
        self.args.push((name, value.into()));
        self
    }

    /// Fluent identifier of the message, useful to test without depending on the text.
    #[must_use]
    pub fn id(&self) -> &'static str {
        self.id
    }
}

impl Localize for Message {
    fn localize(&self, l: &Localizer) -> String {
        let args: Vec<(&str, FluentValue<'_>)> = self
            .args
            .iter()
            .map(|(name, value)| {
                let v = match value {
                    Arg::Text(s) => FluentValue::from(s.as_str()),
                    Arg::Number(n) => FluentValue::from(*n),
                };
                (*name, v)
            })
            .collect();
        l.tr_with(self.id, &args)
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.localize(english()))
    }
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod message_tests;
