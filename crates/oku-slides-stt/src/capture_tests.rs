use speculoos::prelude::*;

use crate::test_support::{fresh_dir, platform_in};
use crate::{Mode, State, Stt};

#[test]
fn begin_recording_refused_once_state_moved_on() {
    let dir = fresh_dir("capture-race");
    let stt = Stt::new(&platform_in(dir.path(), "fr", |s| s.apple = false));
    // Another `listen` won the race: listening already started from idle.
    let session = {
        let mut g = stt.state();
        g.state = State::Listening;
        g.session
    };
    stt.begin_recording(Mode::Listen, State::Idle);
    let g = stt.state();
    assert_that!((g.session, g.state)).is_equal_to((session, State::Listening));
}
