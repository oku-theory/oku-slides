use oku_slides_i18n::{FluentValue, Localize, Localizer, available};
use speculoos::prelude::*;

use super::*;

/// Every `data-*` key with every variable it declares (each `$what` of `data-too-large`).
fn samples(n: usize) -> Vec<(&'static str, Vec<(&'static str, FluentValue<'static>)>)> {
    let mut samples = vec![
        ("data-cell-type", vec![("column", "Ventes".into()), ("type", "number".into())]),
        (
            "data-column-type",
            vec![
                ("column", "Ventes".into()),
                ("type", "integer".into()),
                ("rows", "ra, rb".into()),
                ("more", n.into()),
            ],
        ),
        ("data-no-row", vec![("row", "r1".into())]),
        ("data-no-column", vec![("column", "c1".into())]),
        ("data-id-taken", vec![("id", "c1".into())]),
        ("data-id-invalid", vec![("id", "a/b".into())]),
        ("data-id-missing", vec![]),
        ("data-invalid-name", vec![("name", "Ventes".into())]),
        ("data-non-finite", vec![("count", n.into())]),
        ("data-columns-invalid", vec![]),
        ("data-row-width", vec![("max", n.into())]),
        ("data-needs-base", vec![]),
        ("data-base-row-cells", vec![]),
        ("data-source-mismatch", vec![]),
        ("data-query-parse", vec![("line", "2".into()), ("column", "14".into())]),
        ("data-query-construct", vec![("construct", "ATTACH".into())]),
        ("data-query-too-long", vec![("max", n.into())]),
        ("data-query-too-deep", vec![]),
        ("data-query-rewrite", vec![]),
        ("data-query-unknown-function", vec![("function", "read_csv".into())]),
        ("data-query-run", vec![("reason", "column not found".into())]),
        ("data-query-column-type", vec![("column", "moment".into())]),
        ("data-query-duplicate-column", vec![("column", "product".into())]),
        ("data-query-float-operand", vec![("function", "%".into()), ("operand", "price".into())]),
        ("data-query-timeout", vec![("seconds", n.into())]),
        ("data-query-input-failed", vec![("input", "Ventes".into())]),
    ];
    for what in ["rows", "columns", "text", "overlay"] {
        samples.push(("data-too-large", vec![("what", what.into()), ("max", n.into())]));
    }
    for what in QUERY_BOUNDS {
        samples.push(("data-query-too-large", vec![("what", (*what).into()), ("max", n.into())]));
    }
    samples
}

/// Keys whose French text may equal the English one.
const FR_EQUALS_EN: &[&str] = &[];

/// The `$what` of `data-query-too-large`.
const QUERY_BOUNDS: &[&str] = &["rows", "columns", "text"];

/// Keys whose numeric variable picks a plural form.
const NUMERIC: &[&str] = &[
    "data-column-type",
    "data-non-finite",
    "data-row-width",
    "data-too-large",
    "data-query-too-long",
    "data-query-too-large",
    "data-query-timeout",
];

#[test]
fn every_data_key_formats_in_all_locales() {
    let en = Localizer::new(Some("en"));
    let fr = Localizer::new(Some("fr"));
    for locale in available() {
        let l = Localizer::new(Some(locale));
        for (key, args) in samples(3) {
            assert_that!(l.has(key)).named(&format!("{locale} {key}")).is_true();
            let text = l.tr_with(key, &args);
            assert_that!(text.contains('{')).named(&format!("{locale} {key}: {text}")).is_false();
            assert_that!(text.trim().is_empty()).named(&format!("{locale} {key}")).is_false();
        }
    }
    for (key, args) in samples(3) {
        if !FR_EQUALS_EN.contains(&key) {
            assert_that!(fr.tr_with(key, &args)).named(key).is_not_equal_to(en.tr_with(key, &args));
        }
    }
}

