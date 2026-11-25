//! Valeurs d'exemple des widgets : un jeu de valeurs par widget, dans le pack.
//!
//! Gardées dans `widgets.json` (`PackManifest::examples`) : versionnées, hachées
//! et exportées avec le pack. Un widget sans valeurs gardées montre les valeurs
//! neutres ; il n'a d'entrée qu'à son premier changement, et la perd quand ses
//! valeurs redeviennent neutres.
//!
//! Ils sont la seule source des exemples, layouts compris : en mode pack, la
//! slide `layout-<nom>` du deck de travail n'en est que l'aperçu, refaite
//! depuis le manifeste à chaque écriture (`pack_mode::realign_layouts`) ; une
//! op sur elle s'écrit dans l'exemple (`layout_examples`).
//!
//! Chaque changement passe, en mode pack seulement, par le chemin des
//! écritures du pack (`commit_manifest` → version de la séance, une étape
//! d'annulation du deck mis de côté). Les valeurs se contrôlent comme celles
//! des réglages (`widget_params.rs`) ; une valeur que le schéma n'admet plus
//! est ignorée à l'aperçu et retirée à la prochaine écriture du widget. Une
//! liste gardée que le schéma ne lit plus repart, au geste suivant sur elle
//! (`edit_example_list`), des items par défaut que montre l'aperçu.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::{
    ListItem, ListOp, ListPath, ListSpec, Namespace, Op, PackPin, ParamType, ParamValue, Params, WidgetFile,
    check_external, default_items, param_value, split_kind,
};
use oku_slides_data::{Data, DataId};

use crate::agent::clip;
use crate::data_copy::copy_data_ops;
use crate::error::{
    NotEditingPackSnafu, PackManifestSnafu, PackVersionMissingSnafu, PrepareSnafu, TrialEncodeSnafu,
    TrialUndeclaredSnafu, TrialValueSnafu, UnknownWidgetSnafu, WidgetInvalidSnafu,
};
use crate::example_text::{localized_data, localized_examples};
use crate::pack::PackManifest;
use crate::pack_mode::styled_like;
use crate::widget_params::{checked_value, same_as};
use crate::widget_preview::preview_params;
use crate::widgets_edit::{WidgetTarget, parse_name, trial_fits};
use crate::{Mode, Result, Session};

/// Chiffres hexadécimaux de l'empreinte du pack dans la révision : 52 bits,
/// un entier exact en JavaScript.
const REVISION_HEX: usize = 13;

impl Session {
    /// Révision des valeurs d'exemple pour l'interface : un changement relance les aperçus.
    ///
    /// Tirée de l'empreinte de la version éditée, qui couvre `widgets.json` et
    /// donc les valeurs : elle change avec elles (et avec toute autre écriture
    /// du pack) ; pour un pack de layouts, aussi des slides du deck de travail,
    /// ses exemples vivants. `0` hors du mode pack.
    pub(crate) fn examples_revision(&self) -> u64 {
        let Mode::Pack { edit, .. } = &self.mode else { return 0 };
        let base = edit.base.hex().get(..REVISION_HEX).and_then(|h| u64::from_str_radix(h, 16).ok());
        let slides = &self.doc.sources.deck().slides;
        if slides.is_empty() {
            return base.unwrap_or_default();
        }
        let mut hasher = DefaultHasher::new();
        base.hash(&mut hasher);
        // Comparée dans la même session seulement : la forme de débogage suffit.
        format!("{slides:?}").hash(&mut hasher);
        hasher.finish() >> (64 - 4 * REVISION_HEX)
    }

    /// Les valeurs d'exemple de `name` dans le pack `pack` (voir `widget_target`) ; aucune s'il n'en a pas.
    ///
    /// Celles d'un pack intégré sont traduites (`localized_examples`).
    pub(crate) fn example_values(&self, pack: Option<&Namespace>, name: &WidgetFile) -> Result<Map<String, Value>> {
        let t = self.widget_target(pack)?;
        let manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        let values = manifest.examples.get(name).map(|v| localized_examples(manifest.id, v, &self.localizer));
        Ok(values.unwrap_or_default())
    }

    /// The example data of pack `pack` (see `widget_target`), localized as its values are.
    pub(crate) fn example_data(&self, pack: Option<&Namespace>) -> Result<BTreeMap<DataId, Arc<Data>>> {
        let t = self.widget_target(pack)?;
        let manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        Ok(localized_data(manifest.id, &manifest.data, &self.localizer))
    }

