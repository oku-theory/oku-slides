//! Le reçu d'un tour : outil `report`, et reçu de repli quand l'agent n'en
//! a pas fait (sa dernière narration, première phrase en titre).
//!
//! Les boutons de suite envoient l'action `suggest {prompt}` à l'interface (comme ceux de `render`).

use oku_slides_a2ui::{Action, Component, ComponentKind, Dynamic};
use oku_slides_i18n::Localizer;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use snafu::{OptionExt, ensure};

use super::bounds::{LABEL_MAX, NOTE_CHARS, PROMPT_MAX, TEXT_MAX, WHY_MAX, check_hex, check_text};
use super::check_len;
use super::forms::{Place, Surface, lit, surface};
use super::turn_view::first_line;
use crate::error::{AgentFormSnafu, NoSlideSnafu};
use crate::{Result, Session};

/// `report` : la conclusion d'un tour.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct Report {
    /// Une ligne : ce qui a changé (« Slide 4 : fond bleu nuit »).
    pub title: String,
    /// 1 à 4 faits courts.
    pub facts: Vec<String>,
    /// Slide concernée : sa miniature est montrée.
    #[serde(default)]
    pub slide: Option<String>,
    /// Couleur à montrer (#rrggbb), si pertinent.
    #[serde(default)]
    pub swatch: Option<String>,
    /// L'explication, en Markdown ; repliée.
    #[serde(default)]
    pub why: Option<String>,
    /// 0 à 4 suites ; un clic envoie leur consigne.
    #[serde(default)]
    pub actions: Vec<NextStep>,
}

/// Une suite proposée sous le reçu : un bouton, et la consigne qu'il envoie.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct NextStep {
    /// Libellé du bouton, 2 à 5 mots, dans la langue de l'utilisateur.
    pub label: String,
    /// Consigne envoyée au clic ; le libellé si absente.
    #[serde(default)]
    pub prompt: Option<String>,
}

/// Bornes du nombre de faits d'un reçu.
pub const FACTS: (usize, usize) = (1, 4);
/// Bornes du nombre de suites d'un reçu.
pub const ACTIONS: (usize, usize) = (0, 4);

/// Le reçu : carte, puis « pourquoi » replié, puis les suites.
pub fn receipt(id: &str, spec: &Report, thumb: Option<&str>, l: &Localizer) -> Surface {
    let mut body = vec!["card".to_owned()];
    let mut components = vec![Component::new(
        "card",
        ComponentKind::Receipt {
            title: Dynamic::Literal(spec.title.clone()),
            facts: spec.facts.clone(),
            thumb: thumb.map(str::to_owned),
            swatch: spec.swatch.clone(),
        },
    )];
    if let Some(why) = spec.why.as_deref().filter(|w| !w.trim().is_empty()) {
        components.push(Component::new(
            "why",
            ComponentKind::Disclosure { summary: Dynamic::Literal(l.tr("agent-report-why")), child: "why-text".into() },
        ));
        components.push(Component::new("why-text", ComponentKind::Markdown { text: Dynamic::Literal(why.to_owned()) }));
        body.push("why".into());
    }
    if !spec.actions.is_empty() {
        let ids: Vec<String> = (0..spec.actions.len()).map(|i| format!("next-{i}")).collect();
        for (bid, a) in ids.iter().zip(&spec.actions) {
            let prompt = a.prompt.clone().unwrap_or_else(|| a.label.clone());
            components.push(Component::button(
                bid,
                a.label.clone(),
                false,
                Action::event("suggest", [("prompt", lit(json!(prompt)))]),
            ));
        }
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        components.push(Component::row("next", &refs));
        body.push("next".into());
    }
    let refs: Vec<&str> = body.iter().map(String::as_str).collect();
    components.insert(0, Component::column("root", &refs));
    let mut s = surface(id, Place::Thread, None, components, json!({}));
    s.live = false;
    s
}

/// Fin (octet) de la première phrase : `.`, `!` ou `?` suivi d'un blanc ou de la fin, ou un saut de ligne.
/// « 4.5 », « v0.9 » ne coupent pas ; les délimiteurs étant ASCII, la coupe tombe sur une frontière de caractère.
fn sentence_end(text: &str) -> usize {
    let bytes = text.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        match b {
            b'\n' => return i,
            b'.' | b'!' | b'?' if bytes.get(i + 1).is_none_or(|n| n.is_ascii_whitespace()) => return i + 1,
            _ => {}
        }
    }
    text.len()
}

/// Reçu de repli : la première phrase de `narration` en titre, le reste replié.
pub fn fallback(id: &str, narration: &str, l: &Localizer) -> Option<Surface> {
    let text = narration.trim();
    if text.is_empty() {
        return None;
    }
    let end = sentence_end(text);
    let first = text[..end].trim();
    let title = first_line(first, NOTE_CHARS);
    // Un titre coupé ne perd rien : la phrase entière reste dans l'explication.
    let rest = if title == first { text[end..].trim() } else { text };
    let spec = Report {
        title,
        facts: Vec::new(),
        slide: None,
        swatch: None,
        // Borné comme un `why` d'agent : une narration très longue ne devient pas une carte démesurée.
        why: (!rest.is_empty()).then(|| rest.chars().take(WHY_MAX).collect()),
        actions: Vec::new(),
    };
    Some(receipt(id, &spec, None, l))
}

impl Session {
    /// Surface `report` : bornes vérifiées, miniature de la slide rendue.
    pub fn report_surface(&mut self, id: &str, spec: &Report) -> Result<ReceiptSurfaces> {
        ensure!(!spec.title.trim().is_empty(), AgentFormSnafu { reason: "title: must not be empty".to_string() });
        ensure!(!spec.facts.is_empty(), AgentFormSnafu { reason: "facts: give at least one fact".to_string() });
        check_len("facts", spec.facts.len(), FACTS)?;
        check_len("actions", spec.actions.len(), ACTIONS)?;
        check_text("title", &spec.title, TEXT_MAX)?;
        for (i, f) in spec.facts.iter().enumerate() {
            check_text(&format!("facts[{i}]"), f, TEXT_MAX)?;
        }
        if let Some(why) = &spec.why {
            check_text("why", why, WHY_MAX)?;
        }
        for (i, a) in spec.actions.iter().enumerate() {
            check_text(&format!("actions[{i}].label"), &a.label, LABEL_MAX)?;
            if let Some(p) = &a.prompt {
                check_text(&format!("actions[{i}].prompt"), p, PROMPT_MAX)?;
            }
        }
        if let Some(c) = &spec.swatch {
            check_hex("swatch", c)?;
        }
        let thumb = match &spec.slide {
            Some(slide) => {
                self.deck().slide(slide).context(NoSlideSnafu { id: slide })?;
                Some(self.svg(slide)?)
            }
            None => None,
        };
        let surface = receipt(id, spec, thumb.as_deref(), &self.localizer);
        let plain = (!spec.actions.is_empty())
            .then(|| receipt(id, &Report { actions: Vec::new(), ..spec.clone() }, thumb.as_deref(), &self.localizer));
        Ok(ReceiptSurfaces { surface, plain })
    }
}

/// Le reçu, et sa version sans actions (`None` s'il n'en a pas) : celle qui reste une fois le tour passé.
#[derive(Debug, Clone, PartialEq)]
pub struct ReceiptSurfaces {
    pub surface: Surface,
    pub plain: Option<Surface>,
}

#[cfg(test)]
#[path = "receipt_tests.rs"]
mod receipt_tests;