/// The message of `key` with `n` (every other sample variable kept), the number replaced by `#`.
fn form(l: &Localizer, key: &str, what: Option<&str>, n: usize) -> String {
    let (_, args) = samples(n)
        .into_iter()
        .find(|(k, args)| {
            *k == key && what.is_none_or(|w| args.iter().any(|(name, v)| *name == "what" && *v == FluentValue::from(w)))
        })
        .unwrap();
    l.tr_with(key, &args).replace(&n.to_string(), "#")
}

#[test]
fn every_numeric_data_key_has_distinct_plural_forms_in_ru_and_pl() {
    let ru = Localizer::new(Some("ru"));
    let pl = Localizer::new(Some("pl"));
    let cases: Vec<(&str, Option<&str>)> = NUMERIC
        .iter()
        .flat_map(|key| {
            if *key == "data-too-large" {
                ["rows", "columns", "text", "overlay"].iter().map(|w| (*key, Some(*w))).collect::<Vec<_>>()
            } else if *key == "data-query-too-large" {
                QUERY_BOUNDS.iter().map(|w| (*key, Some(*w))).collect::<Vec<_>>()
            } else {
                vec![(*key, None)]
            }
        })
        .collect();
    for (key, what) in cases {
        let name = format!("{key} {what:?}");
        let [one, two, five] = [1, 2, 5].map(|n| form(&ru, key, what, n));
        assert_that!(one).named(&name).is_not_equal_to(two.clone());
        assert_that!(two).named(&name).is_not_equal_to(five.clone());
        assert_that!(one).named(&name).is_not_equal_to(five);
        assert_that!(form(&ru, key, what, 21)).named(&name).is_equal_to(form(&ru, key, what, 1));
        assert_that!(form(&ru, key, what, 22)).named(&name).is_equal_to(form(&ru, key, what, 2));
        let [one, two, five] = [1, 2, 5].map(|n| form(&pl, key, what, n));
        assert_that!(one).named(&name).is_not_equal_to(two.clone());
        assert_that!(two).named(&name).is_not_equal_to(five.clone());
        assert_that!(one).named(&name).is_not_equal_to(five);
    }
}

#[test]
fn data_column_type_with_no_more_rows_names_only_the_rows() {
    let error: DataError =
        ColumnTypeSnafu { column: "Ventes", ty: ColumnType::Integer, rows: vec!["r1".to_owned()], more: 0_usize }
            .build()
            .into();
    let fr = error.localize(&Localizer::new(Some("fr")));
    assert_that!(error.kind()).is_equal_to(DataErrorKind::ColumnType);
    assert_that!(fr).contains("Ventes");
    assert_that!(fr).contains("r1");
    assert_that!(fr.contains('0')).is_false();
}

#[test]
fn invalid_name_cuts_the_name_to_80_chars_in_the_message() {
    let error = DataError::invalid_name(&"ж".repeat(100));
    let fr = error.localize(&Localizer::new(Some("fr")));
    assert_that!(error.kind()).is_equal_to(DataErrorKind::InvalidName);
    assert_that!(fr.matches('ж').count()).is_equal_to(80);
}

#[test]
fn message_of_an_error_is_its_key_and_localizes_as_the_error() {
    let fr = Localizer::new(Some("fr"));
    let errors: Vec<DataError> = vec![
        QueryParseSnafu { line: 2_u64, column: 14_u64 }.build().into(),
        QueryConstructSnafu { construct: "DELETE" }.build().into(),
        ColumnTypeSnafu { column: "Ventes", ty: ColumnType::Integer, rows: vec!["r1".to_owned()], more: 2_usize }
            .build()
            .into(),
    ];
    let ids: Vec<&str> = errors.iter().map(|e| e.message().id()).collect();
    assert_that!(ids).is_equal_to(vec!["data-query-parse", "data-query-construct", "data-column-type"]);
    for error in &errors {
        assert_that!(error.message().localize(&fr)).is_equal_to(error.localize(&fr));
    }
}
