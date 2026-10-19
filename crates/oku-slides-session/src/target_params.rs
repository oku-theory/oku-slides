//! Paramètres de la cible : l'élément sélectionné, sinon le widget de la slide.
//! Lus dans le deck (jamais de valeur d'exemple ni neutre) et réglés par
//! `set_param` / `edit_list`, un pas d'annulation par réglage (fusionnés pour un curseur).
//! Un widget ou un paramètre sans `label` (ceux du noyau et de `base`) prend son nom traduit
//! (`session-widget-kind-<clé>`, `session-param-<clé>-<nom>` ; clé `content` ou `base-card`, voir `param_fields`).

use oku_slides_core::{ListOp, ListPath, Op, ParamSchema, ParamType, ParamValue, Params, Target, WidgetSchema};
use serde_json::Value;
use snafu::{OptionExt, ensure};

use crate::agent::forms::params_with;
use crate::agent::{FieldKind, ShowParams, Surface};
use crate::error::HostKeySnafu;
use crate::panel::{HOST_TARGET, HostParams};
use crate::param_fields::{coerce, field, kind_name, param_name, schema_default, settable};
use crate::{Result, Session};

/// La cible résolue : son schéma et les réglages qu'on en montre.
struct Resolved<'a> {
    schema: &'a WidgetSchema,
    params: HostParams,
}

impl Session {
    /// Réglages de la cible (surface `host-target`) ; `None` hors d'un deck modifiable
    /// (historique, aide), pour une cible disparue, un graphique ou sans champ.
    pub(crate) fn target_settings(&self, slide: &str, element: Option<&str>) -> Result<Option<Surface>> {
        if self.ensure_editable().is_err() {
            return Ok(None);
        }
        // Une sélection périmée (slide supprimée) n'est pas une erreur : rien à montrer.
        let Ok(Some(r)) = self.resolve_target(slide, element) else { return Ok(None) };
        let p = r.params;
        Ok(Some(params_with(HOST_TARGET, &p.spec, &p.extras(), &self.slide_palette(), &self.localizer)))
    }

    /// Règle le paramètre `key` de la cible : un pas d'annulation par réglage ; seuls
    /// les réglages successifs d'un même curseur (qu'on glisse) fusionnent en un pas.
    ///
    /// Pour une liste, `value` est le tableau de ses items (il remplace la liste).
    pub(crate) fn target_param(&mut self, slide: &str, element: Option<&str>, key: &str, value: Value) -> Result<()> {
        let (p, name, kind) = self.param_of(slide, element, key)?;
        // Un `null` ou un objet n'efface jamais une liste.
        ensure!(!matches!(p.ty, ParamType::List { .. }) || value.is_array(), HostKeySnafu { key });
        let op = Op::SetParam {
            target: target(slide, element),
            param: key.to_owned(),
            value: coerce(&p, value).unwrap_or(Value::Null),
        };
        let merge = (kind == FieldKind::Slider).then(|| format!("target:{slide}/{}/{key}", element.unwrap_or("")));
        let label = self.localizer.tr_with("session-undo-param", &[("label", name.into())]);
        self.apply(&label, vec![op], merge.as_deref())
    }

    /// Change la liste `key` de la cible : un pas d'annulation par changement. `key` est
    /// le paramètre, ou `param/item/colonne` pour la liste imbriquée d'un item.
    pub(crate) fn target_list(&mut self, slide: &str, element: Option<&str>, key: &str, change: ListOp) -> Result<()> {
        let path = ListPath::parse(key).context(HostKeySnafu { key })?;
        let (_, name, _) = self.param_of(slide, element, path.param)?;
        let op = Op::EditList { target: target(slide, element), param: key.to_owned(), change };
        let label = self.localizer.tr_with("session-undo-param", &[("label", name.into())]);
        self.apply(&label, vec![op], None)
    }

    /// Le schéma, le libellé et la nature du champ `key` de la cible ; `HostKey` si
    /// la cible ou le champ n'existent pas (lié au deck, non réglable, inconnu).
    fn param_of(&self, slide: &str, element: Option<&str>, key: &str) -> Result<(ParamSchema, String, FieldKind)> {
        self.ensure_editable()?;
        let r = self.resolve_target(slide, element)?.context(HostKeySnafu { key })?;
        let i = r.params.keys.iter().position(|k| k == key).context(HostKeySnafu { key })?;
        let f = r.params.spec.fields.get(i).context(HostKeySnafu { key })?;
        let p = r.schema.param(key).context(HostKeySnafu { key })?.clone();
        Ok((p, f.label.clone(), f.kind))
    }

    /// `None` : pas de cible montrable (élément disparu, sans schéma, graphique, sans champ).
    fn resolve_target(&self, slide: &str, element: Option<&str>) -> Result<Option<Resolved<'_>>> {
        let view = self.slide(slide)?;
        let (kind, params): (&str, &Params) = match element {
            Some(id) => match view.slide.locate(id) {
                Some(l) => (l.element.kind.as_str(), &l.element.params),
                None => return Ok(None),
            },
            None => (view.slide.kind.as_str(), &view.slide.params),
        };
        if crate::charts::chart_param(&self.registry, kind).is_some() {
            return Ok(None);
        }
        let Some(schema) = self.registry.get(kind) else { return Ok(None) };
        let l = &self.localizer;
        let (mut fields, mut keys, mut notes) = (Vec::new(), Vec::new(), Vec::new());
        for p in &schema.params {
            if let Some(var) = &p.bind {
                notes.push(l.tr_with(
                    "session-target-bound",
                    &[("name", param_name(kind, p, l).into()), ("var", var.as_str().into())],
                ));
                continue;
            }
            if !settable(&p.ty) {
                continue;
            }
            // Une liste absente du deck est vide au rendu : jamais ses items par défaut.
            let shown = match &p.ty {
                ParamType::List { .. } => Some(params.get(&p.name).cloned().unwrap_or(ParamValue::List(Vec::new()))),
                _ => params.get(&p.name).cloned().or_else(|| schema_default(p)),
            };
            if let Some(mut f) = field(p, shown.as_ref(), l) {
                f.label = param_name(kind, p, l);
                keys.push(p.name.clone());
                fields.push(f);
            }
        }
        if fields.is_empty() {
            return Ok(None);
        }
        let title = l.tr_with("session-widget-params", &[("name", kind_name(kind, schema.label.as_deref(), l).into())]);
        let spec = ShowParams { title, slide: slide.to_owned(), fields };
        Ok(Some(Resolved { schema, params: HostParams { spec, keys, notes } }))
    }
}

fn target(slide: &str, element: Option<&str>) -> Target {
    Target { slide: slide.to_owned(), element: element.map(str::to_owned) }
}

#[cfg(test)]
#[path = "target_params_tests.rs"]
mod target_params_tests;
