//! Variantes et paramètres proposés par l'agent : les variantes sont
//! rendues sur un clone du deck ; les gabarits `"$value"` des paramètres
//! sont vérifiés à l'appel, jamais quand l'utilisateur tourne le bouton.

use std::sync::Arc;

use oku_slides_core::{Deck, Op, Paragraph, apply_all};
use serde::Deserialize;
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure};

use super::forms::{self, FieldKind, ParamField, ShowParams, ShowVariants, Surface};
use super::{AgentFormReason, check_len, check_options};
use crate::deck_theme::fill_op_closures;
use crate::error::{AgentFormSnafu, NoSlideSnafu, RenderSnafu};
use crate::{Result, Session};

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

/// Un gabarit ou une valeur JSON d'une autre forme que celle attendue (`at` : où).
fn malformed(at: String, e: &serde_json::Error) -> crate::SessionError {
    AgentFormSnafu { reason: AgentFormReason::Malformed { at, detail: e.to_string() } }.build().into()
}

/// Des ops refusées par le deck (`at` : le champ ou la variante qui les porte).
fn refused(at: String, e: oku_slides_core::TransactionError) -> crate::SessionError {
    AgentFormSnafu { reason: AgentFormReason::Refused { at, source: Box::new(e) } }.build().into()
}

/// Valeur d'un champ riche (des paragraphes) et bornes d'un champ à curseur, cohérentes pour son `kind`.
///
/// Le nombre minimal d'`options` n'est volontairement pas borné ici (seuls un plafond large, leurs
/// identifiants uniques et leurs longueurs le sont) : le champ `theme_field` des
/// tests d'intégration (brief t-104) construit un `Select` avec des options
/// vides quand le gabarit ne s'en sert pas ; borner ce champ romprait ces
/// tests pour un cas qui reste inoffensif (une liste vide ne fait planter ni
/// la sérialisation ni le rendu, seulement l'ergonomie du widget).
fn check_field(i: usize, field: &ParamField) -> Result<()> {
    // Le tableau d'une liste envoie un changement, pas une valeur pour `op_template`.
    ensure!(
        field.kind != FieldKind::List,
        AgentFormSnafu {
            reason: AgentFormReason::NotAvailable {
                at: format!("fields[{i}]"),
                name: "list".into(),
                tool: "show_params"
            }
        }
    );
    if field.kind == FieldKind::Rich {
        ensure!(
            Vec::<Paragraph>::deserialize(&field.value).is_ok(),
            AgentFormSnafu { reason: AgentFormReason::NotParagraphs { at: format!("fields[{i}]") } }
        );
    }
    if field.kind == FieldKind::Select {
        let max = forms::SELECT_OPTIONS_MAX;
        ensure!(
            field.options.len() <= max,
            AgentFormSnafu {
                reason: AgentFormReason::TooMany {
                    at: format!("fields[{i}].options"),
                    max,
                    count: field.options.len()
                }
            }
        );
        check_options(&format!("fields[{i}].options"), &field.options)?;
    }
    if matches!(field.kind, FieldKind::Slider | FieldKind::Stepper) {
        let finite = [field.min, field.max].into_iter().flatten().all(f64::is_finite);
        ensure!(finite, AgentFormSnafu { reason: AgentFormReason::NotFinite { at: format!("fields[{i}]") } });
        // A `Stepper` without a bound passes `None` to the component, which falls
        // back on 1 and 24 (`ui/src/a2ui/catalog.tsx`): only the two bounds present
        // are compared here. A `Slider`, however, falls back on `forms::SLIDER_DEFAULT_*`
        // when a bound is missing (`forms::params`): so the same effective values are
        // compared, otherwise a single bound (e.g. `min: Some(150.0), max: None`)
        // would pass validation and still produce an inverted range on display.
        let bounds = match field.kind {
            FieldKind::Slider => {
                Some((field.min.unwrap_or(forms::SLIDER_DEFAULT_MIN), field.max.unwrap_or(forms::SLIDER_DEFAULT_MAX)))
            }
            _ => field.min.zip(field.max),
        };
        if let Some((min, max)) = bounds {
            ensure!(
                min < max,
                AgentFormSnafu { reason: AgentFormReason::BoundsInverted { at: format!("fields[{i}]"), min, max } }
            );
        }
    }
    Ok(())
}

impl Session {
    /// The op of a field for `value`.
    ///
    /// # Errors
    ///
    /// Fails (`AgentForm`) if template `op_template` filled with `value` is not a valid op.
    pub fn param_op(&self, field: &ParamField, value: &Value) -> Result<Op> {
        parse_op(&field.op_template, value).map_err(|e| malformed("op_template".into(), &e))
    }

