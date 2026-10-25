//! Ce que l'agent sait à chaque tour : le bloc `<oku-slides-context>`
//! (mode, langues de l'interface et du deck, thème, slide, sélection, actions
//! faites sans lui), et les tours d'annulation (un tour = une entrée ⌘Z).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use oku_slides_core::{Namespace, PackPin};

use super::bounds::clip;
use crate::widgets_edit::parse_name;
use crate::{Mode, Session};

/// Longueur du début de consigne dans le libellé d'annulation.
const LABEL_CHARS: usize = 40;

/// Ce que l'interface montre au moment de la consigne.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentUi {
    /// Id of the displayed slide.
    #[serde(default)]
    pub slide: Option<String>,
    /// Id of the selected element on this slide.
    #[serde(default)]
    pub element: Option<String>,
    /// Name of the selected field, as given by [`crate::FieldView`].
    #[serde(default)]
    pub field: Option<String>,
    /// The selected field is being edited in place.
    #[serde(default)]
    pub editing: bool,
    /// Sous-onglet de widget actif (mode pack).
    #[serde(default)]
    pub widget: Option<String>,
    /// Langue que l'interface affiche : elle change à chaud, la session garde celle du lancement.
    #[serde(default)]
    pub locale: Option<String>,
}

/// Début de la consigne, coupé à 40 caractères (« … » compris), sans caractères invisibles.
pub(crate) fn turn_label(prompt: &str) -> String {
    clip(prompt.trim(), LABEL_CHARS)
}

/// Longueur d'une étiquette de langue BCP 47, au plus.
const LANG_TAG_MAX: usize = 35;

/// La langue déclarée du deck pour le contexte : entre guillemets, `unset`, ou
/// `invalid` si ce n'est pas une étiquette de langue (un deck reçu peut porter
/// n'importe quel texte).
pub(crate) fn declared_language(lang: Option<&str>) -> String {
    match lang {
        None => "unset".to_owned(),
        Some(l)
            if (1..=LANG_TAG_MAX).contains(&l.len()) && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') =>
        {
            format!("{l:?}")
        }
        Some(_) => "invalid".to_owned(),
    }
}

/// Les packs épinglés pour le contexte : `<namespace>=<id>`, séparés par des virgules ; `none` sans pack.
pub(crate) fn widget_packs(pins: &BTreeMap<Namespace, PackPin>) -> String {
    if pins.is_empty() {
        return "none".to_owned();
    }
    let packs: Vec<String> = pins.iter().map(|(ns, pin)| format!("{ns}={}", pin.id)).collect();
    packs.join(", ")
}

/// Essais cités au contexte, au plus.
const TRIALS_SHOWN: usize = 10;
/// Longueur d'une valeur d'essai citée au contexte.
const TRIAL_CHARS: usize = 80;

/// « (extends A, B) » après le nom d'un thème qui en étend d'autres ; rien sinon.
fn extends_note(names: &[String]) -> String {
    match names {
        [] => String::new(),
        names => format!(" (extends {})", names.join(", ")),
    }
}

impl Session {
    /// Le nom du thème `ns` que `deck` épingle ; son espace de noms s'il ne se lit pas.
    fn pinned_theme_name(&self, deck: &oku_slides_core::Deck, ns: &Namespace) -> String {
        let manifest = deck.themes.get(ns).and_then(|pin| self.theme_manifest(pin));
        manifest.map_or_else(|| ns.as_str().to_owned(), |m| m.name)
    }

    /// Ouvre le groupe d'annulation du tour `turn` de l'agent (son numéro dans le fil).
    ///
    /// La séance de widgets du mode deck finit ici et à la fin du tour : une
    /// séance ne chevauche jamais l'étape d'un tour.
    pub fn begin_turn(&mut self, turn: u32, prompt: &str) {
        self.end_widget_session();
        let label = self.localizer.tr_with("agent-undo-turn", &[("text", turn_label(prompt).into())]);
        self.doc.sources.begin_group(&label);
        self.turn_steps_begin(turn);
    }

    /// Ferme le groupe du tour, y compris sur le deck mis de côté (mode pack, historique).
    pub fn end_turn(&mut self) {
        self.end_widget_session();
        self.doc.sources.end_group();
        if let Mode::Pack { stash, .. } | Mode::History { stash, .. } = &mut self.mode {
            stash.sources.end_group();
        }
        self.turn_steps_end();
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
            Mode::Deck | Mode::Help { .. } => "deck".to_owned(),
            Mode::Pack { edit, .. } => format!("pack(id={})", edit.pack),
            Mode::History { history, .. } => {
                let view = history.view();
                format!("history(read_only, version={}/{})", view.index + 1, view.count)
            }
        };
        let _ = writeln!(out, "mode: {mode}");
        // En mode pack, le deck de travail n'a pas de thème : le brouillon
        // part du thème édité, que l'agent change par `set_theme`.
        let theme = match &self.mode {
            Mode::Pack { theme, .. } => {
                let names: Vec<String> = self.draft_parents(theme).into_iter().map(|p| p.name).collect();
                format!("theme (draft): {}{}", theme.name, extends_note(&names))
            }
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } => {
                let manifest = deck.theme.as_ref().and_then(|ns| self.theme_manifest(deck.themes.get(ns)?));
                match manifest {
                    Some(m) => {
                        let names: Vec<String> = m.extends.iter().map(|ns| self.pinned_theme_name(deck, ns)).collect();
                        format!("theme: {}{}", m.name, extends_note(&names))
                    }
                    None => "theme: none".to_owned(),
                }
            }
        };
        // La langue affichée par l'interface ; sans elle (ou inconnue), celle de la session.
        let locale = ui.locale.as_deref().and_then(oku_slides_i18n::shipped).unwrap_or(self.localizer.locale());
        let _ = writeln!(
            out,
            "interface language: {} ({locale}) · deck language: {} · {theme} · widget_packs: {}",
            // `locale` est livrée : son nom existe (`locales!`), le code n'est qu'un secours.
            oku_slides_i18n::english_name(locale).unwrap_or(locale),
            declared_language(deck.params.lang.as_deref()),
            widget_packs(self.deck_pins())
        );
        if let Some((i, slide)) =
            ui.slide.as_deref().and_then(|id| deck.slides.iter().enumerate().find(|(_, s)| s.id == id))
        {
            let _ = writeln!(
                out,
                "slide: {} ({}/{}) kind={} title={:?}",
                slide.id,
                i + 1,
                deck.slides.len(),
                slide.kind,
                slide.params.text("title").unwrap_or("")
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
        // Un nom invalide n'est pas cité : il vient tel quel de l'interface.
        if let Some(w) = ui.widget.as_deref().and_then(|w| parse_name(w).ok()) {
            // Ses valeurs d'exemple, celles de `set_widget_trial`.
            match self.example_values(None, &w).ok().filter(|m| !m.is_empty()) {
                Some(m) => {
                    let mut shown: Vec<String> = m
                        .iter()
                        .take(TRIALS_SHOWN)
                        .map(|(k, v)| {
                            format!(
                                "{} = {}",
                                Value::from(k.as_str()),
                                clip(&v.to_string().replace('<', "\\u003c"), TRIAL_CHARS)
                            )
                        })
                        .collect();
                    if m.len() > TRIALS_SHOWN {
                        shown.push("…".into());
                    }
                    let _ = writeln!(out, "widget: {} · trial: {}", w.as_str(), shown.join(", "));
                }
                None => {
                    let _ = writeln!(out, "widget: {}", w.as_str());
                }
            }
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
