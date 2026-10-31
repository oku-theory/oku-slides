//! t-969: a pack file that exports a schema dictionary by hand (without `widget()`) with a
//! mistyped field passed the probe; the registry then failed to read as a whole (`Unreadable`,
//! no widget named), so every widget of the pack was lost instead of that file alone.

use std::collections::BTreeMap;

use oku_slides_core::{Namespace, PackKind, WidgetFile};
use oku_slides_render::{FileProbe, Fonts, assemble, base_probe, probe_file};
use speculoos::prelude::*;

const QUOTE: &str = r#"#import "/oku-slides/lib.typ": widget, params as p
#let widget = widget("quote", params: (body: p.plain()), (args, ctx) => [#args.body])
"#;

/// A schema written by hand whose `params` is a number, not a list.
const HAND_MADE: &str = r#"#let widget = (schema: (name: "kpi", category: "element", params: 3))
"#;

fn wf(name: &str) -> WidgetFile {
    WidgetFile::parse(name).unwrap_or_else(|| panic!("widget file {name}"))
}

#[test]
fn issue_0969_mistyped_hand_written_schema_is_set_aside_by_the_probe() {
    let probe = probe_file(&Fonts::default(), PackKind::Widgets, &wf("kpi"), HAND_MADE, None);
    assert_that!((probe.is_valid(), probe.schema.is_none())).is_equal_to((false, true));
}

#[test]
fn issue_0969_mistyped_hand_written_schema_keeps_the_other_widgets() {
    let fonts = Fonts::default();
    let base = base_probe(&fonts).unwrap_or_else(|e| panic!("base: {e}"));
    let mut probes: BTreeMap<WidgetFile, FileProbe> = BTreeMap::from([
        (wf("quote"), probe_file(&fonts, PackKind::Widgets, &wf("quote"), QUOTE, None)),
        (wf("kpi"), probe_file(&fonts, PackKind::Widgets, &wf("kpi"), HAND_MADE, None)),
    ]);
    let acme = Namespace::parse("acme").unwrap_or_else(|| panic!("namespace"));
    let registry = assemble(&base, &acme, PackKind::Widgets, &mut probes).unwrap_or_else(|e| panic!("assemble: {e}"));
    assert_that!((registry.get("acme/quote").is_some(), registry.get("acme/kpi").is_none())).is_equal_to((true, true));
}
