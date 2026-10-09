use std::io::{Read, Write};
use std::os::fd::AsFd;

use speculoos::prelude::*;

use super::*;

#[test]
fn divert_keeps_a_copy_of_out_and_points_out_at_err() {
    let (mut proto_r, mut proto_w) = std::io::pipe().unwrap();
    let (mut err_r, err_w) = std::io::pipe().unwrap();
    let copy = divert(proto_w.as_fd(), err_w.as_fd()).unwrap();
    drop(err_w);
    // Le descripteur d'origine écrit désormais dans `err`…
    proto_w.write_all(b"print").unwrap();
    drop(proto_w);
    // … et la copie dans le tube d'origine.
    let mut copy = std::fs::File::from(copy);
    copy.write_all(b"frame").unwrap();
    drop(copy);
    let (mut proto, mut err) = (String::new(), String::new());
    proto_r.read_to_string(&mut proto).unwrap();
    err_r.read_to_string(&mut err).unwrap();
    assert_that!((proto.as_str(), err.as_str())).is_equal_to(("frame", "print"));
}
