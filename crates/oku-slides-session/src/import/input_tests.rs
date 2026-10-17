use std::path::Path;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_import::source::SourceDeck;
use oku_slides_import::{ImportErrorKind, ReadLimits, read_pptx};
use speculoos::prelude::*;

use super::{read_on_own_thread, read_source};
use crate::import::ImportErrorKind as Kind;

/// Un lecteur qui panique, comme un bogue du lecteur sur un fichier.
fn panicking(_: &[u8], _: &ReadLimits) -> Result<SourceDeck, oku_slides_import::ImportError> {
    panic!("lecteur en panique")
}

#[test]
fn read_on_own_thread_garbage_gives_the_reader_error() {
    let read = read_on_own_thread(b"not a zip", &ReadLimits::default(), read_pptx)
        .map(|r| r.ok().map(|r| r.map(|_| ()).map_err(|e| e.kind())));
    assert_that!(read.ok().flatten()).is_equal_to(Some(Err(ImportErrorKind::Unreadable)));
}

#[test]
fn read_source_of_a_panicking_reader_is_a_localized_presentation_error() {
    let err = read_source(Path::new("/x/deck.pptx"), b"", &ReadLimits::default(), panicking).unwrap_err();
    assert_that!(err.kind()).is_equal_to(Kind::Presentation);
    let (fr, en) = (err.localize(&Localizer::new(Some("fr"))), err.localize(&Localizer::new(Some("en"))));
    assert_that!(fr.as_str()).contains("/x/deck.pptx");
    assert_that!(fr).is_not_equal_to(en);
}