    /// `show_params` surface.
    ///
    /// Each template is filled with its current value and tried on a clone of the deck.
    ///
    /// # Errors
    ///
    /// Fails if the number of fields is out of bounds, if the slide is unknown, or if a template is
    /// malformed or rejected on the deck clone.
    pub fn params_surface(&self, id: &str, spec: &ShowParams) -> Result<Surface> {
        check_len("fields", spec.fields.len(), forms::FIELDS)?;
        self.deck().slide(&spec.slide).context(NoSlideSnafu { id: &spec.slide })?;
        for (i, field) in spec.fields.iter().enumerate() {
            check_field(i, field)?;
            let op = parse_op(&field.op_template, &field.value)
                .map_err(|e| malformed(format!("fields[{i}].op_template"), &e))?;
            // Un `set_theme` sans épinglages prend sa fermeture, comme dans `apply` ; rien n'est embarqué.
            let mut ops = vec![op];
            fill_op_closures(&self.themes, &self.doc.sources.themes, &mut ops)?;
            let mut deck = self.deck().clone();
            apply_all(&mut deck, ops, &self.registry).map_err(|e| refused(format!("fields[{i}]"), e))?;
        }
        let palette = self.slide_palette();
        Ok(forms::params(id, spec, &palette, &self.localizer))
    }

    /// `show_variants` surface: each variant applied to a clone, rendered, then dropped.
    ///
    /// An invalid variant fails the whole call. A pack that makes a render fail (timeout, memory,
    /// abort): fallback to the base, with its notice, and the variants redone once.
    ///
    /// # Errors
    ///
    /// Fails if the number of variants is out of bounds, if the slide is unknown, or if a variant is
    /// invalid or does not render.
    pub fn variants_surface(&mut self, id: &str, spec: &ShowVariants) -> Result<Surface> {
        check_len("variants", spec.variants.len(), forms::VARIANTS)?;
        self.deck().slide(&spec.slide).context(NoSlideSnafu { id: &spec.slide })?;
        let svgs = self.with_fallback(|s| s.variant_decks(spec), |s| s.render_variants(spec))?;
        Ok(forms::variants(id, spec, &svgs, &self.localizer))
    }

    /// SVG de la slide de `spec` dans chaque variante, rendue dans son emplacement.
    fn render_variants(&mut self, spec: &ShowVariants) -> Result<Vec<Arc<str>>> {
        let set = self.usable_packs()?;
        // Après `usable_packs` : un pack écarté n'est plus au registre qui valide les variantes.
        let decks = self.variant_decks(spec)?;
        let layer = self.theme_layer(self.doc.sources.deck())?;
        let mut svgs: Vec<Arc<str>> = Vec::with_capacity(decks.len());
        for (i, deck) in decks.iter().enumerate() {
            // Le thème du deck : une variante qui en changerait garde celui-ci à l'aperçu.
            let compiled = self.render_deck_preview(&set, deck, &layer)?;
            let page = compiled.layout().page_of(&spec.slide).context(AgentFormSnafu {
                reason: AgentFormReason::SlideRemoved { at: format!("variants[{i}]"), slide: spec.slide.clone() },
            })?;
            svgs.push(compiled.svg(page - 1).context(RenderSnafu)?.into());
        }
        Ok(svgs)
    }

    /// Le deck de chaque variante de `spec` : un clone du deck, ses ops appliquées.
    fn variant_decks(&self, spec: &ShowVariants) -> Result<Vec<Deck>> {
        let mut decks = Vec::with_capacity(spec.variants.len());
        for (i, variant) in spec.variants.iter().enumerate() {
            let (min, max) = forms::OPS;
            ensure!(
                (min..=max).contains(&variant.ops.len()),
                AgentFormSnafu {
                    reason: AgentFormReason::Count {
                        at: format!("variants[{i}].ops"),
                        min,
                        max,
                        count: variant.ops.len()
                    }
                }
            );
            // Un `set_theme` sans épinglages prend sa fermeture, comme dans `apply` ; rien n'est embarqué.
            let mut ops = variant.ops.clone();
            fill_op_closures(&self.themes, &self.doc.sources.themes, &mut ops)?;
            let mut deck = self.doc.sources.deck().clone();
            apply_all(&mut deck, ops, &self.registry).map_err(|e| refused(format!("variants[{i}]"), e))?;
            decks.push(deck);
        }
        Ok(decks)
    }
}

#[cfg(test)]
#[path = "variants_tests.rs"]
mod variants_tests;
