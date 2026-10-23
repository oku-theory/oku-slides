//! t-1112 : le décompte des mises à jour de packs relisait la bibliothèque à chaque annulation.
//!
//! La lecture de la bibliothèque est bloquante (OpenDAL) : appelée depuis un fil qui fait
//! tourner un runtime Tokio (l'agent, ses tests), elle paniquait (« Cannot start a runtime
//! from within a runtime »). Une annulation qui ne change aucun épinglage ne relit plus rien :
//! le décompte ne dépend que des épinglages et de la bibliothèque.

use oku_slides_core::Op;
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{data_dir, fixture_path, session_in};

#[test]
fn issue_1112_undo_without_pin_change_inside_a_runtime_reads_no_library() {
    let data = data_dir();
    let mut s = session_in(&data);
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("ouverture : {}", e.one_line()));
    let op = Op::SetToken { path: vec!["color".into(), "accent".into()], value: json!("#204060") };
    s.apply("accent", vec![op], None).unwrap_or_else(|e| panic!("jeton : {}", e.one_line()));
    let runtime =
        tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap_or_else(|e| panic!("runtime : {e}"));
    let undone = runtime.block_on(async { s.undo().map_err(|e| e.kind()) });
    assert_that!(undone).is_equal_to(Ok(()));
    assert_that!(s.snapshot().can_undo).is_false();
}
