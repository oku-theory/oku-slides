use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{WidgetErrorKind, WidgetFile, WidgetRegistry};
use serde_json::json;
use speculoos::prelude::*;

use super::{PackWidgets, Registries, compile_probe, probe_json, probe_registry, probe_source, registry_from_probe};
use crate::base_registry;
use crate::error::RegistryErrorKind;
use crate::fonts::Fonts;
use crate::load_registry;
use crate::widget_probe::{FileProbe, assemble, probe_file};
use crate::world::OkuSlidesWorld;

#[test]
fn probe_json_no_metadata_returns_missing() {
    let err = probe_json(None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Missing);
}

#[test]
fn probe_json_valid_json_returns_it() {
    let value = probe_json(Some(Ok(json!({ "base": [] })))).unwrap();
    assert_that!(value).is_equal_to(json!({ "base": [] }));
}

#[test]
fn probe_json_conversion_failure_returns_schema_unreadable() {
    let source = serde_json::from_str::<i32>("nope").unwrap_err();
    let err = probe_json(Some(Err(source))).unwrap_err();
    assert_that!(err.kind()).is_equal_to(RegistryErrorKind::Schema);
    assert_that!(err.widget_error().map(|e| e.kind())).is_equal_to(Some(WidgetErrorKind::Unreadable));
}

#[test]
fn registry_from_probe_of_base_probe_equals_load_registry() {
    let json = probe_registry(&Fonts::default(), None).unwrap();
    let from = registry_from_probe(json).unwrap();
    assert_that!(from).is_equal_to(load_registry(&Fonts::default(), None).unwrap());
}

#[test]
fn pack_fingerprint_changes_with_source() {
    let a = PackWidgets::new("p", files(&[("quote", QUOTE_FILE)])).unwrap();
    let b = PackWidgets::new("p", files(&[("quote", &format!("{QUOTE_FILE} "))])).unwrap();
    assert_that!(a.fingerprint()).is_not_equal_to(b.fingerprint());
}

#[test]
fn registries_probe_is_memoized_by_fingerprint() {
    let mut r = Registries::new(Fonts::default());
    let pack = PackWidgets::new("p", files(&[("quote", QUOTE_FILE)])).unwrap();
    let first = r.probe(&pack).unwrap();
    let second = r.probe(&pack).unwrap();
    assert_that!(Arc::ptr_eq(&first, &second)).is_true();
}

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap()
}

fn files(entries: &[(&str, &str)]) -> BTreeMap<WidgetFile, String> {
    entries.iter().map(|(n, s)| (wf(n), (*s).to_owned())).collect()
}

const QUOTE_FILE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
"#;

const KPI_FILE: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#import "_commun.typ": cadre
#let widget = widget("kpi", params: (value: p.plain()), (args, ctx) => cadre(field(ctx, "value", args.value)))
"#;

const COMMON: &str = "#let cadre(body) = block(inset: 8pt, body)\n";

/// Le même pack, en un seul `widgets.typ` écrit à la main.
const HAND_WRITTEN: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let cadre(body) = block(inset: 8pt, body)
#let quote = widget("quote", params: (body: p.rich(), source: p.plain()),
  (args, ctx) => field(ctx, "body", emph(args.body)))
#let kpi = widget("kpi", params: (value: p.plain()), (args, ctx) => cadre(field(ctx, "value", args.value)))
#let registry = (kpi, quote)
"#;

/// Le registre d'un `widgets.typ` écrit à la main, servi sous `/packs/main/`.
fn hand_written_registry(source: &str) -> WidgetRegistry {
    let mut world = OkuSlidesWorld::new(Fonts::default()).unwrap();
    world.add_source("/packs/main/widgets.typ", source.to_owned()).unwrap();
    world.set_main_text(&probe_source(Some("/packs/main/widgets.typ")));
    registry_from_probe(compile_probe(&world).unwrap()).unwrap()
}

