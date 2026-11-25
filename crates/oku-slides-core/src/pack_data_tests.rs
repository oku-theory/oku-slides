use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_data::limits::MAX_BATCH_CELLS;
use oku_slides_data::{BaseId, BaseRef, Column, ColumnType, Data, DataCell, DataId, DataKind, Rows, Table};
use rstest::rstest;
use speculoos::prelude::*;
use uuid::Uuid;

use super::MAX_PACK_DATA;
use crate::{Guides, Namespace, PackFiles, PackKind, PackManifest, PackManifestErrorKind, PackScope};

/// A data of one number column whose overlay holds `cells` cells, one per row.
fn data(cells: usize, base: bool) -> Arc<Data> {
    let rows = Rows::repaired((0..cells).map(|r| format!("r{r}")));
    let overlay = (0..cells).map(|r| (format!("r{r}/c").try_into().unwrap(), DataCell::Number(1.0))).collect();
    let base = base.then(|| BaseRef {
        id: BaseId::try_from(format!("sha256:{}", "ab".repeat(32))).unwrap(),
        bytes: 0,
        rows: 0,
        columns: Vec::new(),
    });
    Arc::new(Data {
        name: "d".to_owned(),
        label: None,
        inline: false,
        kind: DataKind::Manual,
        table: Table {
            generation: "g".to_owned(),
            epoch: String::new(),
            columns: vec![Column { id: "c".to_owned(), name: "C".to_owned(), ty: ColumnType::Number }],
            rows,
            base,
            cells: overlay,
        },
    })
}

/// A widget pack manifest holding `data`, read back from its files.
fn read_with(data: BTreeMap<DataId, Arc<Data>>) -> Result<PackManifest, crate::PackManifestError> {
    let manifest = PackManifest {
        id: Uuid::new_v4(),
        name: "P".into(),
        namespace: Namespace::parse("p").unwrap(),
        requires: Vec::new(),
        description: String::new(),
        parent: None,
        created: "2026-11-21T00:00:00Z".into(),
        scope: PackScope::Library,
        profiles: Vec::new(),
        examples: BTreeMap::new(),
        styles: BTreeMap::new(),
        tokens: Vec::new(),
        order: Vec::new(),
        guides: Guides::default(),
        data,
    };
    let files: PackFiles = manifest.to_files(PackKind::Widgets, BTreeMap::new()).unwrap();
    PackManifest::from_files(&files)
}

/// `count` data sharing `cells` cells as evenly as possible, the first one with a base if asked.
fn many(count: usize, cells: usize, base: bool) -> BTreeMap<DataId, Arc<Data>> {
    (0..count)
        .map(|i| {
            let share = cells / count + usize::from(i < cells % count);
            (DataId::try_from(format!("d{i}")).unwrap(), data(share, base && i == 0))
        })
        .collect()
}

#[rstest]
#[case::count_at_bound(many(MAX_PACK_DATA, 0, false), true)]
#[case::cells_at_bound(many(2, MAX_BATCH_CELLS, false), true)]
#[case::cells_past_bound(many(2, MAX_BATCH_CELLS + 1, false), false)]
#[case::base(many(1, 0, true), false)]
fn pack_manifest_data_bounds(#[case] data: BTreeMap<DataId, Arc<Data>>, #[case] accepted: bool) {
    let read = read_with(data.clone());
    if accepted {
        assert_that!(read.map(|m| m.data)).is_ok_containing(data);
    } else {
        assert_that!(read.map_err(|e| e.kind())).is_err_containing(PackManifestErrorKind::Data);
    }
}

/// The 17th data fails while the manifest is parsed, before any later check.
#[test]
fn pack_manifest_past_max_data_fails_while_parsing() {
    let err = read_with(many(MAX_PACK_DATA + 1, 0, false)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(PackManifestErrorKind::Json);
    assert_that!(format!("{err:?}")).contains(format!("at most {MAX_PACK_DATA} entries").as_str());
}

/// An in-memory manifest (written back from a deck) is checked after the fact.
#[test]
fn check_pack_data_past_max_data_is_refused() {
    let err = super::check_pack_data(&many(MAX_PACK_DATA + 1, 0, false)).unwrap_err();
    assert_that!(err.kind()).is_equal_to(PackManifestErrorKind::Data);
}

#[test]
fn pack_manifest_data_error_localizes_in_french() {
    let err = read_with(many(1, 0, true)).unwrap_err();
    let fr = oku_slides_i18n::Localizer::new(Some("fr"));
    let text = oku_slides_i18n::Localize::localize(&err, &fr);
    assert_that!(text).contains("« d0 »");
    assert_that!(text)
        .is_not_equal_to(oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("en"))));
}

#[test]
fn pack_manifest_without_data_writes_no_data_key() {
    let files = read_with(BTreeMap::new()).unwrap().to_files(PackKind::Widgets, BTreeMap::new()).unwrap();
    assert_that!(files.manifest().contains_key("data")).is_false();
}

/// An example value written as packs did before examples named the pack's data.
#[test]
fn pack_example_with_inline_dataset_is_dropped_with_diagnostic() {
    let mut json = serde_json::to_value(read_with(BTreeMap::new()).unwrap()).unwrap();
    json["examples"] = serde_json::json!({
        "inline-table": { "data": { "dataset": { "source": [["a"], [1]] } }, "header-column": true },
        "inline-chart": { "option": { "series": [{ "type": "bar", "data": [1, 2] }] } },
    });
    let serde_json::Value::Object(json) = json else { panic!("an object") };
    let read = PackManifest::from_files(&PackFiles::new(PackKind::Widgets, json, BTreeMap::new())).unwrap();
    let kept: Vec<(String, Vec<String>)> =
        read.examples.iter().map(|(w, v)| (w.as_str().to_owned(), v.keys().cloned().collect())).collect();
    assert_that!(kept).is_equal_to(vec![("inline-table".to_owned(), vec!["header-column".to_owned()])]);
    let problem = |param: &str| format!("example value `{param}` holds inline chart or table data, ignored");
    assert_that!(crate::pack_examples::diagnosed("inline-table", &problem("data"))).is_true();
    assert_that!(crate::pack_examples::diagnosed("inline-chart", &problem("option"))).is_true();
}

/// The base shipped at `100fe5ab` still verifies and loads, its inline chart and table examples gone.
#[test]
fn older_base_fixture_loads_with_inline_examples_dropped() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/packs/base-100fe5ab");
    let manifest = std::fs::read(dir.join("widgets.json")).unwrap();
    let widgets: Vec<(crate::WidgetFile, Vec<u8>)> = std::fs::read_dir(dir.join("widgets"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|p| {
            let name = p.file_stem().unwrap().to_str().unwrap().to_owned();
            (crate::WidgetFile::parse(&name).unwrap(), std::fs::read(&p).unwrap())
        })
        .collect();
    let hex = "100fe5ab";
    let files =
        PackFiles::from_bytes(PackKind::Widgets, &manifest, widgets.iter().map(|(n, b)| (n.clone(), &b[..]))).unwrap();
    assert_that!(files.version().hex()).starts_with(hex);
    let read = PackManifest::from_files(&files).unwrap();
    let examples: Vec<&str> = read.examples.keys().map(crate::WidgetFile::as_str).collect();
    assert_that!(examples).is_equal_to(vec!["card", "text"]);
    let problem = |param: &str| format!("example value `{param}` holds inline chart or table data, ignored");
    assert_that!(crate::pack_examples::diagnosed("chart", &problem("option"))).is_true();
    assert_that!(crate::pack_examples::diagnosed("table", &problem("data"))).is_true();
}
