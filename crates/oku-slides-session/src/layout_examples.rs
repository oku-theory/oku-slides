//! Mode pack, pack de layouts : une op sur la slide d'un layout s'écrit dans son exemple.
//!
//! Les exemples du manifeste édité sont la seule source des valeurs d'exemple
//! (`widget_examples`). La slide `layout-<nom>` du deck de travail n'en est que
//! l'aperçu : une transaction qui la change (réglage, liste, grille, éléments,
//! saisie sur la scène) passe par la version de la séance (une étape
//! d'annulation, reprise comme toute écriture du pack), puis la slide est
//! refaite depuis l'exemple (`realign_layouts`). Un texte vidé y reste `""`
//! (t-497) : la slide, elle, ne peut pas porter un texte vide.

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};

use oku_slides_core::{Deck, Moved, Op, ParamType, ParamValue, Params, Slide, WidgetFile, WidgetSchema};

use crate::error::{PackJsonSnafu, PackManifestSnafu, UnknownWidgetSnafu};
use crate::pack::{PackManifest, now_created};
use crate::pack_mode::realign_layouts;
use crate::pack_mode_save::AdoptStep;
use crate::widget_examples::emptied;
use crate::widget_params::same_as;
use crate::widget_preview::preview_params;
use crate::widgets_edit::trial_fits;
use crate::{Mode, Result, Session};