    /// `ops` with one `AddData` before them per pack example data their new elements and slides name.
    ///
    /// A view of an added element or slide naming a data the deck lacks, and no `AddData` of
    /// the batch adds, is read from the manifest of the added kind's pack, else of its own
    /// kind's pack (pinned in `pins`): a layout's example holds its elements' data, a widget's
    /// its own. The data is copied once (`copy_data_ops`, under an id the batch does not add)
    /// and every such view names the copy.
    ///
    /// Any view naming a pinned pack's example data id gets that data, whatever brought the
    /// element: a typed one too. A paste never comes here: its refs name copies of its
    /// source's data (`with_pasted_data`).
    /// A batch with copies is checked as external, then prepared as the host's (the copies
    /// carry their cells); `true` says so.
    ///
    /// # Errors
    ///
    /// `Prepare` for a batch the core refuses; a pinned version missing or with a broken manifest.
    pub(crate) fn with_example_data(
        &self,
        pins: &BTreeMap<Namespace, PackPin>,
        mut ops: Vec<Op>,
    ) -> Result<(Vec<Op>, bool)> {
        let deck = self.doc.sources.deck();
        let added: BTreeSet<String> = ops.iter().filter_map(op_data_id).map(str::to_owned).collect();
        let mut found: BTreeSet<Owned> = BTreeSet::new();
        for op in &mut ops {
            each_added(op, &mut |owners, params| {
                for id in view_refs(params) {
                    if !deck.data.contains_key(id.as_str()) && !added.contains(id.as_str()) {
                        found.insert((owners.clone(), id));
                    }
                }
            });
        }
        let mut sources: BTreeMap<Namespace, BTreeMap<DataId, Arc<Data>>> = BTreeMap::new();
        let mut from: BTreeMap<Owned, Namespace> = BTreeMap::new();
        for (owners, id) in found {
            for ns in [&owners.0, &owners.1] {
                if !sources.contains_key(ns) {
                    sources.insert(ns.clone(), self.pinned_data(pins, ns)?);
                }
                if sources.get(ns).is_some_and(|d| d.contains_key(&id)) {
                    from.insert((owners.clone(), id.clone()), ns.clone());
                    break;
                }
            }
        }
        let mut scratch = styled_like(deck, Vec::new());
        let mut copies = Vec::new();
        let mut maps: BTreeMap<Namespace, BTreeMap<DataId, DataId>> = BTreeMap::new();
        for (ns, source) in &sources {
            let refs: Vec<DataId> = from.iter().filter(|(_, n)| *n == ns).map(|((_, id), _)| id.clone()).collect();
            if refs.is_empty() {
                continue;
            }
            let (ops, map) =
                copy_data_ops(source, &refs, &scratch, &added, &mut oku_slides_data::new_id).context(PrepareSnafu)?;
            for op in &ops {
                if let Op::AddData { id: Some(new), name, .. } = op
                    && let Some(data) = map.iter().find(|(_, n)| *n == new).and_then(|(old, _)| source.get(old))
                {
                    scratch.data.insert(new.clone(), Arc::new(Data { name: name.clone(), ..Data::clone(data) }));
                }
            }
            copies.extend(ops);
            maps.insert(ns.clone(), map);
        }
        if copies.is_empty() {
            return Ok((ops, false));
        }
        for op in &mut ops {
            each_added(op, &mut |owners, params| {
                rename_refs(params, &|id| {
                    let ns = from.get(&(owners.clone(), id.clone()))?;
                    maps.get(ns)?.get(id).cloned()
                });
            });
        }
        check_external(&ops).context(PrepareSnafu)?;
        copies.extend(ops);
        Ok((copies, true))
    }

    /// The example data of the pack pinned under `ns` in `pins`, localized; none if `ns` is not pinned.
    fn pinned_data(&self, pins: &BTreeMap<Namespace, PackPin>, ns: &Namespace) -> Result<BTreeMap<DataId, Arc<Data>>> {
        let Some(pin) = pins.get(ns) else { return Ok(BTreeMap::new()) };
        let files = self.doc.sources.packs.get(&pin.version);
        let files = files.context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        let manifest = PackManifest::from_files(files).context(PackManifestSnafu { id: pin.id })?;
        Ok(localized_data(manifest.id, &manifest.data, &self.localizer))
    }

