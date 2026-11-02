//! t-1414: a received theme never enters the library without the parent it extends.
//!
//! A child theme was listed for install while its received parent could not
//! enter the library (blocked or unreadable): the library then held a theme
//! extending a parent it lacked.

use std::path::{Path, PathBuf};

use oku_slides_session::{Request, Storage, ThemeLibrary, VersionRef};
use serde_json::json;
use speculoos::prelude::*;

use crate::common::{add_user_pack, add_user_theme, data_dir, fixture_path, ns, session_in, snapshot_json};

fn themes(data: &Path) -> ThemeLibrary {
    let storage = Storage::local().unwrap_or_else(|e| panic!("{e}"));
    ThemeLibrary::new(Some(data.join("themes")), storage)
}

/// A file saved by someone else whose deck uses their theme `maison-sombre`, extending `maison`.
fn received(them: &Path) -> PathBuf {
    add_user_theme(them, "maison", &[], json!({"color": {"accent": "#204060"}}));
    add_user_theme(them, "maison-sombre", &["maison"], json!({"color": {"surface": "#123456"}}));
    let mut s = session_in(them);
    s.open(&fixture_path()).unwrap_or_else(|e| panic!("{}", e.one_line()));
    s.set_theme(&ns("maison-sombre")).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let path = them.join("received.oku-slides.pdf");
    s.save(Some(&path)).unwrap_or_else(|e| panic!("{}", e.one_line()));
    path
}

#[test]
fn issue_1414_child_theme_of_a_blocked_parent_stays_in_the_deck() {
    let (me, them) = (data_dir(), data_dir());
    let path = received(&them);
    // My pack holds the parent's namespace: the parent cannot enter my library.
    add_user_pack(&me, "maison");
    let mut s = session_in(&me);
    s.open(&path).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let review = snapshot_json(&mut s)["received_to_review"].clone();
    let rows = review.as_array().cloned().unwrap_or_default();
    let installs: Vec<_> = rows.iter().map(|r| r["install"].as_bool()).collect();
    assert_that!(installs).is_equal_to(vec![Some(false), Some(false)]);
    let versions: Vec<VersionRef> = rows
        .iter()
        .map(|r| serde_json::from_value(json!({"id": r["id"], "version": r["version"]})))
        .collect::<Result<_, _>>()
        .unwrap_or_else(|e| panic!("{e}"));
    s.handle(Request::AcceptReceived { versions }).unwrap_or_else(|e| panic!("{}", e.one_line()));
    let names: Vec<_> = themes(&me).themes().into_iter().map(|t| t.namespace).collect();
    assert_that!(names.contains(&ns("maison-sombre"))).is_false();
}
