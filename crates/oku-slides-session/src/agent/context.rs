//! Ce que l'agent sait à chaque tour : le bloc `<oku-slides-context>`
//! (mode, langue, thème, slide, sélection, actions faites sans lui), et
//! les tours d'annulation (un tour = une entrée ⌘Z).

use std::fmt::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Mode, Session};

/// Longueur du début de consigne dans le libellé d'annulation.
const LABEL_CHARS: usize = 40;

/// Ce que l'interface montre au moment de la consigne.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentUi {
    #[serde(default)]
    pub slide: Option<String>,
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub editing: bool,
}

/// Début de la consigne, coupé à 40 caractères.
pub(crate) fn turn_label(prompt: &str) -> String {
    let prompt = prompt.trim();
    if prompt.chars().count() <= LABEL_CHARS {
        return prompt.to_owned();
    }
    let mut cut: String = prompt.chars().take(LABEL_CHARS).collect();
    cut.push('…');
    cut
}

impl Session {
    /// Ouvre le groupe d'annulation d'un tour de l'agent.
    pub fn begin_turn(&mut self, prompt: &str) {
        let label = self.localizer.tr_with("agent-undo-turn", &[("text", turn_label(prompt).into())]);
        self.doc.sources.begin_group(&label);
    }

    /// Ferme le groupe du tour, y compris sur le deck mis de côté par les modèles.
    pub fn end_turn(&mut self) {
        self.doc.sources.end_group();
        if let Mode::Templates { stash, .. } = &mut self.mode {
            stash.sources.end_group();
        }
    }

    /// Une action faite sans l'agent, qu'il apprendra au prochain tour.
    pub fn note(&mut self, text: impl Into<String>) {
        self.agent_notes.push(text.into());
    }

    /// Dossier du document (le `cwd` de la session ACP) ; `None` s'il n'est pas enregistré.
    pub fn document_dir(&self) -> Option<PathBuf> {
        self.doc.path.as_ref().and_then(|p| p.parent()).map(ToOwned::to_owned)
    }

    /// Bloc de contexte joint à la consigne ; vide les notes.
    pub fn agent_context(&mut self, ui: &AgentUi) -> String {
        let deck = self.doc.sources.deck();
        let mut out = String::from("<oku-slides-context>\n");
        let mode = match &self.mode {
            Mode::Deck => "deck".to_owned(),
            Mode::Templates { edit, .. } => format!("templates(set={})", edit.set),
        };
        let _ = writeln!(out, "mode: {mode}");
        let _ = writeln!(
            out,
            "locale: {} · theme: {} · template_set: {}",
            self.localizer.locale(),
            deck.theme.as_deref().unwrap_or("none"),
            self.deck_set()
        );
        if let Some((i, slide)) =
            ui.slide.as_deref().and_then(|id| deck.slides.iter().enumerate().find(|(_, s)| s.id == id))
        {
            let kind =
                serde_json::to_value(slide.kind).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default();
            let _ = writeln!(
                out,
                "slide: {} ({}/{}) kind={kind} title={:?}",
                slide.id,
                i + 1,
                deck.slides.len(),
                slide.title.as_deref().unwrap_or("")
            );
        }
        if ui.element.is_some() || ui.field.is_some() {
            let _ = writeln!(
                out,
                "selection: element={} field={} editing={}",
                ui.element.as_deref().unwrap_or("-"),
                ui.field.as_deref().unwrap_or("-"),
                ui.editing
            );
        }
        if !self.agent_notes.is_empty() {
            out.push_str("since_last_turn:\n");
            for note in self.agent_notes.drain(..) {
                let _ = writeln!(out, "  - {note}");
            }
        }
        out.push_str("</oku-slides-context>");
        out
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod context_tests;