    /// Sets example values of `widget`, all or nothing.
    ///
    /// `null` or the neutral value clears a value; an emptied text or an emptied list
    /// (`[]`) stays empty, see `emptied`; kept values the schema no longer accepts
    /// are removed. Returns the widget's values after the write.
    ///
    /// # Errors
    ///
    /// Fails for an invalid name, outside pack mode (`NotEditingPack`), for `_commun` or a widget
    /// that is unknown or invalid, or a rejected value.
    pub fn set_example_values(&mut self, widget: &str, values: Map<String, Value>) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_pack_mode(), NotEditingPackSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        let neutral = preview_params(&schema, &Map::new(), &self.localizer);
        let mut planned = Vec::with_capacity(values.len());
        for (param, value) in values {
            let p = schema.param(&param).context(TrialUndeclaredSnafu { param: clip(&param, 80) })?;
            let kept = match emptied(&p.ty, &value) {
                Some(empty) => Some(empty),
                None => checked_value(p, value)?.filter(|v| !same_as(neutral.get(&param), v)),
            };
            planned.push((param, kept));
        }
        self.write_examples(widget, |values| {
            values.retain(|k, v| trial_fits(&schema, k, v));
            for (param, kept) in planned {
                match kept {
                    Some(v) => values.insert(param, v),
                    None => values.remove(&param),
                };
            }
            Ok(())
        })
    }

    /// Changes the list `param` of the example values of `widget`.
    ///
    /// Without a kept value, or with one the schema no longer reads (renamed
    /// column, lowered `max`), the list starts from the schema's default items —
    /// what the preview and the table show; a kept value that is not a list is
    /// rejected. When emptied, the list stays `[]`. Returns the widget's values
    /// after the write.
    ///
    /// # Errors
    ///
    /// Fails for an invalid name, outside pack mode, for an unknown or invalid widget, or a parameter
    /// that is undeclared or not a list.
    pub fn edit_example_list(&mut self, widget: &str, param: &str, change: ListOp) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_pack_mode(), NotEditingPackSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let schema = self.valid_schema(&file)?.context(WidgetInvalidSnafu { name: file.as_str() })?;
        // `param` : le paramètre, ou `param/item/colonne` pour la liste imbriquée d'un item.
        let path = ListPath::parse(param).context(TrialUndeclaredSnafu { param: clip(param, 80) })?;
        let p = schema.param(path.param).context(TrialUndeclaredSnafu { param: clip(param, 80) })?;
        let spec = ListSpec::of(&p.name, &p.ty).context(TrialUndeclaredSnafu { param: clip(param, 80) })?;
        let defaults = default_items(p).context(TrialValueSnafu { param })?;
        self.write_examples(widget, |values| {
            // Lue avant le tri des valeurs gardées : une liste que le schéma ne lit plus repart
            // des défauts (ce que montrent l'aperçu et le tableau) ; une non-liste est une erreur.
            let mut items: Vec<ListItem> = match values.get(path.param) {
                Some(v @ Value::Array(_)) => match param_value(&p.ty, path.param, v.clone()) {
                    Ok(Some(ParamValue::List(items))) => items,
                    Ok(_) | Err(_) => defaults,
                },
                Some(v) => match param_value(&p.ty, path.param, v.clone()).context(TrialValueSnafu { param })? {
                    Some(ParamValue::List(items)) => items,
                    _ => Vec::new(),
                },
                None => defaults,
            };
            values.retain(|k, v| trial_fits(&schema, k, v));
            path.apply(&mut items, &spec, change).context(TrialValueSnafu { param })?;
            let stored = serde_json::to_value(&items).context(TrialEncodeSnafu { param })?;
            values.insert(path.param.to_owned(), stored);
            Ok(())
        })
    }

    /// Agent write (tool `set_widget_trial`): the widget's values, written and persisted.
    ///
    /// # Errors
    ///
    /// Fails like [`Self::set_example_values`].
    pub fn set_example_values_by_agent(
        &mut self,
        widget: &str,
        values: Map<String, Value>,
    ) -> Result<Map<String, Value>> {
        let written = self.set_example_values(widget, values)?;
        self.persist(true);
        Ok(written)
    }

    /// Change les valeurs d'exemple de `widget` par `change`, puis écrit le manifeste
    /// (mode pack seulement) ; rend les valeurs écrites.
    ///
    /// Redevenues vides, les valeurs ne sont plus enregistrées. La slide d'un layout
    /// suit son exemple, dans la même étape (`commit_manifest`).
    fn write_examples(
        &mut self,
        widget: &str,
        change: impl FnOnce(&mut Map<String, Value>) -> Result<()>,
    ) -> Result<Map<String, Value>> {
        let file = parse_name(widget)?;
        ensure!(self.in_pack_mode(), NotEditingPackSnafu);
        ensure!(!file.is_common(), UnknownWidgetSnafu { name: file.as_str() });
        let t: WidgetTarget = self.widget_target(None)?;
        ensure!(t.files.widget(&file).is_some(), UnknownWidgetSnafu { name: file.as_str() });
        let mut manifest = PackManifest::from_files(&t.files).context(PackManifestSnafu { id: t.id })?;
        let mut values = manifest.examples.remove(&file).unwrap_or_default();
        change(&mut values)?;
        if !values.is_empty() {
            manifest.examples.insert(file, values.clone());
        }
        let widgets = t.files.widgets().clone();
        self.commit_manifest(&t, manifest, widgets)?;
        Ok(values)
    }
}

