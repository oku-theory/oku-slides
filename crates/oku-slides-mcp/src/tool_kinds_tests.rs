use oku_slides_session::DocumentKind;
use rstest::rstest;
use speculoos::prelude::*;

use super::Kinds;

#[rstest]
#[case(Kinds::DECK_OR_HELP, DocumentKind::Help, true)]
#[case(Kinds::DECK_OR_HELP, DocumentKind::Pack, false)]
#[case(Kinds::PACK.or(Kinds::THEME), DocumentKind::Theme, true)]
#[case(Kinds::PACK.or(Kinds::THEME), DocumentKind::Deck, false)]
#[case(Kinds::ALL, DocumentKind::Pack, true)]
fn kinds_has_exactly_its_kinds(#[case] k: Kinds, #[case] kind: DocumentKind, #[case] has: bool) {
    assert_that!(k.has(kind)).is_equal_to(has);
}
