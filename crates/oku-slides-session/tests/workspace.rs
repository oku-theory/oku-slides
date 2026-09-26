//! Reprise de l'espace de travail : fenêtres, onglets, et chaque session.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_session::{Fonts, Platform, Request, Settings, Workspace, WorkspaceRequest, WorkspaceView};
use serde_json::json;
use speculoos::prelude::*;

/// Un dossier de données neuf, supprimé à la fin du test.
struct Data(PathBuf);

impl Data {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "oku-slides-ws-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn resume(&self) -> PathBuf {
        self.0.join("resume")
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

// Helpers hors des `#[test]` : `unwrap_or_else(panic!)` plutôt que
// `unwrap()`, que clippy refuse ici (`unwrap_used`), comme dans `session.rs`.

fn platform(data: &Path) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.to_owned());
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"))
}

/// L'espace de travail qui reprend l'état laissé dans `data`.
fn resumed(data: &Data) -> Mutex<Workspace> {
    let ws = Workspace::resume(Fonts::default(), &platform(&data.0), data.resume())
        .unwrap_or_else(|e| panic!("reprise impossible : {}", e.one_line()));
    Mutex::new(ws)
}

fn view(ws: &Mutex<Workspace>) -> WorkspaceView {
    Workspace::lock(ws).view()
}

fn send(ws: &Mutex<Workspace>, v: serde_json::Value) {
    let req: WorkspaceRequest = serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"));
    Workspace::apply(ws, req).unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line()));
}

fn tab(ws: &Mutex<Workspace>, id: &str, v: serde_json::Value) -> serde_json::Value {
    let req: Request = serde_json::from_value(v).unwrap_or_else(|e| panic!("requête invalide : {e}"));
    let reply = Workspace::request(ws, id, req).0.unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line()));
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réponse illisible : {e}"))
}

/// Un vrai `.oku-slides.pdf` : un deck JSON n'a pas de fichier associé.
fn saved_pdf(data: &Data, name: &str) -> PathBuf {
    let mut s = oku_slides_session::Session::new(Fonts::default(), &platform(&data.0))
        .unwrap_or_else(|e| panic!("session impossible : {}", e.one_line()));
    s.open(&fixture()).unwrap_or_else(|e| panic!("fixture illisible : {}", e.one_line()));
    let path = data.0.join(format!("{name}.oku-slides.pdf"));
    s.save(Some(&path)).unwrap_or_else(|e| panic!("enregistrement impossible : {}", e.one_line()));
    path
}

fn set_theme(theme: &str) -> serde_json::Value {
    json!({"cmd": "apply", "label": "Thème", "ops": [{"op": "set_theme", "theme": theme}]})
}

fn fixture() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    names.sort();
    names
}

fn write_layout(data: &Data, layout: serde_json::Value) {
    let bytes = serde_json::to_vec(&layout).unwrap_or_else(|e| panic!("layout : {e}"));
    std::fs::write(data.resume().join("workspace.json"), bytes).unwrap_or_else(|e| panic!("workspace.json : {e}"));
}

#[test]
fn first_launch_writes_layout_and_tab_folder() {
    let data = Data::new();
    let ws = resumed(&data);
    assert_that!(view(&ws).windows).has_length(1);
    assert_that!(data.resume().join("workspace.json").exists()).is_true();
    assert_that!(data.resume().join("tabs/t-1/state.json").exists()).is_true();
}

/// Deux fenêtres, trois onglets dont un en édition de modèles, onglet actif,
/// fenêtre active non dernière, historique : tout revient.
#[test]
fn resume_restores_windows_tabs_focus_mode_and_history() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    tab(&a, "t-2", set_theme("pro"));
    send(&a, json!({"cmd": "new_window"}));
    tab(&a, "t-3", json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_SET}));
    send(&a, json!({"cmd": "activate", "tab": "t-1"}));
    let before = view(&a);
    assert_that!(before.focused.as_str()).is_equal_to("w-1");
    drop(a);
    let b = resumed(&data);
    let after = view(&b);
    assert_that!(after.focused).is_equal_to(&before.focused);
    assert_that!(after.windows).is_equal_to(&before.windows);
    assert_that!(view(&b).windows[1].tabs[0].info.editing_templates).is_true();
    let snap = tab(&b, "t-2", json!({"cmd": "undo"}));
    assert_that!(snap["theme"]).is_equal_to(json!("papier"));
}

