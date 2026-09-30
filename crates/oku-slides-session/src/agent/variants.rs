//! Variantes et paramètres proposés par l'agent : les variantes sont
//! rendues sur un clone du deck ; les gabarits `"$value"` des paramètres
//! sont vérifiés à l'appel, jamais quand l'utilisateur tourne le bouton.

use std::sync::Arc;

use oku_slides_core::{Op, apply_all};
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure};

use super::check_len;
use super::forms::{self, FieldKind, ParamField, ShowParams, ShowVariants, Surface};
use crate::error::{AgentFormSnafu, NoSlideSnafu, RenderSnafu};
use crate::{Result, Session, new_renderer};

/// Profondeur maximale d'un gabarit : au-delà, on le laisse tel quel plutôt
/// que de récurser indéfiniment sur une valeur JSON forgée par l'agent.
const MAX_TEMPLATE_DEPTH: usize = 32;

/// Remplace chaque chaîne exactement égale à `"$value"` par `value`.
pub(crate) fn fill_template(template: &Value, value: &Value) -> Value {
    fill_template_at(template, value, 0)
}

fn fill_template_at(template: &Value, value: &Value, depth: usize) -> Value {
    if depth >= MAX_TEMPLATE_DEPTH {
        return template.clone();
    }
    match template {
        Value::String(s) if s == "$value" => value.clone(),
        Value::Array(items) => Value::Array(items.iter().map(|t| fill_template_at(t, value, depth + 1)).collect()),
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, t)| (k.clone(), fill_template_at(t, value, depth + 1))).collect())
        }
        other => other.clone(),
    }
}

/// Gabarit rempli puis désérialisé en `Op`, sans l'habiller d'une erreur de
/// session (pour que l'appelant choisisse son propre préfixe).
fn parse_op(template: &Value, value: &Value) -> std::result::Result<Op, serde_json::Error> {
    serde_json::from_value(fill_template(template, value))
}

/// Bornes d'un champ à curseur, cohérentes pour son `kind`.
///
/// `options` n'est volontairement pas borné ici : le champ `theme_field` des
/// tests d'intégration (brief t-104) construit un `Select` avec des options
/// vides quand le gabarit ne s'en sert pas ; borner ce champ romprait ces
/// tests pour un cas qui reste inoffensif (une liste vide ne fait planter ni
/// la sérialisation ni le rendu, seulement l'ergonomie du widget).
fn check_field(i: usize, field: &ParamField) -> Result<()> {
    if matches!(field.kind, FieldKind::Slider | FieldKind::Stepper) {
        let finite = [field.min, field.max].into_iter().flatten().all(|v| v.is_finite());
        ensure!(finite, AgentFormSnafu { reason: format!("fields[{i}]: min/max must be finite") });
        // Un `Stepper` sans borne la laisse simplement non contrainte (`None`
        // reste `None` côté composant) : on ne compare que les deux bornes
        // présentes. Un `Slider`, lui, retombe sur `forms::SLIDER_DEFAULT_*`
        // quand une borne manque (`forms::params`) : on compare donc les
        // mêmes valeurs effectives, sinon une seule borne fournie (ex.
        // `min: Some(150.0), max: None`) passerait la validation puis
        // produirait quand même une plage inversée à l'affichage.
        let bounds = match field.kind {
            FieldKind::Slider => {
                Some((field.min.unwrap_or(forms::SLIDER_DEFAULT_MIN), field.max.unwrap_or(forms::SLIDER_DEFAULT_MAX)))
            }
            _ => field.min.zip(field.max),
        };
        if let Some((min, max)) = bounds {
            ensure!(min < max, AgentFormSnafu { reason: format!("fields[{i}]: min ({min}) must be < max ({max})") });
        }
    }
    Ok(())
}

impl Session {
    /// L'op d'un champ pour `value`.
    pub fn param_op(&self, field: &ParamField, value: &Value) -> Result<Op> {
        parse_op(&field.op_template, value)
            .map_err(|e| AgentFormSnafu { reason: format!("op_template: {e}") }.build().into())
    }

    /// Surface `show_params`. Chaque gabarit est rempli avec sa valeur
    /// actuelle et essayé sur un clone du deck.
    pub fn params_surface(&self, id: &str, spec: &ShowParams) -> Result<Surface> {
        check_len("fields", spec.fields.len(), forms::FIELDS)?;
        self.deck().slide(&spec.slide).context(NoSlideSnafu { id: &spec.slide })?;
        for (i, field) in spec.fields.iter().enumerate() {
            check_field(i, field)?;
            let op = parse_op(&field.op_template, &field.value)
                .map_err(|e| AgentFormSnafu { reason: format!("fields[{i}]: op_template: {e}") }.build())?;
            let mut deck = self.deck().clone();
            apply_all(&mut deck, vec![op], &self.registry)
                .map_err(|e| AgentFormSnafu { reason: format!("fields[{i}]: {e}") }.build())?;
        }
        let palette: Vec<String> = self.appearance.slide.values().cloned().collect();
        Ok(forms::params(id, spec, &palette, &self.localizer))
    }

    /// Surface `show_variants` : chaque variante appliquée à un clone, rendue,
    /// puis jetée. Une variante invalide fait échouer tout l'appel.
    ///
    /// Un pack qui fait échouer un rendu (délai, mémoire, arrêt) : repli sur
    /// la base, avec son avis, et variantes refaites une fois.
    pub fn variants_surface(&mut self, id: &str, spec: &ShowVariants) -> Result<Surface> {
        check_len("variants", spec.variants.len(), forms::VARIANTS)?;
        self.deck().slide(&spec.slide).context(NoSlideSnafu { id: &spec.slide })?;
        let svgs = self.with_fallback(|s| s.render_variants(spec))?;
        Ok(forms::variants(id, spec, &svgs, &self.localizer))
    }

    /// SVG de la slide de `spec` dans chaque variante, rendue dans son emplacement.
    fn render_variants(&mut self, spec: &ShowVariants) -> Result<Vec<Arc<str>>> {
        let pack = self.usable_pack()?;
        let renderer = match &mut self.variant_renderer {
            Some(r) => r,
            None => self.variant_renderer.insert(new_renderer(
                &self.worker,
                self.render_fonts.clone(),
                &self.localizer,
                pack.as_ref(),
                &self.registry,
            )?),
        };
        let mut svgs: Vec<Arc<str>> = Vec::with_capacity(spec.variants.len());
        for (i, variant) in spec.variants.iter().enumerate() {
            let (min, max) = forms::OPS;
            ensure!(
                (min..=max).contains(&variant.ops.len()),
                AgentFormSnafu {
                    reason: format!("variants[{i}]: ops: expected {min} to {max} entries, got {}", variant.ops.len())
                }
            );
            let mut deck = self.doc.sources.deck().clone();
            apply_all(&mut deck, variant.ops.clone(), &self.registry)
                .map_err(|e| AgentFormSnafu { reason: format!("variants[{i}]: {e}") }.build())?;
            let compiled = renderer.render(&deck, &self.doc.sources.assets).context(RenderSnafu)?;
            let page = compiled
                .layout()
                .page_of(&spec.slide)
                .context(AgentFormSnafu { reason: format!("variants[{i}]: slide {} removed", spec.slide) })?;
            svgs.push(compiled.svg(page - 1).context(RenderSnafu)?.into());
        }
        Ok(svgs)
    }
}

#[cfg(test)]
#[path = "variants_tests.rs"]
mod variants_tests;
