use std::io::Write;

use rstest::rstest;
use speculoos::prelude::*;
use tempfile::NamedTempFile;

use super::{Footer, check, check_footer};
use crate::engine::{read_view, write_base};
use crate::limits::{MAX_BASE_ROW_GROUPS, MAX_COLUMNS, MAX_ROWS};
use crate::test_support::{fold, typed};
use crate::{BaseColumn, BaseId, BaseRef, ColumnType, DataErrorKind, Table};

fn base_of(rows: u32, bytes: u64, columns: usize) -> BaseRef {
    BaseRef {
        id: BaseId::try_from(format!("sha256:{}", "0".repeat(64))).unwrap(),
        bytes,
        rows,
        columns: (0..columns)
            .map(|j| BaseColumn { id: format!("c{j}"), ty: ColumnType::Number, text_bytes: 0 })
            .collect(),
    }
}

fn footer_of(base: &BaseRef) -> Footer {
    let mut fields = vec![("__row_id".to_owned(), Some(ColumnType::Text))];
    fields.extend(base.columns.iter().map(|c| (c.id.clone(), Some(c.ty))));
    Footer { bytes: base.bytes, rows: base.rows as usize, row_groups: 1, uncompressed: 4096, fields }
}

#[rstest]
#[case::rows_at_bound(MAX_ROWS, 1, 1, 4096, true)]
#[case::rows_past_bound(MAX_ROWS + 1, 1, 1, 4096, false)]
#[case::columns_at_bound(10, MAX_COLUMNS, 1, 4096, true)]
#[case::columns_past_bound(10, MAX_COLUMNS + 1, 1, 4096, false)]
#[case::row_groups_at_bound(10, 1, MAX_BASE_ROW_GROUPS, 4096, true)]
#[case::row_groups_past_bound(10, 1, MAX_BASE_ROW_GROUPS + 1, 4096, false)]
#[case::small_file_declaring_3_gib(10, 1, 1, 3 << 30, false)]
fn check_footer_bounds(
    #[case] rows: usize,
    #[case] columns: usize,
    #[case] row_groups: usize,
    #[case] uncompressed: u64,
    #[case] accepted: bool,
) {
    let base = base_of(u32::try_from(rows).unwrap(), 4096, columns);
    let footer = Footer { row_groups, uncompressed, ..footer_of(&base) };

    let result = check(&footer, &base);

    if accepted {
        assert_that!(result).is_ok();
    } else {
        assert_that!(result.unwrap_err().kind()).is_equal_to(DataErrorKind::BaseInvalid);
    }
}

#[test]
fn check_footer_accepts_a_written_base() {
    let (based, file) = fold(&typed());

    assert_that!(check_footer(file.as_file(), based.base.as_ref().unwrap())).is_ok();
}

#[test]
fn check_footer_refuses_a_real_file_past_max_rows() {
    let table = crate::test_support::sized(MAX_ROWS + 1, 0);
    let mut out = NamedTempFile::new().unwrap();
    let mut base = write_base(&table, None, out.as_file_mut()).unwrap();
    base.rows = u32::try_from(MAX_ROWS + 1).unwrap();

    assert_that!(check_footer(out.as_file(), &base).unwrap_err().kind()).is_equal_to(DataErrorKind::BaseInvalid);
}

/// The based table over a file holding `bytes` instead of its base.
fn forged(bytes: &[u8], based: &Table) -> (Table, NamedTempFile) {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(bytes).unwrap();
    let mut table = based.clone();
    if let Some(base) = &mut table.base {
        base.bytes = bytes.len() as u64;
    }
    (table, file)
}

#[rstest]
#[case::truncated("truncated")]
#[case::random("random")]
#[case::wrong_magic("wrong magic")]
#[case::schema_mismatch("schema mismatch")]
fn forged_parquet_is_refused(#[case] how: &str) {
    let (based, file) = fold(&typed());
    let real = std::fs::read(file.path()).unwrap();
    let (table, forged_file) = match how {
        "truncated" => forged(&real[..real.len() / 2], &based),
        "random" => forged(
            &(0..4096_u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13).to_le_bytes()[0]).collect::<Vec<_>>(),
            &based,
        ),
        "wrong magic" => {
            let mut bytes = real.clone();
            let n = bytes.len();
            bytes[n - 4..].copy_from_slice(b"PAR0");
            forged(&bytes, &based)
        }
        _ => {
            let mut table = based.clone();
            if let Some(base) = &mut table.base {
                base.columns[0].ty = ColumnType::Number;
            }
            (table, file)
        }
    };
    let columns: Vec<String> = table.columns.iter().map(|c| c.id.clone()).collect();

    let error = read_view(&table, forged_file.reopen().unwrap(), &columns, 0..10, 100).unwrap_err();

    assert_that!(error.kind()).is_equal_to(DataErrorKind::BaseInvalid);
}

#[test]
fn engine_panic_is_base_invalid() {
    let result: Result<(), _> = super::super::guarded(|| panic!("decoder bug"));

    assert_that!(result.unwrap_err().kind()).is_equal_to(DataErrorKind::BaseInvalid);
}
