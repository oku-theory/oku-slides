use std::collections::BTreeMap;
use std::sync::Arc;

use oku_slides_core::{Namespace, WidgetErrorKind, WidgetFile, WidgetRegistry, qualify};
use serde_json::json;
use speculoos::prelude::*;

use super::{
    PackSet, PackWidgets, Registries, compile_probe, probe_json, probe_registry, probe_source, registry_from_probe,
};
use crate::error::RegistryErrorKind;
use crate::fonts::Fonts;
use crate::widget_probe::{FileProbe, assemble, assemble_set, probe_file};
use crate::world::OkuSlidesWorld;
use crate::{FileProbeCache, load_registry};

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
    let json = probe_registry(&Fonts::default(), &PackSet::default()).unwrap();
    let from = registry_from_probe(json).unwrap();
    assert_that!(from).is_equal_to(load_registry(&Fonts::default(), &PackSet::default()).unwrap());
}

#[test]
fn pack_fingerprint_changes_with_source() {
    let a = PackWidgets::new("p", ns("p"), files(&[("quote", QUOTE_FILE)])).unwrap();
    let b = PackWidgets::new("p", ns("p"), files(&[("quote", &format!("{QUOTE_FILE} "))])).unwrap();
    assert_that!(a.fingerprint()).is_not_equal_to(b.fingerprint());
}

#[test]
fn pack_fingerprint_changes_with_namespace() {
    let a = PackWidgets::new("p", ns("acme"), files(&[("quote", QUOTE_FILE)])).unwrap();
    let b = PackWidgets::new("p", ns("beta"), files(&[("quote", QUOTE_FILE)])).unwrap();
    assert_that!(a.fingerprint()).is_not_equal_to(b.fingerprint());
}

#[test]
fn registries_get_is_memoized_by_fingerprint() {
    let mut r = Registries::new(Fonts::default());
    let set = PackSet::new(vec![PackWidgets::new("p", ns("p"), files(&[("quote", QUOTE_FILE)])).unwrap()]).unwrap();
    let first = r.get(&set).unwrap();
    let second = r.get(&set).unwrap();
    assert_that!(Arc::ptr_eq(&first, &second)).is_true();
}

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap()
}

fn ns(name: &str) -> Namespace {
    Namespace::parse(name).unwrap()
}

/// Le pack `id`, d'espace de noms `namespace`, à un seul widget `card`.
fn pack(id: &str, namespace: &str, widget: &str) -> PackWidgets {
    PackWidgets::new(id, ns(namespace), files(&[(widget, CARD_SRC)])).unwrap()
}

fn fonts() -> Fonts {
    Fonts::default()
}

fn registries() -> Registries {
    Registries::new(Fonts::default())
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

/// Un widget de pack `card`, homonyme de celui du noyau.
const CARD_SRC: &str = r#"#import "/oku-slides/lib.typ": widget, field, params as p
#let widget = widget("card", params: (title: p.plain()), (args, ctx) => field(ctx, "title", args.title))
"#;

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
    world.set_main_text(&probe_source([(&ns("p"), "/packs/main/widgets.typ".to_owned())]));
    registry_from_probe(compile_probe(&world).unwrap()).unwrap()
}

#[test]
fn load_registry_of_files_equals_the_hand_written_widgets_typ() {
    let pack = PackWidgets::new("p", ns("p"), files(&[("quote", QUOTE_FILE), ("kpi", KPI_FILE), ("_commun", COMMON)]))
        .unwrap();
    let assembled = load_registry(&Fonts::default(), &PackSet::new(vec![pack]).unwrap()).unwrap();
    assert_that!(assembled).is_equal_to(hand_written_registry(HAND_WRITTEN));
    assert_that!(assembled.get("p/quote")).is_some();
    assert_that!(assembled.get("p/_commun")).is_none();
}

