use speculoos::prelude::*;

use super::*;

fn limits(rows: usize, cols: usize, text: usize) -> ExtractLimits {
    ExtractLimits {
        max_text_bytes: text,
        sheet_rows: rows,
        sheet_columns: cols,
        max_images: 0,
        max_image_pixels: 0,
        max_image_bytes: 0,
    }
}

fn row(cells: &[(u64, u64, &str)], grid: &Grid<'_>) -> Row {
    let mut row = Row::default();
    for &(col, repeat, text) in cells {
        row.set(col, repeat, text, grid);
    }
    row
}

fn grid<'l>(book: &mut Book<'l>) -> Grid<'l> {
    book.grid().unwrap()
}

#[test]
fn row_set_past_column_cap_counted_not_stored() {
    let l = limits(10, 2, 1000);
    let g = grid(&mut Book::new(&l));
    let r = row(&[(0, 1, "a"), (5, 3, "b")], &g);
    assert_that!(r.cells).is_equal_to(vec!["a".to_owned()]);
    assert_that!(r.width).is_equal_to(8);
}

#[test]
fn row_set_empty_text_ignored() {
    let l = limits(10, 2, 1000);
    let g = grid(&mut Book::new(&l));
    assert_that!(row(&[(0, 1_000_000, "")], &g).width).is_equal_to(0);
}

#[test]
fn grid_push_repeat_clamped_and_rest_counted() {
    let l = limits(3, 2, 1000);
    let mut g = grid(&mut Book::new(&l));
    let r = row(&[(0, 5, "x")], &g);
    g.push(&r, u64::MAX);
    assert_that!(g.rows.len()).is_equal_to(3);
    assert_that!(g.extra_rows).is_equal_to(u64::MAX - 3);
    assert_that!(g.extra_cols).is_equal_to(3);
}

#[test]
fn grid_push_past_text_budget_counted_as_text() {
    let l = limits(100, 2, 20);
    let mut g = grid(&mut Book::new(&l));
    let r = row(&[(0, 1, "abcd")], &g);
    g.push(&r, 5);
    assert_that!(g.rows.len()).is_equal_to(2);
    assert_that!(g.extra_rows).is_equal_to(0);
    assert_that!(g.dropped).is_equal_to(12);
}

#[test]
fn grid_push_content_only_past_column_cap_not_stored() {
    let l = limits(100, 2, 1000);
    let mut g = grid(&mut Book::new(&l));
    let r = row(&[(7, 1, "far")], &g);
    g.push(&r, 3);
    assert_that!(g.rows).is_empty();
    assert_that!(g.extra_cols).is_equal_to(6);
    assert_that!(g.extra_rows).is_equal_to(0);
}

#[test]
fn row_cost_counts_empty_slots_before_a_far_cell() {
    let l = limits(100, 50, 1000);
    let g = grid(&mut Book::new(&l));
    let r = row(&[(49, 1, "1")], &g);
    assert_that!(r.cost()).is_equal_to(1 + SLOT * 50);
    let tight = limits(100, 50, 100);
    let g = grid(&mut Book::new(&tight));
    let r = row(&[(49, 1, "1")], &g);
    assert_that!(r.cells).is_empty();
    assert_that!(r.dropped).is_equal_to(1);
}

#[test]
fn book_budget_shared_then_next_sheets_unread() {
    let l = limits(100, 2, 20);
    let mut book = Book::new(&l);
    let mut first = grid(&mut book);
    let r = row(&[(0, 1, "abcdef")], &first);
    first.push(&r, 1);
    book.add("A".to_owned(), first);
    let second = grid(&mut book);
    assert_that!(second.room()).is_equal_to(10);
    let mut second = second;
    let r = row(&[(0, 1, "abcdefghij")], &second);
    second.push(&r, 1);
    book.add("B".to_owned(), second);
    assert_that!(book.grid().is_none()).is_true();
    assert_that!(book.grid().is_none()).is_true();
    let (md, cuts) = book.render();
    assert_that!(md).ends_with(
        "## B\n\n*(empty sheet)*\n\n… 10 bytes not imported (limit 20)\n\n… 2 sheets not imported (text budget spent)\n",
    );
    assert_that!(cuts)
        .is_equal_to(vec![Cut { what: CutKind::Text, count: 10 }, Cut { what: CutKind::Sheets, count: 2 }]);
}

#[test]
fn render_sections_cuts_and_skipped() {
    let l = limits(1, 1, 1000);
    let mut book = Book::new(&l);
    let mut g = grid(&mut book);
    let r = row(&[(0, 2, "h")], &g);
    g.push(&r, 2);
    book.add("One".to_owned(), g);
    let empty = grid(&mut book);
    book.add("Two #".to_owned(), empty);
    book.skip("Secret|x".to_owned(), "hidden");
    let (md, cuts) = book.render();
    assert_that!(md).is_equal_to(
        "## One\n\n| h |\n| --- |\n\n… 1 rows not imported (limit 1)\n… 1 columns not imported (limit 1)\n\n## Two \\#\n\n*(empty sheet)*\n\n… sheet \"Secret\\|x\" not imported (hidden)\n"
            .to_owned(),
    );
    assert_that!(cuts)
        .is_equal_to(vec![Cut { what: CutKind::Rows, count: 1 }, Cut { what: CutKind::Columns, count: 1 }]);
}

#[test]
fn heading_trailing_hash_kept_as_text() {
    let md = format!("## {}\n", heading("Data #"));
    let events: Vec<_> = pulldown_cmark::Parser::new(&md).collect();
    let text: String = events
        .iter()
        .filter_map(|e| match e {
            pulldown_cmark::Event::Text(t) => Some(t.to_string()),
            _ => None,
        })
        .collect();
    assert_that!(text).is_equal_to("Data #".to_owned());
}

#[test]
fn add_cut_sums_the_same_kind() {
    let mut cuts = vec![Cut { what: CutKind::Rows, count: 2 }];
    add_cut(&mut cuts, Cut { what: CutKind::Rows, count: 3 });
    add_cut(&mut cuts, Cut { what: CutKind::Text, count: 1 });
    assert_that!(cuts).is_equal_to(vec![Cut { what: CutKind::Rows, count: 5 }, Cut { what: CutKind::Text, count: 1 }]);
}
