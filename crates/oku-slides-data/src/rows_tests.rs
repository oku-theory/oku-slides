use rstest::rstest;
use speculoos::prelude::*;

use super::*;

fn ids(n: usize) -> Rows {
    Rows::from_unique((0..n).map(|i| format!("r{i}")))
}

/// Checks every id is found at its own position.
fn assert_consistent(rows: &Rows) {
    for (i, id) in rows.iter().enumerate() {
        assert_that!(rows.index_of(id)).named(id).is_equal_to(Some(i));
        assert_that!(rows.contains(id)).named(id).is_true();
    }
}

#[test]
fn index_of_finds_each_row_and_not_others() {
    let rows = ids(100);
    assert_consistent(&rows);
    assert_that!(rows.index_of("r100")).is_none();
    assert_that!(rows.contains("r100")).is_false();
}

#[rstest]
#[case::first(0)]
#[case::middle(3)]
#[case::last(6)]
fn insert_puts_the_row_at_its_index(#[case] at: usize) {
    let mut rows = ids(6);
    rows.insert(at, "new".to_owned());
    assert_that!(rows.get(at)).is_equal_to(Some("new"));
    assert_that!(rows.len()).is_equal_to(7);
    assert_consistent(&rows);
}

#[test]
fn insert_many_after_the_same_row_relabels_and_stays_ordered() {
    // Each insert halves the gap after `r0`: past ~128 of them the labels are spread again.
    let mut rows = ids(3);
    for i in 0..300 {
        rows.insert(1, format!("n{i}"));
    }
    assert_that!(rows.get(1)).is_equal_to(Some("n299"));
    assert_that!(rows.get(300)).is_equal_to(Some("n0"));
    assert_that!(rows.get(302)).is_equal_to(Some("r2"));
    assert_consistent(&rows);
}

#[test]
fn insert_many_at_the_end_stays_ordered() {
    let mut rows = Rows::default();
    for i in 0..300 {
        rows.insert(rows.len(), format!("n{i}"));
    }
    assert_that!(rows.iter().map(str::to_owned).collect::<Vec<_>>())
        .is_equal_to((0..300).map(|i| format!("n{i}")).collect::<Vec<_>>());
    assert_consistent(&rows);
}

#[test]
fn remove_forgets_the_row() {
    let mut rows = ids(5);
    assert_that!(rows.remove(2)).is_equal_to("r2".to_owned());
    assert_that!(rows.contains("r2")).is_false();
    assert_that!(rows.index_of("r2")).is_none();
    assert_consistent(&rows);
}

#[test]
fn equality_ignores_labels() {
    let mut edited = ids(2);
    edited.insert(1, "x".to_owned());
    edited.remove(1);
    assert_that!(edited).is_equal_to(ids(2));
    let mut other = ids(2);
    other.remove(1);
    assert_that!(other).is_not_equal_to(ids(2));
}

#[test]
fn clone_shares_storage_until_edited() {
    let rows = ids(100);
    let mut copy = rows.clone();
    assert_that!(copy.ptr_eq(&rows)).is_true();
    copy.remove(0);
    assert_that!(copy.ptr_eq(&rows)).is_false();
}

#[test]
fn json_round_trips_as_a_list_of_ids() {
    let json = serde_json::to_string(&ids(2)).unwrap();
    assert_that!(json.as_str()).is_equal_to(r#"["r0","r1"]"#);
    let back: Rows = serde_json::from_str(&json).unwrap();
    assert_that!(back).is_equal_to(ids(2));
    assert_consistent(&back);
}

#[rstest]
#[case::repeated(r#"["a","b","a"]"#)]
#[case::invalid(r#"["a/b"]"#)]
#[case::empty_id(r#"[""]"#)]
fn json_with_bad_ids_is_refused(#[case] json: &str) {
    assert_that!(serde_json::from_str::<Rows>(json)).is_err();
}

#[test]
fn repaired_rows_keep_first_occurrence_and_suffix_duplicates() {
    let rows = Rows::repaired(["a", "a", "b/c", "b"].map(String::from));
    assert_that!(rows.iter().collect::<Vec<_>>()).is_equal_to(vec!["a", "a_2", "x", "b"]);
    assert_that!(rows.index_of("x")).is_equal_to(Some(2));
}