#[test]
fn pack_assembly_imports_each_widget_but_not_common() {
    let pack = PackWidgets::new("p", ns("p"), files(&[("quote", QUOTE_FILE), ("kpi", KPI_FILE), ("_commun", COMMON)]))
        .unwrap();
    assert_that!(pack.assembly()).is_equal_to(
        "#import \"widgets/kpi.typ\": widget as w0\n#import \"widgets/quote.typ\": widget as w1\n\
         #let registry = (w0,w1,).map(w => w + (name: \"p/\" + w.name))\n"
            .to_owned(),
    );
    let empty = PackWidgets::new("p", ns("p"), files(&[("_commun", COMMON)])).unwrap();
    assert_that!(empty.assembly()).is_equal_to("#let registry = ().map(w => w + (name: \"p/\" + w.name))\n".to_owned());
}

#[test]
fn pack_set_assembly_prefixes_names_with_the_namespace() {
    let pack = PackWidgets::new("abc", ns("acme"), files(&[("card", CARD_SRC)])).unwrap();
    assert_that!(pack.assembly()).contains("\"acme/\" + w.name");
}

#[test]
fn pack_assembly_prefix_equals_the_qualify_prefix() {
    let namespace = ns("acme.studio");
    let prefix = qualify(&namespace, "x").trim_end_matches('x').to_owned();
    let pack = PackWidgets::new("abc", namespace, files(&[("card", CARD_SRC)])).unwrap();
    assert_that!(pack.assembly()).contains(format!("(name: \"{prefix}\" + w.name)").as_str());
}

#[test]
fn load_registry_two_packs_registers_both_namespaces() {
    let set = PackSet::new(vec![pack("a1", "acme", "card"), pack("b1", "beta", "card")]).unwrap();
    let r = load_registry(&fonts(), &set).unwrap();
    assert_that!((r.get("acme/card").is_some(), r.get("beta/card").is_some())).is_equal_to((true, true));
}

#[test]
fn load_registry_pack_widget_named_like_a_base_widget_keeps_the_base_one() {
    let set = PackSet::new(vec![pack("a1", "acme", "card")]).unwrap();
    let r = load_registry(&fonts(), &set).unwrap();
    assert_that!(r.get("card")).is_equal_to(crate::base_registry(&fonts()).unwrap().get("card"));
    assert_that!(r.get("acme/card").map(|w| w.params.len())).is_equal_to(Some(1));
}

#[test]
fn pack_set_fingerprint_is_order_independent() {
    let a = PackSet::new(vec![pack("a1", "acme", "card"), pack("b1", "beta", "card")]).unwrap();
    let b = PackSet::new(vec![pack("b1", "beta", "card"), pack("a1", "acme", "card")]).unwrap();
    assert_that!(a.fingerprint()).is_equal_to(b.fingerprint());
}

#[test]
fn pack_set_new_sorts_packs_by_namespace() {
    let set = PackSet::new(vec![pack("b1", "beta", "card"), pack("a1", "acme", "card")]).unwrap();
    let names: Vec<&str> = set.packs().iter().map(|p| p.namespace().as_str()).collect();
    assert_that!(names).is_equal_to(vec!["acme", "beta"]);
}

#[rstest::rstest]
#[case::same_namespace(pack("a1", "acme", "card"), pack("a2", "acme", "card"))]
#[case::same_id(pack("a1", "acme", "card"), pack("a1", "beta", "card"))]
fn pack_set_new_two_packs_sharing_a_namespace_or_an_id_returns_duplicate_pack(
    #[case] first: PackWidgets,
    #[case] second: PackWidgets,
) {
    let err = PackSet::new(vec![first, second]).unwrap_err();
    assert_that!(err.kind()).is_equal_to(crate::RegistryErrorKind::DuplicatePack);
    let fr = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("fr")));
    let en = oku_slides_i18n::Localize::localize(&err, &oku_slides_i18n::Localizer::new(Some("en")));
    assert_that!(fr).is_not_equal_to(en);
}

#[test]
fn registries_get_empty_set_is_the_base_registry() {
    let base = registries().get(&PackSet::default()).unwrap();
    assert_that!(Arc::ptr_eq(&base, &crate::base_registry(&fonts()).unwrap())).is_true();
}

#[test]
fn probe_file_importing_another_pack_is_broken() {
    let src = "#import \"/packs/0123abcd/widgets/card.typ\": widget as other\n#let widget = other\n";
    let probe = registries().probe_file(&WidgetFile::parse("thief").unwrap(), src, None);
    assert_that!(probe.is_valid()).is_false();
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
    let registry = assemble(&base, &ns("acme"), &mut probes).unwrap();
    assert_that!(registry.get("acme/quote")).is_some();
    assert_that!(registry.get("acme/casse")).is_none();
    assert_that!(registry.get("card")).is_some();
}

