//! Toutes les sortes d'items, et le même deck réécrit par LibreOffice.

mod common;

use oku_slides_import::read_pptx;
use oku_slides_import::source::{Item, ItemKind, Role, SourceDeck};
use speculoos::prelude::*;

fn kind(i: &Item) -> String {
    match &i.kind {
        ItemKind::Text(_) => "text".into(),
        ItemKind::Shape(_) => "shape".into(),
        ItemKind::Picture(_) => "picture".into(),
        ItemKind::Table(_) => "table".into(),
        ItemKind::Chart(_) => "chart".into(),
        ItemKind::Group { items, .. } => format!("group[{}]", items.iter().map(kind).collect::<Vec<_>>().join(",")),
        ItemKind::SmartArt(sa) => format!("smartart[{}]", sa.nodes.len()),
        ItemKind::Opaque(o) => format!("opaque({:?})", o.reason),
    }
}

/// Une ligne par slide (sortes d'items) et par avertissement.
fn summary(d: &SourceDeck) -> Vec<String> {
    let mut out: Vec<String> = d
        .slides
        .iter()
        .map(|s| format!("{}: {}", s.id, s.items.iter().map(kind).collect::<Vec<_>>().join(" ")))
        .collect();
    out.extend(d.warnings.iter().map(|w| format!("warning: {:?}", w.kind)));
    out
}

fn titles(d: &SourceDeck) -> Vec<String> {
    d.slides
        .iter()
        .map(|s| {
            s.items
                .iter()
                .find(|i| i.role == Some(Role::Title))
                .and_then(|i| match &i.kind {
                    ItemKind::Text(tb) => {
                        tb.paragraphs.first().map(|p| p.runs.iter().map(|r| r.text.as_str()).collect())
                    }
                    _ => None,
                })
                .unwrap_or_default()
        })
        .collect()
}

fn kitchen() -> SourceDeck {
    read_pptx(&common::pptx("kitchen"), &common::limits()).unwrap_or_else(|e| panic!("lecture de kitchen : {e}"))
}

#[test]
fn kitchen_reads_every_kind_of_item() {
    assert_that!(summary(&kitchen())).is_equal_to(vec![
        "s1: text table".to_owned(),
        "s2: text chart picture group[text,text]".to_owned(),
        "s3: text group[shape,shape] opaque(Math)".to_owned(),
        r#"warning: UnsupportedChart("combo: barChart")"#.to_owned(),
    ]);
}

#[test]
fn kitchen_titles_are_read() {
    assert_that!(titles(&kitchen())).is_equal_to(vec![
        "Tableau".to_owned(),
        "Graphique et image".to_owned(),
        "Processus et équation".to_owned(),
    ]);
}

#[test]
fn kitchen_snapshot_matches() {
    insta::assert_json_snapshot!("kitchen", kitchen());
}

#[test]
fn libreoffice_round_trip_reads_the_same_deck() {
    let lo = read_pptx(&common::file("libreoffice.pptx"), &common::limits())
        .unwrap_or_else(|e| panic!("lecture de libreoffice.pptx : {e}"));
    assert_that!(lo.warnings.is_empty()).is_true();
    assert_that!(titles(&lo)).is_equal_to(titles(&kitchen()));
    let kinds: Vec<String> = lo.slides.iter().flat_map(|s| s.items.iter().map(kind)).collect();
    assert_that!(kinds.iter().any(|k| k == "table") && kinds.iter().any(|k| k == "chart")).is_true();
    insta::assert_json_snapshot!("libreoffice_summary", summary(&lo));
}
