use std::sync::Mutex;

use serde_json::json;
use speculoos::prelude::*;

use super::workspace_fixtures::{Data, ids, saved_pdf, set_theme, view};
use super::*;
use crate::test_support::{apply_ws, workspace_on, workspace_platform};
use crate::{Request, SessionErrorKind};

#[test]
fn request_reports_a_change_of_tab_info_only_once() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let (reply, changed) = Workspace::request(&ws, "t-1", Request::Snapshot);
    assert_that!(reply).is_ok();
    assert_that!(changed).is_false();
    assert_that!(Workspace::request(&ws, "t-1", set_theme()).1).is_true();
    assert_that!(Workspace::request(&ws, "t-1", Request::Snapshot).1).is_false();
    assert_that!(Workspace::request(&ws, "t-9", Request::Snapshot).1).is_false();
}

/// L'ordre des verrous : une requête d'onglet et la fermeture du même
/// onglet, en parallèle, ne s'interbloquent pas (un interblocage échoue au
/// bout de 10 s au lieu de bloquer la suite de tests).
#[test]
fn request_and_close_of_the_same_tab_in_parallel_do_not_deadlock() {
    let data = Data::new();
    let ws = std::sync::Arc::new(workspace_on(&data.0));
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let (done, finished) = std::sync::mpsc::channel();
    {
        let (ws, done) = (std::sync::Arc::clone(&ws), done.clone());
        std::thread::spawn(move || {
            for _ in 0..20 {
                let _ = Workspace::request(&ws, "t-2", Request::Snapshot);
            }
            done.send(()).unwrap();
        });
    }
    {
        let ws = std::sync::Arc::clone(&ws);
        std::thread::spawn(move || {
            apply_ws(&ws, json!({"cmd": "close", "tab": "t-2", "discard": true})).unwrap();
            done.send(()).unwrap();
        });
    }
    for _ in 0..2 {
        finished.recv_timeout(std::time::Duration::from_secs(10)).expect("interblocage");
    }
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
}

#[test]
fn close_last_tab_of_last_window_leaves_no_window() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-1"})).unwrap();
    let v = view(&ws);
    assert_that!(v.windows).is_empty();
    assert_that!(v.focused.as_str()).is_equal_to("");
}

/// Une construction qui chevauche `settle` : l'onglet n'est pas inséré,
/// sa session part à la corbeille (son dossier de reprise avec elle).
#[test]
fn finish_after_settle_refuses_and_trashes_the_built_tab() {
    let data = Data::new();
    let dir = data.0.join("resume");
    let ws = Mutex::new(Workspace::resume(Fonts::default(), &workspace_platform(&data.0), &dir).unwrap());
    let pending = Workspace::lock(&ws).pending(Some("w-1".into()), Opening::Blank, TabPlace::End);
    let session = pending.build().unwrap();
    let built = dir.join(TABS_DIR).join(&pending.id);
    assert_that!(built.exists()).is_true();
    Workspace::settle(&ws);
    let refused = Workspace::finish(&ws, pending, session);
    assert_that!(refused.map(|_| ())).is_err();
    assert_that!(ids(&view(&ws).windows[0])).is_equal_to(vec!["t-1"]);
    assert_that!(built.exists()).is_false();
}

/// Un reste illisible au numéro d'un onglet neuf ne lui coûte pas le fichier
/// qu'il vient d'ouvrir : le reste est mis de côté, l'onglet garde le fichier.
#[test]
fn open_tab_over_a_leftover_resume_dir_keeps_the_opened_file() {
    let data = Data::new();
    let dir = data.0.join("resume");
    let file = saved_pdf(&data, "gardé");
    let ws = Mutex::new(Workspace::resume(Fonts::default(), &workspace_platform(&data.0), &dir).unwrap());
    let leftover = dir.join(TABS_DIR).join("t-2");
    std::fs::create_dir_all(&leftover).unwrap();
    std::fs::write(leftover.join("state.json"), "pas du json").unwrap();
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    let w = &view(&ws).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-2"]);
    assert_that!(w.tabs[0].info.document.path()).is_equal_to(Some(file.as_path()));
    // Le reste n'est ni relu ni écrasé : mis de côté.
    let names: Vec<String> = std::fs::read_dir(dir.join(TABS_DIR))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    assert_that!(names.iter().any(|n| n.starts_with("t-2.failed-"))).is_true();
    // Le reste mis de côté, la reprise écrit l'onglet : il survit au relancement.
    drop(ws);
    let again = Mutex::new(Workspace::resume(Fonts::default(), &workspace_platform(&data.0), &dir).unwrap());
    let w = &view(&again).windows[0];
    assert_that!(ids(w)).is_equal_to(vec!["t-2"]);
    assert_that!(w.tabs[0].info.document.path()).is_equal_to(Some(file.as_path()));
}

