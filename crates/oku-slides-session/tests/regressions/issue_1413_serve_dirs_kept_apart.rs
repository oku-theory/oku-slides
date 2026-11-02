//! t-1413: `Workspace::new_in` emptied `<data>/serve` at launch, so a second dev server on
//! the same data folder deleted the first one's tabs, its agents' work folders included.
//! Each workspace now writes in a random folder of its own under that parent.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_session::{Fonts, Platform, Settings, Workspace};
use speculoos::prelude::*;

use crate::common::{Data, data_dir};

/// A dev-server workspace on `data` under `parent`, and its first tab's work folder.
fn served(data: &Data, parent: &Path) -> (Mutex<Workspace>, PathBuf) {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    let ws = Workspace::new_in(Fonts::default(), &platform, None, parent)
        .unwrap_or_else(|e| panic!("workspace: {}", e.one_line()));
    let ws = Mutex::new(ws);
    let (work, _) = Workspace::with_session(&ws, "t-1", |s| s.agent_work_dir());
    let work = work.unwrap_or_else(|e| panic!("tab: {}", e.one_line()));
    let dir = work.and_then(|w| w.prepare()).unwrap_or_else(|e| panic!("work dir: {}", e.one_line()));
    (ws, dir)
}

#[test]
fn issue_1413_two_servers_on_one_data_folder_keep_each_others_files() {
    let data = data_dir();
    let parent = data.join("serve");
    let (first, first_work) = served(&data, &parent);
    std::fs::write(first_work.join("notes.txt"), "first").unwrap_or_else(|e| panic!("{e}"));
    let (second, second_work) = served(&data, &parent);
    asserting("a folder of its own").that(&second_work).is_not_equal_to(&first_work);
    asserting("the second launch keeps the first one's files").that(&first_work.join("notes.txt").exists()).is_true();
    drop(second);
    asserting("the second one leaving keeps them too").that(&first_work.join("notes.txt").exists()).is_true();
    drop(first);
}
