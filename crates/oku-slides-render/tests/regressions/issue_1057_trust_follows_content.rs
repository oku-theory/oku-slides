//! t-1057 point 2 : la confiance d'un pack suit son contenu, non un id choisi par l'appelant.

use oku_slides_core::{PackFiles, WidgetFile};
use oku_slides_render::{BuiltinPack, PackWidgets, is_trusted_pack};
use speculoos::prelude::*;

/// Les fichiers livrés de `base`.
fn shipped() -> PackFiles {
    BuiltinPack::Base.files().unwrap_or_else(|e| panic!("base : {e}"))
}

#[test]
fn issue_1057_shipped_base_files_are_trusted() {
    let files = shipped();
    let pack = PackWidgets::from_files(BuiltinPack::Base.namespace(), &files).unwrap_or_else(|e| panic!("{e}"));
    let version = files.version();
    assert_that!(pack.id()).is_equal_to(version.hex());
    assert_that!(is_trusted_pack(&pack)).is_true();
}

#[test]
fn issue_1057_base_with_one_widget_source_changed_is_not_trusted() {
    let files = shipped();
    let mut widgets = files.widgets().clone();
    let name = widgets.keys().find(|n| **n != WidgetFile::common()).cloned().unwrap_or_else(|| panic!("widget"));
    if let Some(source) = widgets.get_mut(&name) {
        source.push_str("\n// modifié\n");
    }
    let altered = PackFiles::new(files.kind(), files.manifest().clone(), widgets);
    let pack = PackWidgets::from_files(BuiltinPack::Base.namespace(), &altered).unwrap_or_else(|e| panic!("{e}"));
    assert_that!(is_trusted_pack(&pack)).is_false();
}