#[test]
fn is_tab_id_accepts_only_t_and_digits() {
    assert_that!(is_tab_id("t-12")).is_true();
    assert_that!(is_tab_id("t-999999999999999999")).is_true();
    for bad in ["t-", "t-1a", "../t-1", "/tmp", "t-1234567890123456789", "t-12345678901234567890", "w-1"] {
        asserting(bad).that(&is_tab_id(bad)).is_false();
    }
    assert_that!(is_window_label("w-3")).is_true();
    assert_that!(is_window_label("main")).is_false();
}

/// t-159 : une vue construite plus tard porte un numéro plus grand, même
/// sans changement ; l'interface écarte une vue plus ancienne que la sienne.
#[test]
fn view_generation_grows_with_each_view() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let first = view(&ws).generation;
    let second = view(&ws).generation;
    assert_that!(second).is_greater_than(first);
}

#[test]
fn view_generation_of_a_reply_is_newer_than_an_earlier_view() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let before = view(&ws).generation;
    let reply = apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap().into_json();
    let after: serde_json::Value = serde_json::from_str(&reply).unwrap();
    assert_that!(after["generation"].as_u64().unwrap()).is_greater_than(before);
}

/// Un onglet fermé avant d'avoir été relu n'est plus jamais réveillé : une
/// requête en retard ne réécrit pas le dossier qu'on vient de supprimer.
#[test]
fn slot_closed_while_dormant_refuses_to_wake_and_writes_nothing() {
    let data = Data::new();
    let dir = data.0.join("tabs/t-9");
    let (platform, document) = (workspace_platform(&data.0), crate::TabDocument::Deck { path: None, deck_uid: None });
    let d = Dormant {
        dir: dir.clone(),
        title: "t-9".into(),
        document,
        notice: None,
        fonts: Fonts::default(),
        platform,
        usage: Vec::new(),
    };
    let mut slot = Slot::Dormant(Box::new(d));
    slot.stop_resume("t-9");
    let err = slot.ready_with_links(0, &[]).err().map(|e| e.kind());
    assert_that!(err).is_equal_to(Some(SessionErrorKind::UnknownTab));
    assert_that!(dir.exists()).is_false();
}

#[test]
fn with_session_runs_f_and_reports_that_the_tab_changed() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let (read, changed) = Workspace::with_session(&ws, "t-1", |s| s.deck().slides.len());
    assert_that!(read).is_ok();
    assert_that!(changed).is_false();
    let (applied, changed) = Workspace::with_session(&ws, "t-1", |s| s.handle(set_theme()).is_ok());
    assert_that!(applied).is_ok().is_true();
    assert_that!(changed).is_true(); // l'onglet n'est plus vierge : la barre change
}

#[test]
fn with_session_on_a_closed_tab_returns_unknown_tab_without_calling_f() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    apply_ws(&ws, json!({"cmd": "close", "tab": "t-1", "discard": true})).unwrap();
    let mut called = false;
    let (r, changed) = Workspace::with_session(&ws, "t-1", |_| called = true);
    assert_that!(r.map_err(|e| e.kind())).is_err().is_equal_to(SessionErrorKind::UnknownTab);
    assert_that!(called).is_false();
    assert_that!(changed).is_false();
}

