//! Lecture du prompt ACP de l'hôte : demande de l'utilisateur et langue de l'interface.

use agent_client_protocol::schema::v1::ContentBlock;

const CONTEXT_TAG: &str = "<oku-slides-context>";
const LOCALE_KEY: &str = "interface language: ";

/// Ce que l'agent retient d'un prompt.
#[derive(Debug, PartialEq)]
pub(crate) struct Prompt {
    /// Texte de l'utilisateur (dernier bloc texte), vide s'il n'y en a pas.
    pub request: String,
    /// Bloc de contexte de l'hôte, tel quel.
    pub context: Option<String>,
    /// Code de la langue de l'interface, par exemple `fr`.
    pub locale: Option<String>,
}

/// Extrait la demande, le contexte et la langue ; les instructions de l'hôte sont ignorées.
pub(crate) fn read_prompt(blocks: &[ContentBlock]) -> Prompt {
    let texts: Vec<&str> = blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    let context = texts.iter().find(|t| t.starts_with(CONTEXT_TAG)).map(|t| (*t).to_owned());
    let request = match texts.last() {
        Some(last) if !last.starts_with(CONTEXT_TAG) => (*last).to_owned(),
        _ => String::new(),
    };
    let locale = context.as_deref().and_then(locale_of);
    Prompt { request, context, locale }
}

/// Le texte entre parenthèses après `interface language: `, jusqu'à `)`.
fn locale_of(context: &str) -> Option<String> {
    let rest = &context[context.find(LOCALE_KEY)? + LOCALE_KEY.len()..];
    let line = rest.lines().next()?;
    let (_, after_open) = line.split_once('(')?;
    let (code, _) = after_open.split_once(')')?;
    Some(code.to_owned())
}

/// Instructions courtes de l'agent local, en anglais.
pub(crate) const INSTRUCTIONS: &str = include_str!("instructions/local.md");

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod prompt_tests;