#[test]
fn load_registry_of_files_equals_the_hand_written_widgets_typ() {
    let pack = PackWidgets::new("p", files(&[("quote", QUOTE_FILE), ("kpi", KPI_FILE), ("_commun", COMMON)])).unwrap();
    let assembled = load_registry(&Fonts::default(), Some(&pack)).unwrap();
    assert_that!(assembled).is_equal_to(hand_written_registry(HAND_WRITTEN));
    assert_that!(assembled.get("_commun")).is_none();
}

#[test]
fn pack_assembly_imports_each_widget_but_not_common() {
    let pack = PackWidgets::new("p", files(&[("quote", QUOTE_FILE), ("kpi", KPI_FILE), ("_commun", COMMON)])).unwrap();
    assert_that!(pack.assembly()).is_equal_to(
        "#import \"widgets/kpi.typ\": widget as w0\n#import \"widgets/quote.typ\": widget as w1\n#let registry = (w0,w1,)\n"
            .to_owned(),
    );
    let empty = PackWidgets::new("p", files(&[("_commun", COMMON)])).unwrap();
    assert_that!(empty.assembly()).is_equal_to("#let registry = ()\n".to_owned());
}

#[test]
fn probe_file_of_a_broken_widget_returns_diagnostics_at_line_and_column() {
    let broken = "// Cassé\n#let widget = (\n";
    let probe = probe_file(&Fonts::default(), &wf("casse"), broken, None);
    assert_that!(probe.is_valid()).is_false();
    let d = probe.diagnostics.iter().find(|d| d.widget_file() == Some(wf("casse"))).unwrap();
    assert_that!(d.at.as_ref().map(|a| a.line)).is_equal_to(Some(2));
    assert_that!(d.at.as_ref().is_some_and(|a| a.column >= 1)).is_true();
}

#[test]
fn probe_file_of_common_is_valid_without_schema() {
    let probe = probe_file(&Fonts::default(), &WidgetFile::common(), COMMON, None);
    assert_that!(probe.is_valid()).is_true();
    assert_that!(probe.schema).is_none();
}

#[test]
fn probe_file_importing_common_reads_it() {
    let probe = probe_file(&Fonts::default(), &wf("kpi"), KPI_FILE, Some(COMMON));
    assert_that!(probe.is_valid()).is_true();
    assert_that!(probe.schema.as_ref().and_then(|s| s["name"].as_str())).is_equal_to(Some("kpi"));
    let without = probe_file(&Fonts::default(), &wf("kpi"), KPI_FILE, None);
    assert_that!(without.is_valid()).is_false();
}

#[test]
fn probe_file_whose_declared_name_differs_from_its_file_is_refused() {
    let src = "#import \"/oku-slides/lib.typ\": widget\n#let widget = widget(\"autre\", (args, ctx) => [])\n";
    let probe = probe_file(&Fonts::default(), &wf("kpi"), src, None);
    assert_that!(probe.is_valid()).is_false();
    assert_that!(probe.diagnostics[0].message.as_str()).contains("autre");
    assert_that!(probe.diagnostics[0].at.as_ref().map(|a| a.line)).is_equal_to(Some(2));
}

#[test]
fn assemble_keeps_the_valid_widgets_and_sets_aside_the_broken_one() {
    let fonts = Fonts::default();
    let base = crate::registry::base_probe(&fonts).unwrap();
    let mut probes: BTreeMap<WidgetFile, FileProbe> = BTreeMap::from([
        (wf("quote"), probe_file(&fonts, &wf("quote"), QUOTE_FILE, None)),
        (wf("casse"), probe_file(&fonts, &wf("casse"), "#let widget = (", None)),
    ]);
    let registry = assemble(&base, &mut probes).unwrap();
    assert_that!(registry.get("quote")).is_some();
    assert_that!(registry.get("casse")).is_none();
    assert_that!(registry.get("card")).is_some();
}