#[test]
fn resume_with_corrupt_layout_moves_resume_aside_and_starts_blank() {
    let data = Data::new();
    drop(resumed(&data));
    std::fs::write(data.resume().join("workspace.json"), b"pas du json").unwrap();
    let ws = resumed(&data);
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].tabs[0].info.pristine).is_true();
    let aside: Vec<String> = names(&data.0).into_iter().filter(|n| n.starts_with("resume.failed-")).collect();
    assert_that!(aside).has_length(1);
}

#[test]
fn resume_with_one_corrupt_tab_keeps_the_others_and_its_data() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::write(data.resume().join("tabs/t-2/state.json"), br#"{"version":99}"#).unwrap();
    let ws = resumed(&data);
    let ids: Vec<String> = view(&ws).windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    assert_that!(ids).is_equal_to(vec!["t-1".to_owned(), "t-3".to_owned()]);
    let tabs = names(&data.resume().join("tabs"));
    assert_that!(tabs.iter().filter(|n| n.starts_with("t-2.failed-")).count()).is_equal_to(1);
}

#[test]
fn resume_with_listed_tab_without_folder_drops_it() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::remove_dir_all(data.resume().join("tabs/t-2")).unwrap();
    assert_that!(view(&resumed(&data)).windows[0].tabs).has_length(1);
}

#[test]
fn resume_with_every_tab_lost_opens_one_blank_tab_with_a_fresh_id() {
    let data = Data::new();
    drop(resumed(&data));
    std::fs::remove_dir_all(data.resume().join("tabs/t-1")).unwrap();
    let v = view(&resumed(&data));
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].tabs[0].id.as_str()).is_equal_to("t-2");
}

#[test]
fn resume_removes_orphans_but_keeps_set_aside_and_foreign_folders() {
    let data = Data::new();
    drop(resumed(&data));
    let tabs = data.resume().join("tabs");
    for dir in ["t-7/deck", "t-2.failed-123", "notes"] {
        std::fs::create_dir_all(tabs.join(dir)).unwrap();
    }
    drop(resumed(&data));
    assert_that!(names(&tabs)).is_equal_to(vec!["notes".to_owned(), "t-1".to_owned(), "t-2.failed-123".to_owned()]);
    // Le numéro de l'orphelin n'est jamais réutilisé, et c'est enregistré.
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"}));
    let ids: Vec<String> = view(&ws).windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    assert_that!(ids).is_equal_to(vec!["t-1".to_owned(), "t-8".to_owned()]);
}

/// Fermer le dernier onglet de la dernière fenêtre : rien n'est gardé, le
/// lancement suivant ouvre un onglet vierge neuf.
#[test]
fn resume_after_closing_the_last_tab_opens_one_blank_tab() {
    let data = Data::new();
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "close", "tab": "t-1"}));
    assert_that!(view(&ws).windows).is_empty();
    drop(ws);
    let v = view(&resumed(&data));
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].tabs[0].id.as_str()).is_equal_to("t-2");
    assert_that!(data.resume().join("tabs/t-1").exists()).is_false();
}

