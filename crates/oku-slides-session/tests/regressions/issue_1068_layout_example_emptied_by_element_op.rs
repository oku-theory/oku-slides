//! t-1068 : en mode pack, retirer le dernier élément d'un slot (ou le dernier item d'une liste)
//! sur la slide d'un layout effaçait la valeur de son exemple au lieu de la garder vide : le
//! contenu par défaut revenait (t-497).

use std::path::Path;

use oku_slides_core::{Op, WidgetFile};
use oku_slides_session::{ESSENTIEL_PACK, PackLibrary, Storage};
use serde_json::{Value, json};
use speculoos::prelude::*;

use crate::common::{data_dir, session_in};

/// La valeur `param` de l'exemple du layout `layout` d'Essentiel, dans sa version courante.
fn current_example(data: &Path, layout: &str, param: &str) -> Option<Value> {
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    let library = PackLibrary::new(Some(data.join("widgets")), Some(data.join("layouts")), storage);
    let manifest = library.current_version(ESSENTIEL_PACK)?.manifest;
    let name = WidgetFile::parse(layout)?;
    manifest.examples.get(&name)?.get(param).cloned()
}

#[test]
fn issue_1068_removing_the_last_block_of_a_layout_keeps_its_example_slot_empty() {
    let data = data_dir();
    let mut s = session_in(&data);
    s.edit_pack(ESSENTIEL_PACK).unwrap_or_else(|e| panic!("{e}"));
    let remove: Op =
        serde_json::from_value(json!({ "op": "remove_element", "slide": "layout-content", "element": "b" }))
            .unwrap_or_else(|e| panic!("{e}"));
    s.apply("Retirer", vec![remove], None).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(current_example(&data, "content", "blocks")).is_equal_to(Some(json!([])));
}
