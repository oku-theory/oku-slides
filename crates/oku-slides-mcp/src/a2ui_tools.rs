//! `a2ui_examples`: the props and examples of the components `render` accepts.

use oku_slides_a2ui::{ComponentKind, ComponentReference, agent_reference, examples};
use oku_slides_session::DocumentKind;
use rust_mcp_sdk::schema::CallToolResult;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::de::Error as _;
use serde_json::Value;
use snafu::{IntoError, ResultExt};

use crate::tool_error::{
    A2uiElsewhereSnafu, A2uiHostOnlySnafu, A2uiUnknownSnafu, ArgumentsSnafu, PlaybookSnafu, ToolError,
};
use crate::tools::{Tools, kinds_of};

/// At most this many names per call.
pub(crate) const MAX_NAMES: usize = 25;

/// The longest name echoed back in a refusal; catalog names are shorter.
const MAX_ECHO: usize = 32;

/// The components `a2ui_examples` reads.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct A2uiExamplesArgs {
    /// Component names, such as `Card`; absent: every component render accepts.
    #[serde(default)]
    pub components: Option<Vec<String>>,
}

impl Tools {
    /// Answers `a2ui_examples` for this tab's kind.
    pub(crate) fn a2ui_examples(&self, args: Value) -> CallToolResult {
        let args: A2uiExamplesArgs = match self.parse(args) {
            Ok(a) => a,
            Err(reply) => return reply,
        };
        self.reply(examples_for(args.components.as_deref(), self.kind), |r| self.json_result(&r))
    }
}

/// The reference of `wanted` (absent: all), in catalog order, or why a name is refused.
pub(crate) fn examples_for(
    wanted: Option<&[String]>,
    kind: DocumentKind,
) -> Result<Vec<ComponentReference>, ToolError> {
    let reference = agent_reference().context(PlaybookSnafu)?;
    let Some(wanted) = wanted else { return Ok(reference.to_vec()) };
    if wanted.len() > MAX_NAMES {
        let detail = serde_json::Error::custom(format!("components: at most {MAX_NAMES} names"));
        return Err(ArgumentsSnafu.into_error(detail));
    }
    if let Some(missing) = wanted.iter().find(|w| !reference.iter().any(|c| &c.component == *w)) {
        return Err(refusal(missing, reference, kind));
    }
    Ok(reference.iter().filter(|c| wanted.contains(&c.component)).cloned().collect())
}

/// `wanted` safe to echo: ASCII alphanumerics only, marked `…` (or `?` when nothing is left) if altered.
fn echo(wanted: &str) -> String {
    let kept: String = wanted.chars().filter(char::is_ascii_alphanumeric).take(MAX_ECHO).collect();
    match (kept.is_empty(), kept == wanted) {
        (true, _) => "?".to_owned(),
        (false, true) => kept,
        (false, false) => format!("{kept}\u{2026}"),
    }
}

/// Why `render` does not take `wanted`: unknown, shown by another tool, or host only.
fn refusal(wanted: &str, reference: &[ComponentReference], kind: DocumentKind) -> ToolError {
    let name = echo(wanted);
    let root = examples().into_iter().find(|e| e.component == wanted).map(|e| e.components[0].kind.clone());
    let themes_here = kinds_of("show_themes").is_some_and(|k| k.has(kind));
    match root {
        None => {
            let known = reference.iter().map(|c| c.component.as_str()).collect::<Vec<_>>().join(", ");
            A2uiUnknownSnafu { name, known }.build()
        }
        Some(k) if k.is_field() => A2uiElsewhereSnafu { name, tool: "show_params" }.build(),
        Some(ComponentKind::Receipt { .. } | ComponentKind::SlideThumb { .. }) => {
            A2uiElsewhereSnafu { name, tool: "report" }.build()
        }
        Some(ComponentKind::ThemeChoice { .. }) if themes_here => {
            A2uiElsewhereSnafu { name, tool: "show_themes" }.build()
        }
        Some(_) => A2uiHostOnlySnafu { name }.build(),
    }
}

#[cfg(test)]
#[path = "a2ui_tools_tests.rs"]
mod tests;
