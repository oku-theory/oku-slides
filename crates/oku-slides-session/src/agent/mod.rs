//! Ce que la session offre à l'agent : images des slides, tours d'annulation,
//! surfaces (formes, variantes), contexte et la session d'un onglet
//! (`Sessions`, `TabSession`). Les outils MCP (`oku-slides-mcp`) et le
//! client ACP (`oku-slides-agent`) passent par ici.

pub mod context;
pub mod diff;
pub mod forms;
mod tabs;
pub mod variants;

pub use context::AgentUi;
pub use forms::{
    Anchor, AskChoice, AskConfirm, AskRewrite, FieldKind, ParamField, PermissionChoice, PermissionKind, Place,
    ShowParams, ShowVariants, Surface, Variant,
};
pub use tabs::{Sessions, TabSession};

use oku_slides_core::{Op, Paragraph, SlideField};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};

use crate::error::{AgentFormSnafu, NoSlideSnafu, RenderSnafu};
use crate::text::paragraphs_to_text;
use crate::{Result, Session};

/// Pixels par point des images pour l'agent : 960 × 540 pour une slide.
const AGENT_PIXEL_PER_PT: f32 = 0.5;

impl Session {
    /// Image PNG de la slide `id`, pour que l'agent voie le rendu.
    pub fn png(&self, id: &str) -> Result<Vec<u8>> {
        let compiled = self.compiled.as_ref().context(NoSlideSnafu { id })?;
        let page = compiled.layout().page_of(id).context(NoSlideSnafu { id })?;
        Ok(compiled.png(page - 1, AGENT_PIXEL_PER_PT).context(RenderSnafu)?)
    }
}

/// Refuse un nombre d'entrées hors de `min..=max` (`AgentForm`).
pub fn check_len(what: &str, len: usize, (min, max): (usize, usize)) -> Result<()> {
    snafu::ensure!(
        (min..=max).contains(&len),
        AgentFormSnafu { reason: format!("{what}: expected {min} to {max} entries, got {len}") }
    );
    Ok(())
}

/// Champ de slide visé par `@title`, `@subtitle`, `@kicker`, `@text`.
fn slide_field(element: &str) -> Option<SlideField> {
    match element {
        "@title" => Some(SlideField::Title),
        "@subtitle" => Some(SlideField::Subtitle),
        "@kicker" => Some(SlideField::Kicker),
        "@text" => Some(SlideField::Text),
        _ => None,
    }
}

impl Session {
    /// Texte actuel d'un élément ou d'un champ, pour une reformulation.
    fn text_of(&self, slide: &str, element: &str) -> Result<String> {
        let sl = self.deck().slide(slide).context(NoSlideSnafu { id: slide })?;
        let text = match slide_field(element) {
            Some(SlideField::Title) => sl.title.clone(),
            Some(SlideField::Subtitle) => sl.subtitle.clone(),
            Some(SlideField::Kicker) => sl.kicker.clone(),
            Some(_) => sl.text.clone(),
            None => sl.locate(element).and_then(|l| l.element.body.text().map(paragraphs_to_text)),
        };
        Ok(text.context(AgentFormSnafu { reason: format!("{slide}/{element}: no text to rewrite") })?)
    }

    /// Surface `ask_rewrite` : vérifie les propositions et lit le texte actuel.
    pub fn rewrite_surface(&self, id: &str, spec: &AskRewrite) -> Result<Surface> {
        check_len("proposals", spec.proposals.len(), forms::PROPOSALS)?;
        let original = self.text_of(&spec.slide, &spec.element)?;
        Ok(forms::rewrite(id, spec, &original, &self.localizer))
    }

    /// Ops qui remplacent le texte visé par `text` (champ) ou `paragraphs` (élément).
    ///
    /// Un champ n'a pas de paragraphes : `text` suffit. Un élément est du
    /// markdown lu par l'interface (`textToParagraphs`, `ui/src/deck.ts`) ;
    /// `paragraphs` en porte le résultat, absent seulement en cas d'erreur.
    pub fn rewrite_ops(&self, spec: &AskRewrite, text: &str, paragraphs: Option<Vec<Paragraph>>) -> Result<Vec<Op>> {
        self.text_of(&spec.slide, &spec.element)?;
        Ok(vec![match slide_field(&spec.element) {
            Some(field) => {
                Op::SetSlideField { slide: spec.slide.clone(), field, value: Value::String(text.to_owned()) }
            }
            None => {
                let paragraphs = paragraphs.context(AgentFormSnafu { reason: "paragraphs missing" })?;
                Op::SetText { slide: spec.slide.clone(), element: spec.element.clone(), text: paragraphs }
            }
        }])
    }
}