/// Les réglages vidés par `ops` : slide, paramètre et valeur envoyée (`set_param` d'une slide entière).
pub(crate) fn emptying_ops(ops: &[Op]) -> Vec<(String, String, Value)> {
    ops.iter()
        .filter_map(|op| match op {
            Op::SetParam { target, param, value } if target.element.is_none() => {
                Some((target.slide.clone(), param.clone(), value.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Un texte blanc ou une liste vide : une valeur vidée, selon le type du paramètre (`emptied`).
fn is_blank(v: &Value) -> bool {
    v.as_str().is_some_and(|s| s.trim().is_empty()) || v.as_array().is_some_and(Vec::is_empty)
}

/// La valeur vide d'une liste sans minimum ou d'un slot, vidés par une op de liste ou d'élément.
fn emptied_collection(ty: &ParamType) -> Option<Value> {
    match ty {
        ParamType::Slot { .. } => Some(Value::Array(Vec::new())),
        _ => emptied(ty, &Value::Array(Vec::new())),
    }
}

/// Le layout dont `slide` est l'aperçu dans le deck de travail (`layout-<nom>`).
fn layout_of(slide: &Slide) -> Option<WidgetFile> {
    slide.id.strip_prefix("layout-").and_then(WidgetFile::parse)
}

/// L'exemple `example` après le passage de la slide de `before` à `after`.
///
/// Seuls les paramètres changés bougent : présent, il prend sa nouvelle valeur (retirée si
/// elle est la valeur neutre) ; absent, il part, sauf vidé par la dernière op de `emptying`
/// qui le règle, où il garde la valeur vide, ou, sans elle, une liste sans minimum ou un slot
/// vidés par une op de liste ou d'élément (t-497). Les valeurs que le schéma ne lit plus sont
/// retirées (un slot reste).
fn next_example(
    mut example: Map<String, Value>,
    (before, after): (&Params, &Params),
    schema: &WidgetSchema,
    neutral: &Params,
    emptying: &[(&String, &Value)],
) -> Map<String, Value> {
    let keys: std::collections::BTreeSet<&String> = before.0.keys().chain(after.0.keys()).collect();
    for k in keys.into_iter().filter(|k| before.get(k) != after.get(k)) {
        let now = after.get(k).and_then(|v: &ParamValue| serde_json::to_value(v).ok());
        match now {
            Some(v) if same_as(neutral.get(k), &v) => {
                example.remove(k);
            }
            Some(v) => {
                example.insert(k.clone(), v);
            }
            None => {
                let ty = schema.param(k).map(|p| &p.ty);
                // La dernière op d'un lot qui règle le paramètre décide.
                let vide = match emptying.iter().rfind(|(p, _)| *p == k) {
                    Some((_, v)) => ty.and_then(|ty| emptied(ty, v)),
                    None => ty.and_then(emptied_collection),
                };
                match vide {
                    Some(v) => example.insert(k.clone(), v),
                    None => example.remove(k),
                };
            }
        }
    }
    // Un réglage vidé alors que la slide n'avait déjà pas la valeur : aucun changement de slide.
    for (k, v) in emptying {
        if before.get(k).is_none()
            && after.get(k).is_none()
            && let Some(empty) = schema.param(k).and_then(|p| emptied(&p.ty, v))
        {
            example.insert((*k).clone(), empty);
        }
    }
    // Un slot (les éléments posés sur la slide) ne se règle pas par une valeur : `trial_fits` le refuserait.
    let slot = |k: &str| schema.param(k).is_some_and(|p| matches!(p.ty, ParamType::Slot { .. }));
    example.retain(|k, v| slot(k) || trial_fits(schema, k, v));
    example
}

impl Session {
    /// En mode pack sur un pack de layouts, écrit dans les exemples les changements que `next`
    /// (le deck de travail après `ops`) apporte aux slides des layouts ; `false` s'il n'y en a
    /// pas : l'appelant inscrit `next` comme toute transaction.
    ///
    /// La transaction entière (paramètres du deck compris) passe en une étape `label`, de clé
    /// `merge`, avec la version de la séance ; les éléments de `moves` gardent leur nœud. Le
    /// deck rendu d'abord, un échec ne change rien.
    pub(crate) fn commit_layout_examples(
        &mut self,
        next: &Deck,
        ops: &[Op],
        label: &str,
        merge: Option<&str>,
        moves: &[Moved],
    ) -> Result<bool> {
        let Mode::Pack { edit, .. } = &self.mode else { return Ok(false) };
        let edit = edit.clone();
        let emptying = emptying_ops(ops);
        let current = self.doc.sources.deck();
        let changed: Vec<(&Slide, &Slide, WidgetFile)> = next
            .slides
            .iter()
            .filter_map(|after| {
                let name = layout_of(after)?;
                let before = current.slide(&after.id)?;
                let empties = emptying.iter().any(|(s, _, v)| *s == after.id && is_blank(v));
                (before.params != after.params || empties).then_some((before, after, name))
            })
            .collect();
        if changed.is_empty() {
            return Ok(false);
        }
        let t = self.widget_target(None)?;
        let mut manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        let examples_before = manifest.examples.clone();
        for (before, after, name) in &changed {
            let schema = self.registry.get(&after.kind).context(UnknownWidgetSnafu { name: name.as_str() })?;
            let neutral = preview_params(schema, &Map::new(), &self.localizer);
            let theirs: Vec<(&String, &Value)> =
                emptying.iter().filter(|(s, _, _)| *s == after.id).map(|(_, p, v)| (p, v)).collect();
            let example = manifest.examples.remove(name).unwrap_or_default();
            let example = next_example(example, (&before.params, &after.params), schema, &neutral, &theirs);
            if !example.is_empty() {
                manifest.examples.insert(name.clone(), example);
            }
        }
        // Rien de changé dans les exemples (une valeur neutre réglée à l'identique) : transaction ordinaire.
        if manifest.examples == examples_before {
            return Ok(false);
        }
        manifest.parent = edit.parent.clone();
        manifest.created = now_created();
        let files = manifest.to_files(t.kind, t.files.widgets().clone()).context(PackJsonSnafu { id: t.id })?;
        let mut deck = next.clone();
        realign_layouts(&mut deck, &manifest, &files);
        // Rendu avant l'écriture : un échec ne laisse ni version ni étape.
        self.render_deck(&deck)?;
        self.adopt_session_version(deck, files, &edit, false, AdoptStep::Op(label, merge, moves))?;
        self.rerender()?;
        self.doc.dirty = true;
        self.persist(true);
        Ok(true)
    }
}

#[cfg(test)]
#[path = "layout_examples_tests.rs"]
mod layout_examples_tests;
