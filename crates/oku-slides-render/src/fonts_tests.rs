use speculoos::prelude::*;

use super::*;

#[test]
fn fonts_files_lists_added_files_in_order() {
    let f = Fonts::new(vec![vec![1u8], vec![2u8]]).with(vec![vec![3u8]]);
    let files: Vec<Vec<u8>> = f.files().map(<[u8]>::to_vec).collect();
    assert_that!(files).is_equal_to(vec![vec![1u8], vec![2u8], vec![3u8]]);
}

#[test]
fn fonts_default_has_no_files() {
    assert_that!(Fonts::default().files().count()).is_equal_to(0);
}

#[test]
fn fonts_shared_files_point_to_the_same_bytes() {
    let f = Fonts::new(vec![vec![1u8, 2]]);
    let shared: Vec<_> = f.shared_files().collect();
    let files: Vec<&[u8]> = f.files().collect();
    assert_that!(shared[0].as_ref().as_ptr()).is_equal_to(files[0].as_ptr());
}
