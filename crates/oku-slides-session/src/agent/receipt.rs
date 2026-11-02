//! Le reçu d'un tour : outil `report`, et reçu de repli quand l'agent n'en
//! a pas fait (sa dernière narration, première phrase en titre).
//!
//! Les boutons de suite envoient l'action `suggest {prompt}` à l'interface (comme ceux de `render`).
//! Under the card, the turn's traces: a button per element the agent changed (action
//! `trace {slide, element, pane}`, handled by the interface only), a caption for one gone.
//!
//! La slide d'un reçu est une référence (identifiant, empreinte de sa page au moment du reçu) que
//! l'interface résout en miniature : le reçu ne porte jamais le rendu de la slide (t-450).

use oku_slides_a2ui::{Action, Component, ComponentKind, Dynamic, SlideRef, TextVariant};
use oku_slides_i18n::Localizer;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use snafu::{OptionExt, ensure};

use super::bounds::{LABEL_MAX, NOTE_CHARS, PROMPT_MAX, TEXT_MAX, WHY_MAX, check_hex, check_text};
use super::forms::{Place, Surface, lit, surface};
use super::turn_view::first_line;
use super::{AgentFormReason, TRACE_EVENT, TRACES_SHOWN, Trace, TraceName, check_len};
use crate::error::{AgentFormSnafu, NoSlideSnafu};
use crate::param_fields::kind_name;
use crate::views::PageHash;
use crate::{Result, Session};

/// `report`: the conclusion of a turn.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct Report {
    /// One line: what changed ("Slide 4: midnight blue background").
    pub title: String,
    /// 1 to 4 short facts.
    pub facts: Vec<String>,
    /// Slide concerned: its thumbnail is shown.
    #[serde(default)]
    pub slide: Option<String>,
    /// Color to show (#rrggbb), if relevant.
    #[serde(default)]
    pub swatch: Option<String>,
    /// The explanation, in Markdown; collapsed.
    #[serde(default)]
    pub why: Option<String>,
    /// 0 to 4 next steps; a click sends their prompt.
    #[serde(default)]
    pub actions: Vec<NextStep>,
}

/// A next step offered under the receipt: a button, and the prompt it sends.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct NextStep {
    /// Button label, 2 to 5 words, in the user's language.
    pub label: String,
    /// Prompt sent on click; the label if absent.
    #[serde(default)]
    pub prompt: Option<String>,
}

/// Bornes du nombre de faits d'un reçu.
pub const FACTS: (usize, usize) = (1, 4);
/// Bornes du nombre de suites d'un reçu.
pub const ACTIONS: (usize, usize) = (0, 4);

/// The name a trace's chip shows: its text, or its kind's localized name.
fn trace_name(name: &TraceName, l: &Localizer) -> String {
    match name {
        TraceName::Text(text) => text.clone(),
        TraceName::Kind { kind, label } => kind_name(kind, label.as_ref(), l),
    }
}

/// "object: summary": gone, data changed, or the number of changes.
fn trace_label(t: &Trace, l: &Localizer) -> String {
    let name = trace_name(&t.name, l);
    if t.removed {
        l.tr_with("agent-trace-removed", &[("name", name.into())])
    } else if t.data {
        l.tr_with("agent-trace-data", &[("name", name.into())])
    } else {
        l.tr_with("agent-trace", &[("name", name.into()), ("count", i64::from(t.changes).into())])
    }
}

/// The row of traces (`traces`), its components pushed to `components`; `None` without traces.
fn traces_row(traces: &[Trace], components: &mut Vec<Component>, l: &Localizer) -> Option<String> {
    if traces.is_empty() {
        return None;
    }
    let mut ids = Vec::new();
    for (i, t) in traces.iter().take(TRACES_SHOWN).enumerate() {
        let id = format!("trace-{i}");
        let label = trace_label(t, l);
        components.push(if t.removed {
            Component::text(&id, label, Some(TextVariant::Caption))
        } else {
            let context =
                [("slide", lit(json!(t.slide))), ("element", lit(json!(t.element))), ("pane", lit(json!(t.data)))];
            Component::button(&id, label, false, Action::event(TRACE_EVENT, context))
        });
        ids.push(id);
    }
    let more = traces.len().saturating_sub(TRACES_SHOWN);
    if more > 0 {
        let text = l.tr_with("agent-trace-more", &[("count", more.into())]);
        components.push(Component::text("traces-more", text, Some(TextVariant::Caption)));
        ids.push("traces-more".into());
    }
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    components.push(Component::row("traces", &refs));
    Some("traces".into())
}