/// t-249 : après `settle`, l'agent ne change plus rien (sa reprise est écrite).
#[test]
fn with_session_after_settle_returns_unknown_tab_without_calling_f() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    Workspace::settle(&ws);
    let mut called = false;
    let (r, changed) = Workspace::with_session(&ws, "t-1", |_| called = true);
    assert_that!(r.map_err(|e| e.kind())).is_err().is_equal_to(SessionErrorKind::UnknownTab);
    assert_that!(called).is_false();
    assert_that!(changed).is_false();
}

#[test]
fn with_session_releases_the_workspace_lock_while_f_runs() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let (free, _) = Workspace::with_session(&ws, "t-1", |_| ws.try_lock().is_ok());
    assert_that!(free).is_ok().is_true();
}

#[test]
fn open_elsewhere_is_true_only_for_the_file_of_another_tab() {
    use crate::agent::Sessions;
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let file = saved_pdf(&data, "ailleurs");
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    apply_ws(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})).unwrap();
    // t-2 tient le fichier.
    assert_that!(ws.open_elsewhere("t-1", &file)).is_true();
    assert_that!(ws.open_elsewhere("t-2", &file)).is_false();
    assert_that!(ws.open_elsewhere("t-1", &data.0.join("inconnu.oku-slides.pdf"))).is_false();
}

#[test]
fn tab_ids_lists_the_tabs_of_every_window() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_window"})).unwrap();
    assert_that!(Workspace::lock(&ws).tab_ids()).is_equal_to(vec!["t-1".to_owned(), "t-2".to_owned()]);
}

#[test]
fn closed_by_reports_the_tabs_a_request_closed_or_replaced() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    apply_ws(&ws, json!({"cmd": "new_tab", "window": "w-1"})).unwrap();
    let (r, closed) =
        Workspace::closed_by(&ws, |ws| apply_ws(ws, json!({"cmd": "close", "tab": "t-1", "discard": true})));
    assert_that!(r).is_ok();
    assert_that!(closed).is_equal_to(vec!["t-1".to_owned()]);
    // Onglet vierge remplacé par un fichier : fermé lui aussi.
    let file = saved_pdf(&data, "remplace");
    let (_, closed) =
        Workspace::closed_by(&ws, |ws| apply_ws(ws, json!({"cmd": "open_tab", "window": "w-1", "path": file})));
    assert_that!(closed).is_equal_to(vec!["t-2".to_owned()]);
    let (_, closed) = Workspace::closed_by(&ws, |ws| apply_ws(ws, json!({"cmd": "new_tab", "window": "w-1"})));
    assert_that!(closed).is_empty();
}

#[test]
fn a_new_tab_launches_its_render_worker_at_its_first_request() {
    let data = Data::new();
    let ws = workspace_on(&data.0);
    let slot = Workspace::lock(&ws).session("t-1").unwrap();
    let started = || matches!(&*relock(&slot), Slot::Ready(s) if s.worker.is_started());
    assert_that!(started()).is_false();
    let (rendered, _) = Workspace::with_session(&ws, "t-1", |s| s.compiled.is_some());
    assert_that!(rendered).is_ok().is_true();
    assert_that!(started()).is_true();
}

#[test]
fn with_session_wakes_a_dormant_resumed_tab() {
    let data = Data::new();
    let dir = data.0.join("resume");
    let ws = Mutex::new(Workspace::resume(Fonts::default(), &workspace_platform(&data.0), &dir).unwrap());
    Workspace::request(&ws, "t-1", set_theme()).0.unwrap();
    Workspace::settle(&ws);
    drop(ws);
    let again = Mutex::new(Workspace::resume(Fonts::default(), &workspace_platform(&data.0), &dir).unwrap());
    let slot = Workspace::lock(&again).session("t-1").unwrap();
    assert_that!(matches!(*relock(&slot), Slot::Dormant(_))).is_true();
    let (theme, _) =
        Workspace::with_session(&again, "t-1", |s| s.deck().theme.as_ref().and_then(crate::test_support::theme_name));
    assert_that!(theme).is_ok().is_equal_to(Some("pro"));
    assert_that!(matches!(*relock(&slot), Slot::Ready(_))).is_true();
}
