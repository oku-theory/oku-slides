//! Tool families: the model picks one or two, then sees only their tools.

use std::collections::HashSet;
use std::sync::OnceLock;

use oku_slides_session::DocumentKind;

/// An MCP tool family, picked by the model before it sees any schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Family {
    /// Slide content, elements, charts and tables.
    Edit,
    /// Start, steer and stop a presentation.
    Present,
    /// Open, create, save, merge, history.
    Files,
    /// Packs (widgets and layouts) and themes.
    Packs,
    /// App settings.
    Configure,
    /// Explain or diagnose a failure.
    Troubleshoot,
    /// Questions and reports: always exposed, never routable.
    AskReport,
}

impl Family {
    /// The families the model can pick.
    pub(crate) const ROUTABLE: [Family; 6] =
        [Family::Edit, Family::Present, Family::Files, Family::Packs, Family::Configure, Family::Troubleshoot];

    /// The key the model writes in its routing reply.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Family::Edit => "edit",
            Family::Present => "present",
            Family::Files => "files",
            Family::Packs => "packs",
            Family::Configure => "configure",
            Family::Troubleshoot => "troubleshoot",
            Family::AskReport => "ask_report",
        }
    }

    /// One line of English for the routing prompt.
    pub(crate) fn summary(self) -> &'static str {
        match self {
            Family::Edit => "change slide content, elements, charts and tables, or move between slides",
            Family::Present => "start, steer or stop a presentation",
            Family::Files => "open, create, save, copy, move, merge a deck or browse its history",
            Family::Packs => "edit packs of widgets and slide layouts, and themes",
            Family::Configure => "change app settings: language, dictation, the app agent, interface",
            Family::Troubleshoot => "diagnose an error, a failed render or import, or why something fails",
            Family::AskReport => "ask the user a question or report back",
        }
    }
}

/// The families of an MCP tool; empty when the tool is unknown.
pub(crate) fn families_of(tool: &str) -> &'static [Family] {
    use Family::{AskReport, Configure, Edit, Files, Packs, Present, Troubleshoot};
    match tool {
        "apply" | "slide" | "widgets" | "chart_kind" | "show_as" | "show_params" | "show_variants" | "show_themes"
        | "show_packs" | "add_pack" | "remove_pack" => &[Edit],
        "focus" => &[Edit, Present],
        "deck_outline" => &[Edit, Present, Files],
        "render_slide" => &[Edit, Troubleshoot],
        "present" => &[Present],
        "open" | "new" | "save" | "save_copy" | "move" | "merge" | "history" => &[Files],
        "layouts" => &[Edit, Packs],
        "packs" | "open_pack" | "open_theme" | "save_pack" | "delete_pack" | "edit_theme" | "theme_tokens"
        | "widget_source" | "widget_sources" | "set_widget" | "remove_widget" | "set_widget_trial"
        | "preview_widget" | "set_theme_extends" | "select_item" | "set_preview_theme" | "rename_theme"
        | "save_theme" => &[Packs],
        "settings" => &[Configure, Troubleshoot],
        "set_setting"
        | "set_interface"
        | "show_dictation"
        | "set_dictation"
        | "download_dictation_model"
        | "cancel_dictation_download"
        | "select_agent"
        | "download_agent_model"
        | "cancel_agent_model_download" => &[Configure],
        "import_report" | "import_diff" | "diagnose" => &[Troubleshoot],
        "ask_choice" | "ask_confirm" | "ask_rewrite" | "report" | "render" => &[AskReport],
        _ => &[],
    }
}

/// The GBNF grammar of the routing step: one or two keys of `offered`, comma-separated.
#[cfg_attr(
    all(not(test), not(all(target_os = "macos", target_arch = "aarch64"))),
    expect(dead_code, reason = "read by the Gguf engine, on Apple silicon only")
)]
pub(crate) fn route_grammar(offered: &[Family]) -> String {
    let keys: Vec<String> = offered.iter().map(|f| format!("\"{}\"", f.key())).collect();
    format!("root ::= family (\", \" family)?\nfamily ::= {}\n", keys.join(" | "))
}

/// The first two families of `offered` that the reply names, without duplicates.
pub(crate) fn parse_route(reply: &str, offered: &[Family]) -> Vec<Family> {
    let lower = reply.to_lowercase();
    let mut found = Vec::new();
    for token in lower.split(|c: char| !(c.is_ascii_lowercase() || c == '_')) {
        if let Some(f) = offered.iter().copied().find(|f| f.key() == token)
            && !found.contains(&f)
        {
            found.push(f);
            if found.len() == 2 {
                break;
            }
        }
    }
    found
}

/// The tools every kind of tab lists, computed once from `oku_slides_mcp::tool_names_for`.
fn shared_tools() -> &'static HashSet<String> {
    static SHARED: OnceLock<HashSet<String>> = OnceLock::new();
    SHARED.get_or_init(|| {
        let mut kinds = [DocumentKind::Deck, DocumentKind::Help, DocumentKind::Pack, DocumentKind::Theme]
            .map(|k| oku_slides_mcp::tool_names_for(k).into_iter().collect::<HashSet<String>>())
            .into_iter();
        let first = kinds.next().unwrap_or_default();
        kinds.fold(first, |acc, k| acc.intersection(&k).cloned().collect())
    })
}

/// The routable families a tab offers, in `ROUTABLE` order, given the tools it lists.
///
/// A listed tool counts for its families unless every kind lists it and it spans several
/// families (`focus`, `deck_outline`, `settings`): such a tool says nothing about the tab.
/// A shared tool of one family (`apply`, `set_setting`) still counts, so every tab keeps
/// `edit`; `configure` is always offered, all its tools being shared.
pub(crate) fn offered_families<'a>(listed: impl IntoIterator<Item = &'a str>) -> Vec<Family> {
    let shared = shared_tools();
    let own: Vec<&str> = listed.into_iter().filter(|t| !shared.contains(*t) || families_of(t).len() == 1).collect();
    Family::ROUTABLE
        .into_iter()
        .filter(|f| *f == Family::Configure || own.iter().any(|t| families_of(t).contains(f)))
        .collect()
}

#[cfg(test)]
#[path = "families_tests.rs"]
mod families_tests;
