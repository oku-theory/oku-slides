//! Grille d'une slide du deck de travail en mode pack : colonnes et lignes dans sa carte de cible.
//!
//! Un paramètre `grid` ne se règle pas comme les autres (`param_fields::settable`) : en mode
//! pack, la carte de la slide d'un layout qui en déclare un offre deux steppers, adressés par
//! les clés `grid/columns` et `grid/rows` (jamais un nom de paramètre, qui n'a pas de `/`), et
//! appliqués par `set_grid`, un pas d'annulation chacun. Le deck ne l'offre pas. Testé avec la
//! carte de cible (`target_params_tests.rs`).

use oku_slides_core::{GRID_TRACKS, Grid, Op};
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};
use snafu::OptionExt;

use crate::agent::{FieldKind, ParamField};
use crate::error::HostKeySnafu;
use crate::views::SlideView;
use crate::{Result, Session};

/// Préfixe des clés des champs de grille de la carte de cible.
pub(crate) const GRID_KEY: &str = "grid/";

/// Les champs colonnes et lignes de la grille `grid` de la slide `slide`, avec leurs clés.
pub(crate) fn grid_fields(slide: &str, grid: Grid, l: &Localizer) -> Vec<(String, ParamField)> {
    [("columns", "session-panel-columns", grid.columns), ("rows", "session-panel-rows", grid.rows)]
        .into_iter()
        .map(|(axis, label, value)| {
            let field = ParamField {
                label: l.tr(label),
                kind: FieldKind::Stepper,
                value: json!(value),
                op_template: json!({ "op": "set_grid", "slide": slide, axis: "$value" }),
                min: Some(f64::from(*GRID_TRACKS.start())),
                max: Some(f64::from(*GRID_TRACKS.end())),
                step: None,
                options: Vec::new(),
                columns: Vec::new(),
                item_label: None,
            };
            (format!("{GRID_KEY}{axis}"), field)
        })
        .collect()
}

impl Session {
    /// La grille réglable de la slide de `view` : en mode pack, si son widget déclare `grid`,
    /// que la vue a une zone et que la slide n'a pas de blocs (une grille les cacherait).
    pub(crate) fn settable_grid(&self, view: &SlideView<'_>) -> Option<Grid> {
        let s = view.slide;
        let has_blocks = s.params.slot("blocks").is_some_and(|b| !b.is_empty());
        let declares_grid = self.registry.get(&s.kind).and_then(|w| w.param("grid")).is_some();
        (self.in_pack_mode() && view.zone.is_some() && declares_grid && !has_blocks)
            .then(|| s.resolved_grid(&self.registry))
    }

    /// Applique le champ de grille `key` (`grid/columns` ou `grid/rows`) de la slide `slide` : un pas d'annulation.
    ///
    /// `HostKey` si la slide n'offre pas de grille (deck, élément, pas de paramètre `grid`, blocs),
    /// pour un autre axe ou une valeur qui n'est pas un entier ; une grille hors bornes est refusée par `set_grid`.
    pub(crate) fn target_grid(&mut self, slide: &str, element: Option<&str>, key: &str, value: &Value) -> Result<()> {
        self.ensure_editable()?;
        let axis = key.strip_prefix(GRID_KEY).context(HostKeySnafu { key })?;
        let grid = match element {
            None => self.settable_grid(&self.slide(slide)?),
            Some(_) => None,
        };
        let grid = grid.context(HostKeySnafu { key })?;
        let tracks = value.as_u64().and_then(|v| u32::try_from(v).ok()).context(HostKeySnafu { key })?;
        let (columns, rows) = match axis {
            "columns" => (tracks, grid.rows),
            "rows" => (grid.columns, tracks),
            _ => return HostKeySnafu { key }.fail()?,
        };
        let op = Op::SetGrid {
            slide: slide.to_owned(),
            columns: (axis == "columns").then_some(columns),
            rows: (axis == "rows").then_some(rows),
        };
        let label = self.localizer.tr_with("session-undo-grid", &[("columns", columns.into()), ("rows", rows.into())]);
        self.apply(&label, vec![op], None)
    }
}
