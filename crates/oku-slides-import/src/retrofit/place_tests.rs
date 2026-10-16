use speculoos::prelude::*;

use super::*;
use crate::source::Picture;
use crate::test_support::{item, text_item};

#[test]
fn aspects_read_the_frames_of_pictures_inside_groups() {
    let pic = |id: &str, asset: &str, w: f64, h: f64| {
        item(id, (0.0, 0.0, w, h), ItemKind::Picture(Picture { asset: asset.to_owned(), crop: Default::default() }))
    };
    let group = item(
        "g",
        (0.0, 0.0, 400.0, 100.0),
        ItemKind::Group { items: vec![pic("g/1", "b", 200.0, 50.0)], smartart: None },
    );
    let (a, t) = (pic("p", "a", 400.0, 300.0), text_item("t", (0.0, 0.0, 10.0, 10.0), "x"));
    let found = aspects(&[&a, &group, &t]);
    assert_that!(found).is_equal_to(BTreeMap::from([("a".to_owned(), 0.75), ("b".to_owned(), 0.25)]));
}
