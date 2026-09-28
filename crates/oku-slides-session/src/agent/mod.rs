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
    ShowParams, ShowVariants, SuggestNext, Suggestion, Surface, Variant,
};
pub use tabs::{Sessions, TabSession};

use oku_slides_core::{Op, Paragraph, ParamType, Target, apply_all};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};

use crate::error::{AgentFormSnafu, NoSlideSnafu, OpSnafu, RenderSnafu, ResponseSnafu};
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

impl Session {
    /// Premier paramètre `rich` du schéma du widget `kind`, s'il en a un.
    fn rich_param(&self, kind: &str) -> Option<&str> {
        self.registry.get(kind)?.params.iter().find(|p| p.ty == ParamType::Rich).map(|p| p.name.as_str())
    }

    /// Refuse `@field` s'il n'est pas déclaré par le schéma du widget de la
    /// slide, par le chemin d'une op refusée aujourd'hui (`SessionError::Op`,
    /// `OpErrorKind::UnknownParam`) : un `set_param` sonde (sur un clone du
    /// deck, jamais appliqué pour de vrai) porte le refus du cœur.
    fn check_field(&self, slide: &str, field: &str) -> Result<()> {
        let mut probe = self.deck().clone();
        let op = Op::SetParam {
            target: Target { slide: slide.to_owned(), element: None },
            param: field.to_owned(),
            value: Value::Null,
        };
        apply_all(&mut probe, vec![op], &self.registry).context(OpSnafu)?;
        Ok(())
    }

    /// Texte actuel d'un champ `@<paramètre>` de la slide, ou du premier
    /// paramètre `rich` d'un élément, pour une reformulation.
    fn text_of(&self, slide: &str, element: &str) -> Result<String> {
        let deck = self.deck();
        let sl = deck.slide(slide).context(NoSlideSnafu { id: slide })?;
        let text = match element.strip_prefix('@') {
            Some(field) => {
                self.check_field(slide, field)?;
                self.field_text(deck, sl, field).map(str::to_owned)
            }
            None => sl
                .locate(element)
                .and_then(|l| self.rich_param(&l.element.kind).and_then(|p| l.element.params.rich(p)))
                .map(paragraphs_to_text),
        };
        Ok(text.context(AgentFormSnafu { reason: format!("{slide}/{element}: no text to rewrite") })?)
    }

    /// Surface `ask_rewrite` : vérifie les propositions et lit le texte actuel.
    pub fn rewrite_surface(&self, id: &str, spec: &AskRewrite) -> Result<Surface> {
        check_len("proposals", spec.proposals.len(), forms::PROPOSALS)?;
        let original = self.text_of(&spec.slide, &spec.element)?;
        Ok(forms::rewrite(id, spec, &original, &self.localizer))
    }

    /// Ops qui remplacent le texte visé par `text` (champ `@<paramètre>`) ou
    /// `paragraphs` (élément, sur son premier paramètre `rich`).
    ///
    /// Un champ n'a pas de paragraphes : `text` suffit. Un élément est du
    /// markdown lu par l'interface (`textToParagraphs`, `ui/src/deck.ts`) ;
    /// `paragraphs` en porte le résultat, absent seulement en cas d'erreur.
    pub fn rewrite_ops(&self, spec: &AskRewrite, text: &str, paragraphs: Option<Vec<Paragraph>>) -> Result<Vec<Op>> {
        self.text_of(&spec.slide, &spec.element)?;
        let no_text = || AgentFormSnafu { reason: format!("{}/{}: no text to rewrite", spec.slide, spec.element) };
        Ok(vec![match spec.element.strip_prefix('@') {
            Some(field) => Op::SetParam {
                target: Target { slide: spec.slide.clone(), element: None },
                param: field.to_owned(),
                value: Value::String(text.to_owned()),
            },
            None => {
                let sl = self.deck().slide(&spec.slide).context(NoSlideSnafu { id: &spec.slide })?;
                let kind = sl.locate(&spec.element).map(|l| l.element.kind.clone()).context(no_text())?;
                let param = self.rich_param(&kind).context(no_text())?.to_owned();
                let paragraphs = paragraphs.context(AgentFormSnafu { reason: "paragraphs missing" })?;
                let value = serde_json::to_value(paragraphs).context(ResponseSnafu)?;
                Op::SetParam {
                    target: Target { slide: spec.slide.clone(), element: Some(spec.element.clone()) },
                    param,
                    value,
                }
            }
        }])
    }
}