/// The id an `AddData` names.
pub(crate) fn op_data_id(op: &Op) -> Option<&str> {
    match op {
        Op::AddData { id: Some(id), .. } => Some(id.as_str()),
        _ => None,
    }
}

/// The namespaces of a view's owners: the added element's or slide's, then the view's own element's.
pub(crate) type Owners = (Namespace, Namespace);

/// A view's data ref with its owners.
type Owned = (Owners, DataId);

/// Calls `f` on the params of the element or slide `op` adds and of the elements of its slots,
/// at any depth, with their `Owners`. A kind without a namespace takes its parent's.
pub(crate) fn each_added(op: &mut Op, f: &mut dyn FnMut(&Owners, &mut Params)) {
    let (kind, params) = match op {
        Op::AddElement { element, .. } => (&element.kind, &mut element.params),
        Op::AddSlide { slide, .. } => (&slide.kind, &mut slide.params),
        _ => return,
    };
    if let Some(ns) = namespace_of(kind) {
        each_params(&(ns.clone(), ns), params, f);
    }
}

/// `f` on `params`, then on the params of the elements of its slots.
fn each_params(owners: &Owners, params: &mut Params, f: &mut dyn FnMut(&Owners, &mut Params)) {
    f(owners, params);
    for value in params.0.values_mut() {
        if let ParamValue::Slot(elements) = value {
            for e in elements {
                let own = namespace_of(&e.kind).unwrap_or_else(|| owners.1.clone());
                each_params(&(owners.0.clone(), own), &mut e.params, f);
            }
        }
    }
}

/// The namespace of `kind`, if it has one.
fn namespace_of(kind: &str) -> Option<Namespace> {
    split_kind(kind).and_then(|(ns, _)| Namespace::parse(ns))
}

/// The data the views of `params` name, as `Deck::data_users` reads them.
pub(crate) fn view_refs(params: &Params) -> Vec<DataId> {
    params
        .0
        .values()
        .filter_map(|v| match v {
            ParamValue::Value(v) => v.get("data").and_then(Value::as_str),
            ParamValue::Text(_) | ParamValue::Rich(_) | ParamValue::Slot(_) | ParamValue::List(_) => None,
        })
        .filter_map(|id| DataId::try_from(id.to_owned()).ok())
        .collect()
}

/// The views of `params` name the copy `copy` gives for their data, if any.
pub(crate) fn rename_refs(params: &mut Params, copy: &dyn Fn(&DataId) -> Option<DataId>) {
    for value in params.0.values_mut() {
        if let ParamValue::Value(v) = value
            && let Some(Value::String(id)) = v.get_mut("data")
            && let Ok(old) = DataId::try_from(id.clone())
            && let Some(new) = copy(&old)
        {
            new.as_str().clone_into(id);
        }
    }
}

#[cfg(test)]
#[path = "widget_examples_tests.rs"]
mod widget_examples_tests;

#[cfg(test)]
#[path = "widget_examples_data_tests.rs"]
mod widget_examples_data_tests;

#[cfg(test)]
#[path = "widget_examples_list_tests.rs"]
mod widget_examples_list_tests;

/// Un texte ou une liste vidés : gardés vides (pour essayer le widget sans eux), et non
/// remis à la valeur de l'aperçu comme le ferait `checked_value`. Seul `null` remet la
/// valeur d'aperçu. Une liste vidée sous son `min` passe par `checked_value`, qui la refuse.
pub(crate) fn emptied(ty: &ParamType, value: &Value) -> Option<Value> {
    let blank = |v: &Value| v.as_str().is_some_and(|s| s.trim().is_empty());
    match ty {
        ParamType::List { min: 0, .. } if value.as_array().is_some_and(Vec::is_empty) => Some(Value::Array(Vec::new())),
        ParamType::Plain if blank(value) => Some(Value::String(String::new())),
        ParamType::Rich if blank(value) || value.as_array().is_some_and(|ps| ps.iter().all(paragraph_is_empty)) => {
            Some(Value::Array(Vec::new()))
        }
        _ => None,
    }
}

/// Un paragraphe bien formé (`{"runs": [...]}`) sans texte. Tout autre élément n'est pas
/// « vide » : il passe par `checked_value`, qui le refuse.
fn paragraph_is_empty(p: &Value) -> bool {
    p.get("runs").and_then(Value::as_array).is_some_and(|runs| {
        runs.iter().all(|r| r.get("t").and_then(Value::as_str).is_some_and(|t| t.trim().is_empty()))
    })
}
