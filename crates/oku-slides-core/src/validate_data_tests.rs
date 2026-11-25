use std::sync::Arc;

use oku_slides_data::{CellKey, Column, ColumnType, Data, DataCell, DataId, DataKind, Rows, Table};
use rstest::rstest;
use serde_json::json;
use speculoos::prelude::*;

use super::*;
use crate::test_registry::registry;
use crate::validate::validate;

fn did(i: usize) -> DataId {
    DataId::try_from(format!("d{i:03}")).unwrap()
}

/// A data `name` with one number column `c1` and one row `r1`, holding `cell`.
fn data(name: &str, cell: DataCell) -> Arc<Data> {
    let table = Table {
        generation: "g1".to_owned(),
        epoch: String::new(),
        columns: vec![Column { id: "c1".to_owned(), name: "Sales".to_owned(), ty: ColumnType::Number }],
        rows: Rows::repaired(["r1".to_owned()]),
        base: None,
        cells: [(CellKey { row: "r1".to_owned(), column: "c1".to_owned() }, cell)].into_iter().collect(),
    };
    Arc::new(Data { name: name.to_owned(), label: None, inline: false, kind: DataKind::Manual, table })
}

fn deck_of(data: Vec<Arc<Data>>) -> Deck {
    Deck { data: data.into_iter().enumerate().map(|(i, d)| (did(i), d)).collect(), ..Deck::default() }
}

fn ids(issues: &[Issue]) -> Vec<&'static str> {
    issues.iter().map(|i| i.message.id()).collect()
}

#[test]
fn validate_two_names_differing_by_case_reports_name_taken() {
    let deck = deck_of(vec![data("Sales", DataCell::Number(1.0)), data("sales", DataCell::Number(2.0))]);
    let issues = validate(&deck, &registry(), None);
    let expected = data_issue(Message::new("issue-data-name-taken").arg("data", "sales"));
    assert_that!(issues).is_equal_to(vec![expected]);
}

#[test]
fn validate_cell_not_fitting_type_reports_cell_type() {
    let deck = deck_of(vec![data("sales", DataCell::Text("twelve".to_owned()))]);
    let issues = validate(&deck, &registry(), None);
    assert_that!(issues).is_equal_to(vec![cell_type("sales", "Sales", "number")]);
}

#[rstest]
#[case::at_the_cap(200, 200)]
#[case::one_past(201, 201)]
fn validate_data_issues_capped_at_200_then_more(#[case] count: usize, #[case] expected_len: usize) {
    let data = (0..count).map(|i| data(&format!("t{i}"), DataCell::Number(1.0))).collect();
    // Every data read truncated: one issue each.
    let mut deck = deck_of(data);
    deck.truncated = deck.data.keys().cloned().collect();
    let issues = validate(&deck, &registry(), None);
    assert_that!(issues.len()).is_equal_to(expected_len);
    let more = issues.iter().filter(|i| i.message.id() == "issue-more").count();
    assert_that!(more).is_equal_to(usize::from(count > MAX_DATA_ISSUES));
    if count > MAX_DATA_ISSUES {
        let last = issues.last().map(|i| i.message.clone());
        assert_that!(last).is_equal_to(Some(Message::new("issue-more").arg("count", count - MAX_DATA_ISSUES)));
    }
}

#[test]
fn validate_non_data_issues_not_capped() {
    let slides: Vec<_> = (0..250).map(|_| json!({ "id": "s", "kind": "essentiel/content" })).collect();
    let deck: Deck = serde_json::from_value(json!({ "version": 10, "slides": slides })).unwrap();
    let issues = validate(&deck, &registry(), None);
    let duplicates = ids(&issues).iter().filter(|id| **id == "issue-duplicate-slide-id").count();
    assert_that!(duplicates).is_equal_to(249);
    assert_that!(ids(&issues)).does_not_contain("issue-more");
}

/// Each Task 3 key with sample arguments.
fn task3_samples() -> Vec<(&'static str, Vec<(&'static str, oku_slides_i18n::FluentValue<'static>)>)> {
    vec![
        ("issue-data-name-taken", vec![("data", "sales".into())]),
        (
            "issue-data-cell-type",
            vec![("data", "sales".into()), ("column", "Amount".into()), ("type", "number".into())],
        ),
        ("issue-data-too-many", vec![("count", 300.into()), ("dropped", "d256, d257".into())]),
        ("issue-data-truncated", vec![("data", "sales".into())]),
        ("issue-data-base-missing", vec![("data", "sales".into())]),
        ("issue-data-edits-lost", vec![("data", "sales".into())]),
        ("issue-more", vec![("count", 3.into())]),
        ("op-data-deck-full", vec![("what", "row-ids".into()), ("max", 1_000_000.into())]),
    ]
}

#[test]
fn every_task3_key_formats_in_all_locales() {
    use oku_slides_i18n::Localizer;
    for locale in oku_slides_i18n::available() {
        let l = Localizer::new(Some(locale));
        for (key, args) in task3_samples() {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr_with(key, &args);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
            assert_that!(text.trim().is_empty()).named(&format!("{locale} {key}")).is_false();
        }
    }
    let (en, fr) = (Localizer::new(Some("en")), Localizer::new(Some("fr")));
    for (key, args) in task3_samples() {
        assert_that!(fr.tr_with(key, &args)).named(key).is_not_equal_to(en.tr_with(key, &args));
    }
}
