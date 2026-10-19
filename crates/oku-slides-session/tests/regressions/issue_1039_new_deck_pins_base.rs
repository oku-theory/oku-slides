//! t-1039 : un deck neuf écrivait son texte `base/text` sans épingler `base` : élément inconnu.

use oku_slides_core::kinds;
use speculoos::prelude::*;

use crate::common::{data_dir, session_in};

#[test]
fn issue_1039_new_deck_knows_its_base_text() {
    let data = data_dir();
    let s = session_in(&data);
    let text = s.sources_for_tests().registry().get(kinds::base::TEXT).map(|w| w.typography);
    assert_that!((s.deck().widgets.contains_key(&kinds::base::namespace()), text)).is_equal_to((true, Some(true)));
}