#[test]
fn assemble_sets_aside_a_widget_whose_slot_accepts_an_unknown_kind() {
    let fonts = Fonts::default();
    let base = crate::registry::base_probe(&fonts).unwrap();
    let holder = "#import \"/oku-slides/lib.typ\": widget, params as p\n\
        #let widget = widget(\"holder\", params: (kids: p.slot(accepts: (\"acme/nope\",))), (args, ctx) => [])\n";
    let mut probes = BTreeMap::from([
        (wf("holder"), probe_file(&fonts, &wf("holder"), holder, None)),
        (wf("quote"), probe_file(&fonts, &wf("quote"), QUOTE_FILE, None)),
    ]);
    assert_that!(probes[&wf("holder")].is_valid()).is_true();
    let registry = assemble(&base, &ns("acme"), &mut probes).unwrap();
    assert_that!(probes[&wf("holder")].is_valid()).is_false();
    assert_that!((registry.get("acme/holder").is_none(), registry.get("acme/quote").is_some()))
        .is_equal_to((true, true));
}

#[test]
fn assemble_set_registers_every_namespace_and_sets_aside_the_broken_widget_of_its_pack() {
    let fonts = Fonts::default();
    let base = crate::registry::base_probe(&fonts).unwrap();
    let mut acme = BTreeMap::from([
        (wf("quote"), probe_file(&fonts, &wf("quote"), QUOTE_FILE, None)),
        (wf("casse"), probe_file(&fonts, &wf("casse"), "#let widget = (", None)),
    ]);
    let mut beta = BTreeMap::from([(wf("quote"), probe_file(&fonts, &wf("quote"), QUOTE_FILE, None))]);
    let (a, b) = (ns("acme"), ns("beta"));
    let registry = assemble_set(&base, &mut [(&a, &mut acme), (&b, &mut beta)]).unwrap();
    assert_that!((registry.get("acme/quote").is_some(), registry.get("beta/quote").is_some()))
        .is_equal_to((true, true));
    assert_that!(registry.get("acme/casse")).is_none();
    assert_that!(registry.get("card")).is_some();
}

#[test]
fn assemble_set_sets_aside_the_culprit_in_the_pack_that_holds_it() {
    let fonts = Fonts::default();
    let base = crate::registry::base_probe(&fonts).unwrap();
    let holder = "#import \"/oku-slides/lib.typ\": widget, params as p\n\
        #let widget = widget(\"holder\", params: (kids: p.slot(accepts: (\"acme/nope\",))), (args, ctx) => [])\n";
    let mut acme = BTreeMap::from([(wf("holder"), probe_file(&fonts, &wf("holder"), holder, None))]);
    let mut beta = BTreeMap::from([(wf("quote"), probe_file(&fonts, &wf("quote"), QUOTE_FILE, None))]);
    let (a, b) = (ns("acme"), ns("beta"));
    let registry = assemble_set(&base, &mut [(&a, &mut acme), (&b, &mut beta)]).unwrap();
    assert_that!((acme[&wf("holder")].is_valid(), beta[&wf("quote")].is_valid())).is_equal_to((false, true));
    assert_that!(registry.get("beta/quote")).is_some();
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
    // `_commun` modifié : tous les widgets sont recompilés.
    let common = "#let cadre(body) = block(inset: 4pt, body)\n";
    r.probe_file(&wf("quote"), &changed, Some(common));
    assert_that!(r.file_compiles).is_equal_to(4);
    r.probe_file(&wf("kpi"), KPI_FILE, Some(common));
    assert_that!(r.file_compiles).is_equal_to(5);
}

#[test]
fn registries_probe_file_sees_a_common_imported_through_an_expression() {
    // L'import de `_commun` n'est pas écrit en toutes lettres.
    let kpi = KPI_FILE.replace("\"_commun.typ\"", "(\"_com\" + \"mun.typ\")");
    let mut r = Registries::new(Fonts::default());
    assert_that!(r.probe_file(&wf("kpi"), &kpi, Some(COMMON)).is_valid()).is_true();
    let without_cadre = "#let autre(body) = body\n";
    assert_that!(r.probe_file(&wf("kpi"), &kpi, Some(without_cadre)).is_valid()).is_false();
}

