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
    Text(String),
    Number(f64),
}

macro_rules! number_arg {
    ($($t:ty),*) => { $( impl From<$t> for Arg { fn from(n: $t) -> Self { Arg::Number(n as f64) } } )* };
}
number_arg!(usize, u64, u32, i64, i32, f32, f64);

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
    pub fn new(id: &'static str) -> Self {
        Self { id, args: Vec::new() }
    }

    /// Ajoute un paramètre.
    pub fn arg(mut self, name: &'static str, value: impl Into<Arg>) -> Self {
        self.args.push((name, value.into()));
        self
    }

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
mod message_tests {
    use super::*;

    #[test]
    fn message_with_args_localizes_and_displays_in_english() {
        let m = Message::new("chart-series-empty").arg("index", 2usize);
        assert_eq!(m.localize(&Localizer::new(Some("fr"))), "série 2 sans données");
        assert_eq!(m.to_string(), "series 2 has no data");
    }
}
