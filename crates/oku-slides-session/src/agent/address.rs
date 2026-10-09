//! Adresse d'un champ par l'agent : `{slide, owner, field}` (fiches ancrées, `ask_rewrite`).
//!
//! L'adresse vient de l'agent : elle n'est jamais prise telle quelle. Elle est résolue contre le
//! deck actif — la slide, puis l'objet (la slide ou un de ses éléments), puis un champ texte que
//! le schéma du widget de l'objet déclare (`Slide::text_fields`). Ce qui sort d'ici est le
//! paramètre du schéma, pas le nom reçu : une op `set_param` le porte, aucun Typst ne le lit.

use oku_slides_core::{FieldOwner, Op, Paragraph, ParamType, Target, TextField};
use serde_json::Value;
use snafu::{OptionExt, ResultExt};

use super::forms::{self, Anchor, AskRewrite, Surface};
use super::{AgentFormReason, check_len};
use crate::error::{
    AgentFormSnafu, FieldNeededSnafu, NoSlideSnafu, ResponseSnafu, UnknownFieldSnafu, UnknownOwnerSnafu,
};
use crate::{Result, Session};

impl Session {
    /// Le champ texte `field` de `owner` sur la slide `slide` ; sans `field`, le premier
    /// paramètre `rich` d'un élément. Les cellules d'une liste ne sont pas adressables.
    fn resolve_field(&self, slide: &str, owner: &str, field: Option<&str>) -> Result<TextField<'_>> {
        let sl = self.deck().slide(slide).context(NoSlideSnafu { id: slide })?;
        let is_slide = owner == sl.id;
        snafu::ensure!(is_slide || sl.locate(owner).is_some(), UnknownOwnerSnafu { slide, owner });
        let mut own = sl.text_fields(&self.registry).into_iter().filter(|tf| tf.owner_id == owner && tf.list.is_none());
        let found = match field {
            Some(name) => own.find(|tf| tf.param.name == name).context(UnknownFieldSnafu { owner, field: name })?,
            None if is_slide => FieldNeededSnafu { owner }.fail()?,
            None => own.find(|tf| tf.param.ty == ParamType::Rich).context(FieldNeededSnafu { owner })?,
        };
        Ok(found)
    }

    /// Refuse une ancre hors du deck actif : slide inconnue, objet qui n'est ni la slide ni un
    /// de ses éléments, champ que l'objet ne déclare pas ; une slide se vise par un de ses champs.
    pub fn check_anchor(&self, anchor: &Anchor) -> Result<()> {
        match &anchor.field {
            Some(field) => self.resolve_field(&anchor.slide, &anchor.owner, Some(field)).map(drop),
            None => {
                let sl = self.deck().slide(&anchor.slide).context(NoSlideSnafu { id: &anchor.slide })?;
                snafu::ensure!(anchor.owner != sl.id, FieldNeededSnafu { owner: &anchor.owner });
                snafu::ensure!(
                    sl.locate(&anchor.owner).is_some(),
                    UnknownOwnerSnafu { slide: &anchor.slide, owner: &anchor.owner }
                );
                Ok(())
            }
        }
    }

    /// Surface `ask_rewrite` : vérifie les propositions et l'adresse, lit le texte actuel.
    pub fn rewrite_surface(&self, id: &str, spec: &AskRewrite) -> Result<Surface> {
        check_len("proposals", spec.proposals.len(), forms::PROPOSALS)?;
        let tf = self.resolve_field(&spec.slide, &spec.owner, spec.field.as_deref())?;
        let original = crate::fields::field_text(self.deck(), &tf);
        Ok(forms::rewrite(id, spec, &original, &self.localizer))
    }

    /// Ops qui remplacent le texte du champ visé : `text` pour un champ `plain`, `paragraphs`
    /// pour un champ `rich`.
    ///
    /// Un champ `rich` est du markdown lu par l'interface (`textToParagraphs`, `ui/src/deck.ts`) ;
    /// `paragraphs` en porte le résultat, absent seulement en cas d'erreur. L'adresse est résolue
    /// de nouveau : le deck a pu changer depuis la question.
    pub fn rewrite_ops(&self, spec: &AskRewrite, text: &str, paragraphs: Option<Vec<Paragraph>>) -> Result<Vec<Op>> {
        let tf = self.resolve_field(&spec.slide, &spec.owner, spec.field.as_deref())?;
        let element = match tf.owner {
            FieldOwner::Slide => None,
            FieldOwner::Element(e) => Some(e.id.clone()),
        };
        let value = if tf.param.ty == ParamType::Rich {
            let paragraphs =
                paragraphs.context(AgentFormSnafu { reason: AgentFormReason::Missing { at: "paragraphs".into() } })?;
            serde_json::to_value(paragraphs).context(ResponseSnafu)?
        } else {
            Value::String(text.to_owned())
        };
        Ok(vec![Op::SetParam {
            target: Target { slide: spec.slide.clone(), element },
            param: tf.param.name.clone(),
            value,
        }])
    }
}