/// Un identifiant altéré ne sort jamais du dossier : ni lu, ni supprimé.
#[test]
fn resume_with_tampered_ids_ignores_them_and_touches_nothing_outside() {
    let data = Data::new();
    drop(resumed(&data));
    let victim = data.0.join("victime");
    std::fs::create_dir_all(&victim).unwrap();
    std::fs::write(victim.join("state.json"), br#"{"version":1,"mode":"deck"}"#).unwrap();
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 2, "next_window": 2, "focused": "w-1",
               "windows": [{"label": "w-1", "tabs": ["t-1", "../../victime", "t-1"], "active": "t-1"},
                           {"label": "../x", "tabs": ["t-1"], "active": "t-1"}]}),
    );
    let ws = resumed(&data);
    let v = view(&ws);
    assert_that!(v.windows).has_length(1);
    assert_that!(v.windows[0].tabs).has_length(1);
    send(&ws, json!({"cmd": "close_window", "window": "w-1", "discard": ["t-1"]}));
    assert_that!(victim.join("state.json").exists()).is_true();
}

/// Un identifiant altéré dans `kept` ne sort jamais du dossier non plus : ni
/// lu, ni supprimé, et le dossier voisin qu'il désigne reste intact.
#[test]
fn resume_with_tampered_kept_ids_touches_nothing_outside() {
    let data = Data::new();
    drop(resumed(&data));
    let victim = data.0.join("victime");
    std::fs::create_dir_all(&victim).unwrap();
    std::fs::write(victim.join("state.json"), br#"{"version":1,"mode":"deck"}"#).unwrap();
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 2, "next_window": 2, "focused": "w-1",
               "windows": [{"label": "w-1", "tabs": ["t-1"], "active": "t-1"}],
               "kept": ["../../victime", "t-1", "t-1"]}),
    );
    drop(resumed(&data));
    assert_that!(victim.join("state.json").exists()).is_true();
    let victim_contents = std::fs::read(victim.join("state.json")).unwrap();
    assert_that!(victim_contents).is_equal_to(br#"{"version":1,"mode":"deck"}"#.to_vec());
    assert_that!(victim.exists()).is_true();
}

/// Des compteurs trop bas ne donnent jamais un identifiant déjà pris.
#[test]
fn resume_with_low_counters_never_reuses_an_id() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 1, "next_window": 1, "focused": "w-1",
               "windows": [{"label": "w-1", "tabs": ["t-1", "t-2"], "active": "t-1"}]}),
    );
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_window"}));
    let v = view(&ws);
    assert_that!(v.windows[1].label.as_str()).is_equal_to("w-2");
    assert_that!(v.windows[1].tabs[0].id.as_str()).is_equal_to("t-3");
}

/// Des compteurs trop hauts (fichier altéré) sont ramenés au plus grand
/// identifiant relu : les identifiants restent valides.
#[test]
fn resume_with_huge_counters_resets_them() {
    let data = Data::new();
    drop(resumed(&data));
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 18446744073709551615u64, "next_window": 18446744073709551615u64,
               "focused": "w-1", "windows": [{"label": "w-1", "tabs": ["t-1"], "active": "t-1"}]}),
    );
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_window"}));
    let v = view(&ws);
    assert_that!(v.windows[1].label.as_str()).is_equal_to("w-2");
    assert_that!(v.windows[1].tabs[0].id.as_str()).is_equal_to("t-2");
}

/// Un onglet cité mais illisible (ici : sans `state.json`) garde son dossier,
/// ce lancement-ci et les suivants : il peut contenir du travail.
#[test]
fn resume_with_cited_unrestorable_tab_keeps_its_folder_across_launches() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::remove_file(data.resume().join("tabs/t-2/state.json")).unwrap();
    for _ in 0..2 {
        let ws = resumed(&data);
        assert_that!(view(&ws).windows[0].tabs).has_length(1);
        drop(ws);
        assert_that!(data.resume().join("tabs/t-2/deck/document.json").exists()).is_true();
    }
}