#[test]
fn pack_fingerprint_changes_with_any_file() {
    let a = PackWidgets::new("p", ns("p"), files(&[("kpi", KPI_FILE), ("_commun", COMMON)])).unwrap();
    let b =
        PackWidgets::new("p", ns("p"), files(&[("kpi", KPI_FILE), ("_commun", "#let cadre(body) = body\n")])).unwrap();
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

#[test]
fn registries_keep_a_bounded_number_of_probes_per_file() {
    let mut r = Registries::new(Fonts::default());
    for i in 0..FileProbeCache::PER_FILE + 2 {
        r.probe_file(&wf("quote"), &format!("{QUOTE_FILE}// {i}\n"), None);
    }
    r.probe_file(&wf("kpi"), KPI_FILE, Some(COMMON));
    assert_that!(r.files.len()).is_equal_to(FileProbeCache::PER_FILE + 1);
    // Les plus récentes restent servies sans recompiler.
    let compiles = r.file_compiles;
    r.probe_file(&wf("quote"), &format!("{QUOTE_FILE}// {}\n", FileProbeCache::PER_FILE + 1), None);
    r.probe_file(&wf("quote"), &format!("{QUOTE_FILE}// 2\n"), None);
    assert_that!(r.file_compiles).is_equal_to(compiles);
    // La plus ancienne a été évincée.
    r.probe_file(&wf("quote"), &format!("{QUOTE_FILE}// 0\n"), None);
    assert_that!(r.file_compiles).is_equal_to(compiles + 1);
}

/// L'ensemble du pack `i` : un seul widget, dont la source porte `i`.
fn pack_n(i: usize) -> PackSet {
    PackSet::new(vec![PackWidgets::new("p", ns("p"), files(&[("quote", &format!("{QUOTE_FILE}// {i}\n"))])).unwrap()])
        .unwrap()
}

#[test]
fn registries_keep_a_bounded_number_of_pack_registries() {
    let mut r = Registries::new(Fonts::default());
    for i in 0..super::MAX_CACHED_PACKS + 3 {
        r.get(&pack_n(i)).unwrap();
    }
    assert_that!(r.cache.len()).is_equal_to(super::MAX_CACHED_PACKS);
    // Le plus récent reste servi.
    let last = pack_n(super::MAX_CACHED_PACKS + 2);
    let first = r.get(&last).unwrap();
    assert_that!(Arc::ptr_eq(&first, &r.get(&last).unwrap())).is_true();
}

#[test]
fn registries_evict_the_least_recently_served_pack() {
    let mut r = Registries::new(Fonts::default());
    let first = r.get(&pack_n(0)).unwrap();
    for i in 1..super::MAX_CACHED_PACKS {
        r.get(&pack_n(i)).unwrap();
    }
    // Le premier, servi de nouveau, devient le plus récent : le second est évincé à sa place.
    assert_that!(Arc::ptr_eq(&first, &r.get(&pack_n(0)).unwrap())).is_true();
    r.get(&pack_n(super::MAX_CACHED_PACKS)).unwrap();
    assert_that!(r.cache.contains_key(&pack_n(0).fingerprint())).is_true();
    assert_that!(r.cache.contains_key(&pack_n(1).fingerprint())).is_false();
    assert_that!(r.cache.len()).is_equal_to(super::MAX_CACHED_PACKS);
}

#[test]
fn registries_alternating_two_versions_of_a_file_does_not_recompile() {
    let mut r = Registries::new(Fonts::default());
    let (a, b) = (format!("{QUOTE_FILE}// a\n"), format!("{QUOTE_FILE}// b\n"));
    r.probe_file(&wf("quote"), &a, None);
    r.probe_file(&wf("quote"), &b, None);
    assert_that!(r.file_compiles).is_equal_to(2);
    for _ in 0..3 {
        r.probe_file(&wf("quote"), &a, None);
        r.probe_file(&wf("quote"), &b, None);
    }
    assert_that!(r.file_compiles).is_equal_to(2);
}
