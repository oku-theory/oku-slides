//! Familles d'outils : le modèle en choisit une ou deux, puis ne voit que leurs outils.

/// Famille d'outils MCP, choisie par le modèle avant de voir les schémas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Family {
    /// Contenu des diapositives, éléments, graphiques et tableaux.
    Edit,
    /// Lancer, piloter et arrêter une présentation.
    Present,
    /// Ouvrir, créer, enregistrer, fusionner, historique.
    Files,
    /// Modèles, widgets et thèmes.
    Templates,
    /// Réglages de l'app.
    Configure,
    /// Expliquer ou diagnostiquer un échec.
    Troubleshoot,
    /// Questions et rapports : toujours exposée, jamais routable.
    AskReport,
}

impl Family {
    /// Les familles que le modèle peut choisir.
    pub(crate) const ROUTABLE: [Family; 6] =
        [Family::Edit, Family::Present, Family::Files, Family::Templates, Family::Configure, Family::Troubleshoot];

    /// Clé écrite par le modèle dans sa réponse de routage.
    pub(crate) fn key(self) -> &'static str {
        match self {
            Family::Edit => "edit",
            Family::Present => "present",
            Family::Files => "files",
            Family::Templates => "templates",
            Family::Configure => "configure",
            Family::Troubleshoot => "troubleshoot",
            Family::AskReport => "ask_report",
        }
    }

    /// Une ligne d'anglais pour l'invite de routage.
    pub(crate) fn summary(self) -> &'static str {
        match self {
            Family::Edit => "change slide content, elements, charts and tables, or move between slides",
            Family::Present => "start, steer or stop a presentation",
            Family::Files => "open, create, save, copy, move, merge a deck or browse its history",
            Family::Templates => "edit templates, widgets and themes",
            Family::Configure => "change app settings: language, dictation, interface",
            Family::Troubleshoot => "diagnose an error, a failed render or import, or why something fails",
            Family::AskReport => "ask the user a question or report back",
        }
    }
}

/// Familles d'un outil MCP ; vide si l'outil est inconnu.
pub(crate) fn families_of(tool: &str) -> &'static [Family] {
    use Family::{AskReport, Configure, Edit, Files, Present, Templates, Troubleshoot};
    match tool {
        "apply" | "slide" | "widgets" | "chart_kind" | "show_as" | "show_params" | "show_variants" => &[Edit],
        "focus" => &[Edit, Present],
        "deck_outline" => &[Edit, Present, Files],
        "render_slide" => &[Edit, Troubleshoot],
        "present" => &[Present],
        "open" | "new" | "save" | "save_copy" | "move" | "merge" | "history" => &[Files],
        "templates"
        | "edit_templates"
        | "save_templates"
        | "close_templates"
        | "delete_template_set"
        | "edit_theme"
        | "theme_tokens"
        | "widget_source"
        | "widget_sources"
        | "set_widget"
        | "remove_widget"
        | "set_widget_trial"
        | "preview_widget" => &[Templates],
        "settings" => &[Configure, Troubleshoot],
        "set_setting" | "set_interface" => &[Configure],
        "import_report" | "import_diff" | "diagnose" => &[Troubleshoot],
        "ask_choice" | "ask_confirm" | "ask_rewrite" | "report" | "render" => &[AskReport],
        _ => &[],
    }
}

/// Grammaire GBNF de l'étape de routage : une ou deux clés de famille, séparées par une virgule.
#[cfg_attr(
    all(not(test), not(all(target_os = "macos", target_arch = "aarch64"))),
    expect(dead_code, reason = "lu par le moteur Gguf, sur Apple silicon seulement")
)]
pub(crate) fn route_grammar() -> String {
    let keys: Vec<String> = Family::ROUTABLE.iter().map(|f| format!("\"{}\"", f.key())).collect();
    format!("root ::= family (\", \" family)?\nfamily ::= {}\n", keys.join(" | "))
}

/// Les deux premières familles routables nommées dans la réponse, sans doublon.
pub(crate) fn parse_route(reply: &str) -> Vec<Family> {
    let lower = reply.to_lowercase();
    let mut found = Vec::new();
    for token in lower.split(|c: char| !(c.is_ascii_lowercase() || c == '_')) {
        if let Some(f) = Family::ROUTABLE.into_iter().find(|f| f.key() == token)
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

#[cfg(test)]
#[path = "families_tests.rs"]
mod families_tests;
