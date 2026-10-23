//! Surface `host-data` : les données d'un graphique ou d'un tableau sélectionné, au dock de
//! l'agent. Lues dans le deck (paramètre `host(chart)` ou `host(table)` de l'élément) ; leurs
//! modifications passent par `data_edit`.

use oku_slides_a2ui::{Action, ComponentKind, Dynamic, SeriesSwatch};
use oku_slides_core::HostType;
use serde_json::Value;

use crate::agent::Surface;
use crate::agent::forms::data_surface;
use crate::{Result, Session};

/// Surface des données de l'élément sélectionné.
pub(crate) const HOST_DATA: &str = "host-data";
/// Événement d'une modification ; son contexte lit `/edit`.
pub(crate) const DATA_EVENT: &str = "data";
/// Groupe d'empilement que pose l'interface.
pub(crate) const STACK_GROUP: &str = "total";

/// L'élément d'un graphique ou d'un tableau et son paramètre de données.
pub(crate) struct DataTarget {
    /// Le paramètre d'hôte de l'élément.
    pub param: String,
    /// Graphique ou tableau.
    pub host: HostType,
    /// Sa valeur dans le deck.
    pub value: Value,
}

impl Session {
    /// Surface `host-data` de `element` ; `None` hors d'un deck modifiable, pour un élément
    /// absent ou sans graphique ni tableau.
    pub(crate) fn data_panel(&self, slide: &str, element: &str) -> Result<Option<Surface>> {
        if self.ensure_editable().is_err() {
            return Ok(None);
        }
        let Some(target) = self.data_target(slide, element) else { return Ok(None) };
        let (title, data) = match target.host {
            HostType::Table => ("session-data-table", table_data(&target.value)),
            _ => ("session-data-chart", chart_data(&target.value, &self.appearance.chart_palette)),
        };
        Ok(Some(data_surface(HOST_DATA, self.localizer.tr(title), data)))
    }

    /// Le paramètre graphique (sinon tableau) de `element` sur `slide`, s'il en a un.
    pub(crate) fn data_target(&self, slide: &str, element: &str) -> Option<DataTarget> {
        let deck = self.doc.sources.deck();
        let located = deck.slide(slide)?.locate(element)?;
        let schema = self.registry.get(&located.element.kind)?;
        [HostType::Chart, HostType::Table].into_iter().find_map(|host| {
            let param = schema.host_param(host)?.name.clone();
            match located.element.params.get(&param) {
                Some(oku_slides_core::ParamValue::Value(v)) => Some(DataTarget { param, host, value: v.clone() }),
                _ => None,
            }
        })
    }
}

/// L'action des composants de données : `data`, avec la modification écrite en `/edit`.
fn data_action() -> Action {
    Action::event(DATA_EVENT, [("edit", Dynamic::path("/edit"))])
}

/// `dataset.source` d'une option de graphique ou de tableau ; vide sans données.
fn source(option: &Value) -> Vec<Vec<Value>> {
    let rows = option.pointer("/dataset/source").and_then(Value::as_array);
    rows.map(|rows| rows.iter().map(|r| r.as_array().cloned().unwrap_or_default()).collect()).unwrap_or_default()
}

fn series(option: &Value) -> &[Value] {
    option.get("series").and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn kind_of(s: &Value) -> Option<&str> {
    s.get("type").and_then(Value::as_str)
}

/// Barres ou lignes : l'empilement et la couleur par colonne ont un sens.
fn cartesian(s: &Value) -> bool {
    matches!(kind_of(s), Some("bar" | "line"))
}

/// Plusieurs séries, toutes en barres ou en lignes.
pub(crate) fn stackable(option: &Value) -> bool {
    let all = series(option);
    all.len() > 1 && all.iter().all(cartesian)
}

/// Toutes les séries partagent un même groupe d'empilement.
fn stacked(option: &Value) -> bool {
    let all = series(option);
    let first = all.first().and_then(|s| s.get("stack")).and_then(Value::as_str);
    first.is_some() && all.iter().all(|s| s.get("stack").and_then(Value::as_str) == first)
}

/// Couleur `#rgb` ou `#rrggbb` ramenée à `#rrggbb` en minuscules ; `None` pour toute autre valeur.
pub(crate) fn hex_color(value: &Value) -> Option<String> {
    let s = value.as_str()?.trim();
    let digits = s.strip_prefix('#')?;
    if !matches!(digits.len(), 3 | 6) || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let lower = digits.to_ascii_lowercase();
    let full = if lower.len() == 3 { lower.chars().flat_map(|c| [c, c]).collect() } else { lower };
    Some(format!("#{full}"))
}

/// Séries qu'on peut colorer, nommées par leur en-tête : barres et lignes lisent la colonne
/// i + 1, un nuage la paire (2i, 2i + 1) ; secteurs, radar et cascade suivent la palette.
fn swatches(option: &Value, header: &[Value]) -> Vec<SeriesSwatch> {
    let all = series(option);
    let column = |i: usize| -> Option<usize> {
        if all.iter().all(cartesian) {
            Some(i + 1)
        } else if all.iter().all(|s| kind_of(s) == Some("scatter")) {
            Some(2 * i + 1)
        } else {
            None
        }
    };
    let name = |col: usize| header.get(col).map(cell_text).unwrap_or_default();
    all.iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let color = s.pointer("/itemStyle/color").and_then(hex_color);
            column(i).map(|col| SeriesSwatch { name: name(col), color })
        })
        .collect()
}

/// Le texte d'une cellule d'en-tête.
fn cell_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// `ChartData` d'une option de graphique, avec la palette du thème.
pub(crate) fn chart_data(option: &Value, palette: &[String]) -> ComponentKind {
    let source = source(option);
    let kind = series(option).first().and_then(kind_of).map(str::to_owned);
    let width = source.first().map_or(0, Vec::len);
    let checkbox_columns = if kind.as_deref() == Some("waterfall") && width > 2 { vec![2] } else { Vec::new() };
    let header = source.first().cloned().unwrap_or_default();
    ComponentKind::ChartData {
        kind,
        checkbox_columns,
        stacked: stackable(option).then(|| stacked(option)),
        series: swatches(option, &header),
        palette: palette.iter().filter_map(|c| hex_color(&Value::String(c.clone()))).collect(),
        source,
        edit: Dynamic::path("/edit"),
        action: data_action(),
    }
}

/// `TableData` d'une option de tableau ; les drapeaux absents prennent leurs défauts.
pub(crate) fn table_data(option: &Value) -> ComponentKind {
    let flag = |key: &str, default: bool| option.get(key).and_then(Value::as_bool).unwrap_or(default);
    let align = option
        .get("align")
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|v| v.as_str().unwrap_or("auto").to_owned()).collect());
    ComponentKind::TableData {
        source: source(option),
        header_row: flag("header-row", true),
        header_column: flag("header-column", false),
        striped: flag("striped", false),
        align: align.unwrap_or_default(),
        edit: Dynamic::path("/edit"),
        action: data_action(),
    }
}

#[cfg(test)]
#[path = "data_panel_tests.rs"]
mod data_panel_tests;
