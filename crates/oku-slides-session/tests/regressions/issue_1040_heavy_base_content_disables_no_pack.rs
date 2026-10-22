//! t-1040 : un deck dont le contenu dépassait le délai faisait écarter `base` et Essentiel : tout
//! devenait inconnu.
//!
//! Le pack `base` seul dans l'emplacement était accusé de l'échec borné, puis écarté : les
//! éléments devenaient des boîtes « inconnu » bon marché et le rendu réussissait. Une version
//! livrée de `base` est de confiance : l'échec est celui du deck, rendu tel quel.

use oku_slides_core::{Op, kinds};
use oku_slides_render::RenderError;
use oku_slides_session::{Request, SessionError, Settings};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{data_dir, session_with_settings};

/// Délai de rendu, bien en deçà du rendu des slides lourdes.
pub(crate) const TIMEOUT_MS: u64 = 100;

/// `slides` slides `content`, chacune un tableau `base/table` de `rows` lignes (en-tête comprise)
/// × `columns` colonnes.
pub(crate) fn heavy_table_slides(slides: usize, rows: usize, columns: usize) -> Vec<Op> {
    (0..slides)
        .map(|i| {
            let header: Vec<Value> = (0..columns).map(|c| json!(format!("Colonne {c}"))).collect();
            let body = (1..rows).map(|r| (0..columns).map(|c| json!(format!("Cellule {i}-{r}-{c}"))).collect());
            let source: Vec<Vec<Value>> = std::iter::once(header).chain(body).collect();
            serde_json::from_value(json!({ "op": "add_slide", "slide": {
                "id": format!("s-lourde-{i}"), "kind": kinds::essentiel::CONTENT, "params": { "blocks": [{
                    "id": format!("t-{i}"), "kind": kinds::base::TABLE,
                    "params": { "data": { "dataset": { "source": source } } } }] } } }))
            .unwrap_or_else(|e| panic!("{e}"))
        })
        .collect()
}

/// L'échec de rendu sous `e` est-il borné (délai, mémoire, arrêt) ?
pub(crate) fn bounded(e: &SessionError) -> Option<bool> {
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(e);
    while let Some(c) = cause {
        if let Some(r) = c.downcast_ref::<RenderError>() {
            return Some(r.kind().is_bounded());
        }
        cause = c.source();
    }
    None
}

#[test]
fn issue_1040_deck_too_heavy_with_the_builtins_alone_disables_no_pack() {
    let data = data_dir();
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("{e}"));
    settings.render.timeout_ms = TIMEOUT_MS;
    settings.render.probe_budget_ms = TIMEOUT_MS;
    let mut s = session_with_settings(data.as_ref(), settings);
    s.new_deck().unwrap_or_else(|e| panic!("{}", e.one_line()));
    // 40 slides, chacune un tableau de 100 × 20 cellules : bien plus de 100 ms.
    let err = s.apply("Lourd", heavy_table_slides(40, 100, 20), None).err();
    assert_that!(err.as_ref().and_then(bounded)).is_equal_to(Some(true));
    // Aucun pack écarté : `base` donne toujours ses widgets, et aucun avis ne nomme de pack.
    let widgets: Value = serde_json::from_str(&s.handle(Request::Widgets).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(widgets["widgets"].get(kinds::base::TEXT).is_some()).is_true();
    let snapshot: Value = serde_json::from_str(&s.handle(Request::Snapshot).unwrap_or_else(|e| panic!("{e}")))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_that!(snapshot["notice"].clone()).is_equal_to(Value::Null);
}