/// The receipt: the card, then its traces, the folded "why", then the follow-ups.
pub fn receipt(id: &str, spec: &Report, slide: Option<&SlideRef>, traces: &[Trace], l: &Localizer) -> Surface {
    let mut body = vec!["card".to_owned()];
    let mut components = vec![Component::new(
        "card",
        ComponentKind::Receipt {
            title: Dynamic::Literal(spec.title.clone()),
            facts: spec.facts.clone(),
            slide: slide.cloned(),
            swatch: spec.swatch.clone(),
        },
    )];
    body.extend(traces_row(traces, &mut components, l));
    if let Some(why) = spec.why.as_deref().filter(|w| !w.trim().is_empty()) {
        components.push(Component::new(
            "why",
            ComponentKind::Disclosure {
                summary: Dynamic::Literal(l.tr("agent-report-why")),
                child: "why-text".into(),
                open: None,
            },
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
            b'.' | b'!' | b'?' if bytes.get(i + 1).is_none_or(u8::is_ascii_whitespace) => return i + 1,
            _ => {}
        }
    }
    text.len()
}

/// Fallback receipt: the first sentence of `narration` as title, the rest folded.
///
/// `written` (the turn's guide writes, `Session::guide_write_fact`) become its facts. Without
/// narration, the first of them is the title; with neither, `agent-trace-title` titles the
/// traces, and without traces either there is no receipt. `traces` follow the card, as in
/// [`receipt`].
pub fn fallback(id: &str, narration: &str, written: &[String], traces: &[Trace], l: &Localizer) -> Option<Surface> {
    let text = narration.trim();
    if text.is_empty() {
        let (title, facts) = match written.split_first() {
            Some((title, facts)) => (title.clone(), facts.to_vec()),
            None if !traces.is_empty() => (l.tr("agent-trace-title"), Vec::new()),
            None => return None,
        };
        let spec = Report { title, facts, slide: None, swatch: None, why: None, actions: Vec::new() };
        return Some(receipt(id, &spec, None, traces, l));
    }
    let end = sentence_end(text);
    let first = text[..end].trim();
    let title = first_line(first, NOTE_CHARS);
    // A cut title loses nothing: the whole sentence stays in the explanation.
    let rest = if title == first { text[end..].trim() } else { text };
    let spec = Report {
        title,
        slide: None,
        swatch: None,
        // Bounded like an agent's `why`: a very long narration does not become an oversized card.
        why: (!rest.is_empty()).then(|| rest.chars().take(WHY_MAX).collect()),
        actions: Vec::new(),
        facts: written.to_vec(),
    };
    Some(receipt(id, &spec, None, traces, l))
}

impl Session {
    /// `report` surface: bounds checked, blank facts dropped, slide named by its page hash.
    ///
    /// The turn's guide writes (`written`) follow the agent's facts, outside its bounds; the open
    /// turn's traces follow the card, in the plain copy too.
    ///
    /// # Errors
    ///
    /// Fails (`AgentForm`) on an empty title, facts all blank or out of bounds, or text rejected by
    /// the bounds.
    pub fn report_surface(&mut self, id: &str, spec: &Report, written: &[String]) -> Result<ReceiptSurfaces> {
        ensure!(
            !spec.title.trim().is_empty(),
            AgentFormSnafu { reason: AgentFormReason::Empty { at: "title".into() } }
        );
        // Un fait blanc est écarté : la carte ne montre pas de ligne vide (t-458) ; la borne
        // compte donc les faits montrés (t-465).
        let facts: Vec<String> = spec.facts.iter().filter(|f| !f.trim().is_empty()).cloned().collect();
        // Des faits tous blancs ne disent rien : le reçu montrerait une ligne vide.
        ensure!(!facts.is_empty(), AgentFormSnafu { reason: AgentFormReason::Empty { at: "facts".into() } });
        check_len("facts", facts.len(), FACTS)?;
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
        let slide = match &spec.slide {
            Some(slide) => {
                self.deck().slide(slide).context(NoSlideSnafu { id: slide })?;
                self.slide_ref(slide)
            }
            None => None,
        };
        let facts = facts.into_iter().chain(written.iter().cloned()).collect();
        let spec = Report { facts, ..spec.clone() };
        let traces = self.open_traces();
        let surface = receipt(id, &spec, slide.as_ref(), &traces, &self.localizer);
        let plain = (!spec.actions.is_empty()).then(|| {
            receipt(id, &Report { actions: Vec::new(), ..spec.clone() }, slide.as_ref(), &traces, &self.localizer)
        });
        Ok(ReceiptSurfaces { surface, plain })
    }

    /// La slide `id` telle que rendue maintenant ; `None` tant qu'elle n'a pas de page rendue.
    fn slide_ref(&self, id: &str) -> Option<SlideRef> {
        let page = self.page_of(id)?;
        let hash = self.compiled.as_ref()?.page_hash(page.checked_sub(1)?)?;
        Some(SlideRef { id: id.to_owned(), hash: PageHash(Some(hash)).hex() })
    }
}

/// Le reçu, et sa version sans actions (`None` s'il n'en a pas) : celle qui reste une fois le tour passé.
#[derive(Debug, Clone, PartialEq)]
pub struct ReceiptSurfaces {
    /// The full receipt, with its actions (undo, next steps).
    pub surface: Surface,
    /// The same receipt without actions, shown once the turn is over.
    pub plain: Option<Surface>,
}

#[cfg(test)]
#[path = "receipt_tests.rs"]
mod receipt_tests;