#[test]
fn assemble_sets_aside_an_incompatible_override_with_a_diagnostic() {
    let fonts = Fonts::default();
    let base = crate::registry::base_probe(&fonts).unwrap();
    let card = "#import \"/oku-slides/lib.typ\": widget, params as p\n#let widget = widget(\"card\", params: (title: p.plain()), (args, ctx) => [])\n";
    let mut probes = BTreeMap::from([(wf("card"), probe_file(&fonts, &wf("card"), card, None))]);
    assert_that!(probes[&wf("card")].is_valid()).is_true();
    let registry = assemble(&base, &mut probes).unwrap();
    assert_that!(probes[&wf("card")].is_valid()).is_false();
    assert_that!(registry.get("card")).is_equal_to(base_registry(&fonts).unwrap().get("card"));
}

#[test]
fn registries_probe_file_recompiles_only_what_changed() {
    let mut r = Registries::new(Fonts::default());
    r.probe_file(&wf("quote"), QUOTE_FILE, Some(COMMON));
    r.probe_file(&wf("kpi"), KPI_FILE, Some(COMMON));
    assert_that!(r.file_compiles).is_equal_to(2);
    // Un widget modifié : lui seul est recompilé.
    let changed = QUOTE_FILE.replace("source: p.plain()", "source: p.plain(), note: p.plain()");
    r.probe_file(&wf("quote"), &changed, Some(COMMON));
    r.probe_file(&wf("kpi"), KPI_FILE, Some(COMMON));
    assert_that!(r.file_compiles).is_equal_to(3);
    // `_commun` modifié : seuls ceux qui l'importent sont recompilés.
    let common = "#let cadre(body) = block(inset: 4pt, body)\n";
    r.probe_file(&wf("quote"), &changed, Some(common));
    assert_that!(r.file_compiles).is_equal_to(3);
    r.probe_file(&wf("kpi"), KPI_FILE, Some(common));
    assert_that!(r.file_compiles).is_equal_to(4);
}

#[test]
fn pack_fingerprint_changes_with_any_file() {
    let a = PackWidgets::new("p", files(&[("kpi", KPI_FILE), ("_commun", COMMON)])).unwrap();
    let b = PackWidgets::new("p", files(&[("kpi", KPI_FILE), ("_commun", "#let cadre(body) = body\n")])).unwrap();
    assert_that!(a.fingerprint()).is_not_equal_to(b.fingerprint());
}

#[test]
fn probe_file_without_a_widget_export_attaches_its_errors_to_the_file() {
    for src in ["#let autre = 1\n", "#let widget = 1\n"] {
        let probe = probe_file(&Fonts::default(), &wf("kpi"), src, None);
        asserting(src).that(&probe.is_valid()).is_false();
        let errors: Vec<_> = probe.diagnostics.iter().filter(|d| d.severity == crate::Severity::Error).collect();
        asserting(src).that(&errors.is_empty()).is_false();
        for d in errors {
            asserting(src).that(&d.widget_file()).is_equal_to(Some(wf("kpi")));
            asserting(src).that(&d.place.as_deref().is_some_and(|p| p.contains("main.typ"))).is_false();
        }
    }
}

#[test]
fn probe_file_error_raised_in_the_library_points_at_the_call_in_the_file() {
    let src = "#import \"/oku-slides/lib.typ\": widget\n\n#let widget = widget(\"kpi\", category: \"autre\", (args, ctx) => [])\n";
    let probe = probe_file(&Fonts::default(), &wf("kpi"), src, None);
    assert_that!(probe.is_valid()).is_false();
    let d = probe.diagnostics.iter().find(|d| d.severity == crate::Severity::Error).unwrap();
    assert_that!(d.widget_file()).is_equal_to(Some(wf("kpi")));
    assert_that!(d.at.as_ref().map(|a| a.line)).is_equal_to(Some(3));
}