/// Les onglets d'une fenêtre refusée (label invalide ou en double) restent
/// cités : leurs dossiers ne sont pas supprimés.
#[test]
fn resume_with_rejected_window_keeps_the_folders_of_its_tabs() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_window"}));
    drop(a);
    write_layout(
        &data,
        json!({"version": 2, "next_tab": 3, "next_window": 3, "focused": "w-1",
               "windows": [{"label": "w-1", "tabs": ["t-1"], "active": "t-1"},
                           {"label": "w-1", "tabs": ["t-2"], "active": "t-2"}]}),
    );
    drop(resumed(&data));
    drop(resumed(&data));
    assert_that!(data.resume().join("tabs/t-2/state.json").exists()).is_true();
}

#[test]
fn close_tab_and_close_window_remove_their_folders() {
    let data = Data::new();
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_tab", "window": "w-1"}));
    send(&ws, json!({"cmd": "new_window"}));
    send(&ws, json!({"cmd": "close", "tab": "t-2"}));
    send(&ws, json!({"cmd": "close_window", "window": "w-2"}));
    assert_that!(names(&data.resume().join("tabs"))).is_equal_to(vec!["t-1".to_owned()]);
    drop(ws);
    assert_that!(view(&resumed(&data)).windows[0].tabs).has_length(1);
}

/// `focus` seul n'écrit rien : `settle` force l'écriture avant de quitter.
#[test]
fn settle_persists_a_focus_change_that_focus_alone_does_not_write() {
    let data = Data::new();
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_window"})); // enregistré : focus w-2.
    Workspace::lock(&ws).focus("w-1").unwrap_or_else(|e| panic!("focus impossible : {}", e.one_line()));
    // Rien n'est encore écrit pour ce focus retrouvé sur `w-1`.
    Workspace::settle(&ws);
    drop(ws);
    assert_that!(view(&resumed(&data)).focused.as_str()).is_equal_to("w-1");
}

#[test]
fn open_files_same_file_twice_opens_one_tab_in_the_focused_window() {
    let data = Data::new();
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "new_window"}));
    tab(&ws, "t-2", set_theme("pro"));
    let file = saved_pdf(&data, "essai");
    Workspace::open_files(&ws, &[file.clone(), file]);
    let v = view(&ws);
    assert_that!(v.windows[0].tabs).has_length(1);
    assert_that!(v.windows[1].tabs).has_length(2);
    assert_that!(v.windows[1].active.as_str()).is_equal_to("t-3");
}

#[test]
fn open_files_unreadable_file_sets_a_notice_once_and_opens_the_next() {
    let data = Data::new();
    let ws = resumed(&data);
    tab(&ws, "t-1", set_theme("pro"));
    Workspace::open_files(&ws, &[data.0.join("absent.oku-slides.pdf"), fixture()]);
    let w = &view(&ws).windows[0];
    assert_that!(w.tabs).has_length(2);
    let first = tab(&ws, "t-1", json!({"cmd": "snapshot"}));
    let notice = first["notice"].as_str().unwrap_or_else(|| panic!("avis attendu : {first}"));
    assert_that!(notice.contains("absent.oku-slides.pdf")).is_true();
    let second = tab(&ws, "t-1", json!({"cmd": "snapshot"}));
    assert_that!(second["notice"].as_str()).is_none();
}

#[test]
fn open_files_after_settle_opens_nothing() {
    let data = Data::new();
    let ws = resumed(&data);
    let file = saved_pdf(&data, "essai");
    Workspace::settle(&ws);
    Workspace::open_files(&ws, &[file]);
    assert_that!(view(&ws).windows[0].tabs).has_length(1);
}

#[test]
fn apply_open_tab_after_settle_is_refused() {
    let data = Data::new();
    let ws = resumed(&data);
    Workspace::settle(&ws);
    let window = view(&ws).windows[0].label.clone();
    let req = WorkspaceRequest::OpenTab { window, path: saved_pdf(&data, "essai") };
    let err = Workspace::apply(&ws, req).unwrap_err();
    assert_that!(err.kind()).is_equal_to(oku_slides_session::SessionErrorKind::UnknownWindow);
}
