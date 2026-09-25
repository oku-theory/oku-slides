use speculoos::prelude::*;

use super::pdf_date;

#[test]
fn pdf_date_now_follows_pdf_date_syntax() {
    let d = pdf_date();
    asserting(&d).that(&d.len()).is_equal_to("D:20260925120000Z".len());
    asserting(&d).that(&(d.starts_with("D:20") && d.ends_with('Z'))).is_true();
    asserting(&d).that(&d[2..16].bytes().all(|b| b.is_ascii_digit())).is_true();
}
