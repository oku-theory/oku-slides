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
        let finite = [field.min, field.max].into_iter().flatten().all(|v| v.is_finite());
        ensure!(finite, AgentFormSnafu { reason: AgentFormReason::NotFinite { at: format!("fields[{i}]") } });
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
            ensure!(
                min < max,
                AgentFormSnafu { reason: AgentFormReason::BoundsInverted { at: format!("fields[{i}]"), min, max } }
            );
        }
    }
    Ok(())
}

impl Session {
    /// L'op d'un champ pour `value`.
    pub fn param_op(&self, field: &ParamField, value: &Value) -> Result<Op> {
        parse_op(&field.op_template, value).map_err(|e| malformed("op_template".into(), &e))
    }

    /// Surface `show_params`. Chaque gabarit est rempli avec sa valeur
    /// actuelle et essayé sur un clone du deck.
    pub fn params_surface(&self, id: &str, spec: &ShowParams) -> Result<Surface> {
        check_len("fields", spec.fields.len(), forms::FIELDS)?;
        self.deck().slide(&spec.slide).context(NoSlideSnafu { id: &spec.slide })?;
        for (i, field) in spec.fields.iter().enumerate() {
            check_field(i, field)?;
            let op = parse_op(&field.op_template, &field.value)
                .map_err(|e| malformed(format!("fields[{i}].op_template"), &e))?;
            let mut deck = self.deck().clone();
            apply_all(&mut deck, vec![op], &self.registry).map_err(|e| refused(format!("fields[{i}]"), e))?;
        }
        let palette = self.slide_palette();
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
        let svgs = self.with_fallback(|s| s.variant_decks(spec), |s| s.render_variants(spec))?;
        Ok(forms::variants(id, spec, &svgs, &self.localizer))
    }

    /// SVG de la slide de `spec` dans chaque variante, rendue dans son emplacement.
    fn render_variants(&mut self, spec: &ShowVariants) -> Result<Vec<Arc<str>>> {
        let set = self.usable_packs()?;
        // Après `usable_packs` : un pack écarté n'est plus au registre qui valide les variantes.
        let decks = self.variant_decks(spec)?;
        let tokens = self.theme_tokens(self.doc.sources.deck());
        let renderer = match &mut self.variant_renderer {
            Some(r) => r,
            None => self.variant_renderer.insert(new_renderer(
                &self.worker,
                self.render_fonts.clone(),
                &self.localizer,
                &set,
                &self.registry,
            )?),
        };
        let mut svgs: Vec<Arc<str>> = Vec::with_capacity(decks.len());
        for (i, deck) in decks.iter().enumerate() {
            // Le thème du deck : une variante qui en changerait garde celui-ci à l'aperçu.
            let compiled = crate::deck_theme::render_themed(renderer, deck, tokens.as_ref(), &self.doc.sources.assets)
                .context(RenderSnafu)?;
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
            let mut deck = self.doc.sources.deck().clone();
            apply_all(&mut deck, variant.ops.clone(), &self.registry)
                .map_err(|e| refused(format!("variants[{i}]"), e))?;
            decks.push(deck);
        }
        Ok(decks)
    }
}

#[cfg(test)]
#[path = "variants_tests.rs"]
mod variants_tests;
