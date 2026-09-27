//! Reprise de l'espace de travail : fenêtres, onglets, et chaque session.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_session::{
    Fonts, Platform, Request, SessionErrorKind, Settings, Workspace, WorkspaceRequest, WorkspaceView,
};
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
/// fenêtre active non dernière, deck : tout revient sauf la pile
/// d'annulation (perdue au relancement, t-188 / ADR-0002).
#[test]
fn resume_restores_windows_tabs_focus_mode_and_deck_without_undo() {
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
    // Le deck revient tel quel ; la pile d'annulation ne survit pas au relancement.
    let snap = tab(&b, "t-2", json!({"cmd": "undo"}));
    assert_that!(snap["theme"]).is_equal_to(json!("pro"));
    assert_that!(snap["can_undo"]).is_equal_to(json!(false));
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

/// t-158 : un onglet n'est relu qu'à sa première requête. Illisible, il
/// reste à sa place : son état est mis de côté, il repart d'un deck vierge
/// et le dit ; les autres onglets n'en savent rien.
#[test]
fn resume_with_one_corrupt_tab_sets_it_aside_at_its_first_request() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::write(data.resume().join("tabs/t-2/state.json"), br#"{"version":99}"#).unwrap();
    let ws = resumed(&data);
    let ids: Vec<String> = view(&ws).windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    assert_that!(ids).is_equal_to(vec!["t-1".to_owned(), "t-2".to_owned(), "t-3".to_owned()]);
    let aside =
        |data: &Data| names(&data.resume().join("tabs")).iter().filter(|n| n.starts_with("t-2.failed-")).count();
    assert_that!(aside(&data)).is_equal_to(0);
    let snap = tab(&ws, "t-2", json!({"cmd": "snapshot"}));
    let expected = platform(&data.0).localizer.tr("notice-tab-resume-failed");
    assert_that!(snap["notice"].as_str()).is_equal_to(Some(expected.as_str()));
    assert_that!(aside(&data)).is_equal_to(1);
    assert_that!(data.resume().join("tabs/t-2/state.json").exists()).is_true();
    assert_that!(view(&ws).windows[0].tabs[1].info.pristine).is_true();
}

/// t-158 : au lancement, la barre montre les onglets tels qu'enregistrés,
/// sans relire leur session (un état illisible n'y change rien).
#[test]
fn resume_shows_saved_tab_titles_without_reading_the_tabs() {
    let data = Data::new();
    let file = saved_pdf(&data, "rapport");
    let a = resumed(&data);
    send(&a, json!({"cmd": "open_tab", "window": "w-1", "path": file}));
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::write(data.resume().join("tabs/t-2/state.json"), br#"{"version":99}"#).unwrap();
    let v = view(&resumed(&data));
    let t2 = v.windows[0].tabs.iter().find(|t| t.id == "t-2").unwrap_or_else(|| panic!("t-2 absent : {v:?}"));
    assert_that!(t2.info.title.as_str()).is_equal_to("rapport");
    assert_that!(t2.info.path.as_deref()).is_equal_to(Some(oku_slides_platform::storage::resolve(&file).as_path()));
}

/// t-158 : un onglet inactif revient entier à sa première requête.
#[test]
fn resume_reads_an_inactive_tab_at_its_first_request() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    tab(&a, "t-2", set_theme("pro"));
    send(&a, json!({"cmd": "activate", "tab": "t-1"}));
    drop(a);
    let b = resumed(&data);
    assert_that!(view(&b).windows[0].tabs[1].info.dirty).is_true();
    assert_that!(tab(&b, "t-2", json!({"cmd": "snapshot"}))["theme"]).is_equal_to(json!("pro"));
}

/// t-158 : les infos d'onglet sont enregistrées dès qu'elles changent, pas
/// seulement avec la structure : un arrêt brutal garde le bon titre.
#[test]
fn request_that_changes_a_tab_records_its_info_in_workspace_json() {
    let data = Data::new();
    let a = resumed(&data);
    tab(&a, "t-1", set_theme("pro"));
    let bytes = std::fs::read(data.resume().join("workspace.json")).unwrap();
    let layout: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_that!(layout["info"]["t-1"]["dirty"]).is_equal_to(json!(true));
}

/// `workspace.json` n'est pas digne de confiance : un onglet sans infos
/// enregistrées est relu au lancement, comme avant ; illisible, il est retiré.
#[test]
fn resume_without_saved_info_reads_the_tab_at_launch() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    let bytes = std::fs::read(data.resume().join("workspace.json")).unwrap();
    let mut layout: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    layout["info"].as_object_mut().unwrap().remove("t-2");
    write_layout(&data, layout);
    std::fs::write(data.resume().join("tabs/t-2/state.json"), br#"{"version":99}"#).unwrap();
    let ids: Vec<String> = view(&resumed(&data)).windows[0].tabs.iter().map(|t| t.id.clone()).collect();
    assert_that!(ids).is_equal_to(vec!["t-1".to_owned(), "t-3".to_owned()]);
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

/// t-158 : fermer un onglet endormi et modifié pose la question (l'onglet
/// est relu d'abord : ses infos enregistrées peuvent être en retard).
#[test]
fn close_of_a_dormant_modified_tab_is_refused_until_discarded() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    tab(&a, "t-2", set_theme("pro"));
    send(&a, json!({"cmd": "activate", "tab": "t-1"}));
    drop(a);
    let b = resumed(&data);
    let close = |discard: bool| {
        let req: WorkspaceRequest = serde_json::from_value(json!({"cmd": "close", "tab": "t-2", "discard": discard}))
            .unwrap_or_else(|e| panic!("requête invalide : {e}"));
        Workspace::apply(&b, req)
    };
    let refused = close(false).err().map(|e| e.kind());
    assert_that!(refused).is_equal_to(Some(SessionErrorKind::Unsaved));
    assert_that!(close(true).is_ok()).is_true();
    assert_that!(view(&b).windows[0].tabs).has_length(1);
}

/// Infos enregistrées en retard (arrêt brutal entre l'état de l'onglet et
/// `workspace.json`) : fermer relit l'onglet et pose quand même la question.
#[test]
fn close_of_a_dormant_tab_with_stale_clean_info_is_refused() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    tab(&a, "t-2", set_theme("pro"));
    send(&a, json!({"cmd": "activate", "tab": "t-1"}));
    drop(a);
    let bytes = std::fs::read(data.resume().join("workspace.json")).unwrap_or_else(|e| panic!("layout : {e}"));
    let mut layout: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("layout : {e}"));
    layout["info"]["t-2"]["dirty"] = json!(false);
    write_layout(&data, layout);
    let b = resumed(&data);
    let req: WorkspaceRequest = serde_json::from_value(json!({"cmd": "close", "tab": "t-2"}))
        .unwrap_or_else(|e| panic!("requête invalide : {e}"));
    let refused = Workspace::apply(&b, req).err().map(|e| e.kind());
    assert_that!(refused).is_equal_to(Some(SessionErrorKind::Unsaved));
    assert_that!(data.resume().join("tabs/t-2/state.json").exists()).is_true();
}

/// Une entrée `info` malformée ne coûte que son onglet (relu au lancement),
/// pas tout l'espace de travail.
#[test]
fn resume_with_one_malformed_tab_info_reads_that_tab_and_keeps_the_rest() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    let bytes = std::fs::read(data.resume().join("workspace.json")).unwrap_or_else(|e| panic!("layout : {e}"));
    let mut layout: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("layout : {e}"));
    layout["info"]["t-2"] = json!({"title": 42});
    write_layout(&data, layout);
    let v = view(&resumed(&data));
    assert_that!(v.windows[0].tabs).has_length(2);
    let aside: Vec<String> = names(&data.0).into_iter().filter(|n| n.starts_with("resume.failed-")).collect();
    assert_that!(aside).is_empty();
}

/// `workspace.json` d'avant t-158, sans `info` : les onglets sont relus au
/// lancement, avec leur titre.
#[test]
fn resume_of_a_layout_without_info_restores_every_tab() {
    let data = Data::new();
    let file = saved_pdf(&data, "bilan");
    let a = resumed(&data);
    send(&a, json!({"cmd": "open_tab", "window": "w-1", "path": file}));
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    let bytes = std::fs::read(data.resume().join("workspace.json")).unwrap_or_else(|e| panic!("layout : {e}"));
    let mut layout: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("layout : {e}"));
    layout.as_object_mut().unwrap_or_else(|| panic!("objet attendu")).remove("info");
    write_layout(&data, layout);
    let v = view(&resumed(&data));
    let titles: Vec<&str> = v.windows[0].tabs.iter().map(|t| t.info.title.as_str()).collect();
    assert_that!(titles.contains(&"bilan")).is_true();
    assert_that!(v.windows[0].tabs).has_length(2);
}

/// Relancé sur un seul onglet vierge, un fichier ouvert le remplace encore :
/// l'onglet endormi, enregistré vierge, est relu pour s'en assurer.
#[test]
fn open_tab_after_resume_replaces_the_restored_blank_tab() {
    let data = Data::new();
    drop(resumed(&data));
    let file = saved_pdf(&data, "note");
    let ws = resumed(&data);
    send(&ws, json!({"cmd": "open_tab", "window": "w-1", "path": file}));
    let v = view(&ws);
    assert_that!(v.windows[0].tabs).has_length(1);
    assert_that!(v.windows[0].tabs[0].info.title.as_str()).is_equal_to("note");
}

/// Fermer la seule fenêtre quitte l'app en gardant tout : ses onglets
/// endormis ne sont pas relus pour rien (un état illisible reste en place).
#[test]
fn close_window_of_the_only_window_leaves_dormant_tabs_asleep() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    std::fs::write(data.resume().join("tabs/t-2/state.json"), br#"{"version":99}"#)
        .unwrap_or_else(|e| panic!("state.json : {e}"));
    let b = resumed(&data);
    send(&b, json!({"cmd": "close_window", "window": "w-1"}));
    let aside = names(&data.resume().join("tabs")).into_iter().filter(|n| n.starts_with("t-2.failed-")).count();
    assert_that!(aside).is_equal_to(0);
}

/// Un champ `info` qui n'est pas un objet ne coûte que les infos : les
/// onglets sont relus au lancement, rien n'est mis de côté.
#[test]
fn resume_with_info_that_is_not_an_object_reads_the_tabs() {
    let data = Data::new();
    let a = resumed(&data);
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    let bytes = std::fs::read(data.resume().join("workspace.json")).unwrap_or_else(|e| panic!("layout : {e}"));
    let mut layout: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("layout : {e}"));
    layout["info"] = json!([1, 2]);
    write_layout(&data, layout);
    assert_that!(view(&resumed(&data)).windows[0].tabs).has_length(2);
    let aside: Vec<String> = names(&data.0).into_iter().filter(|n| n.starts_with("resume.failed-")).collect();
    assert_that!(aside).is_empty();
}

/// t-143 : au lancement, seules les `keep_failed` (3) dernières reprises
/// mises de côté restent, à la racine comme parmi les onglets ; un dossier
/// étranger n'est jamais touché.
#[test]
fn resume_prunes_old_failed_folders_beyond_the_setting() {
    let data = Data::new();
    drop(resumed(&data));
    for secs in [10, 20, 30, 40] {
        std::fs::create_dir_all(data.0.join(format!("resume.failed-{secs}"))).unwrap_or_else(|e| panic!("{e}"));
        std::fs::create_dir_all(data.resume().join(format!("tabs/t-9.failed-{secs}")))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    std::fs::create_dir_all(data.0.join("resume.failed-notes")).unwrap_or_else(|e| panic!("{e}"));
    drop(resumed(&data));
    let root: Vec<String> = names(&data.0).into_iter().filter(|n| n.starts_with("resume.failed-")).collect();
    assert_that!(root).is_equal_to(vec![
        "resume.failed-20".to_owned(),
        "resume.failed-30".to_owned(),
        "resume.failed-40".to_owned(),
        "resume.failed-notes".to_owned(),
    ]);
    let tabs: Vec<String> = names(&data.resume().join("tabs")).into_iter().filter(|n| n.contains(".failed-")).collect();
    assert_that!(tabs).is_equal_to(vec![
        "t-9.failed-20".to_owned(),
        "t-9.failed-30".to_owned(),
        "t-9.failed-40".to_owned(),
    ]);
}

/// Le réglage `resume.keep_failed` fait foi (ici 1), et une reprise mise de
/// côté pendant ce lancement reste jusqu'au suivant, même à 0.
#[test]
fn resume_prunes_with_the_configured_count_and_spares_this_launch() {
    let data = Data::new();
    drop(resumed(&data));
    for secs in [10, 20] {
        std::fs::create_dir_all(data.0.join(format!("resume.failed-{secs}"))).unwrap_or_else(|e| panic!("{e}"));
    }
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("{e}"));
    settings.paths.data = Some(data.0.clone());
    settings.resume.keep_failed = 0;
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(data.resume().join("workspace.json"), b"pas du json").unwrap_or_else(|e| panic!("{e}"));
    drop(Workspace::resume(Fonts::default(), &platform, data.resume()).unwrap_or_else(|e| panic!("{}", e.one_line())));
    let root: Vec<String> = names(&data.0).into_iter().filter(|n| n.starts_with("resume.failed-")).collect();
    assert_that!(root).has_length(1);
    assert_that!(root.iter().any(|n| n == "resume.failed-10" || n == "resume.failed-20")).is_false();
}

/// t-145 : au relancement, ouvrir le fichier d'un onglet en mode modèles
/// active cet onglet (son fichier reste celui du deck mis de côté), sans
/// doublon, et il reste en mode modèles.
#[test]
fn open_tab_of_a_file_whose_tab_edits_templates_activates_it() {
    let data = Data::new();
    let file = saved_pdf(&data, "atelier");
    let a = resumed(&data);
    send(&a, json!({"cmd": "open_tab", "window": "w-1", "path": file}));
    let id = view(&a).windows[0].active.clone();
    tab(&a, &id, json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_SET}));
    send(&a, json!({"cmd": "new_tab", "window": "w-1"}));
    drop(a);
    // Relancé : l'onglet dort (t-158), reconnu d'après ses infos enregistrées.
    let b = resumed(&data);
    send(&b, json!({"cmd": "open_tab", "window": "w-1", "path": file}));
    let v = view(&b);
    assert_that!(v.windows[0].tabs).has_length(2);
    assert_that!(v.windows[0].active.as_str()).is_equal_to(id.as_str());
    assert_that!(tab(&b, &id, json!({"cmd": "snapshot"}))["editing"]["set"])
        .is_equal_to(json!(oku_slides_session::DEFAULT_SET));
}

/// Un `OpenTab` de `path` dans la première fenêtre de la vue.
fn open_in_first_window(ws: &Mutex<Workspace>, path: &Path) {
    let window = view(ws).windows[0].label.clone();
    send(ws, json!({"cmd": "open_tab", "window": window, "path": path}));
}

/// La réponse de `SameDeck` pour `path`.
fn same_deck(ws: &Mutex<Workspace>, path: &Path) -> serde_json::Value {
    let reply = Workspace::apply(ws, WorkspaceRequest::SameDeck { path: path.to_owned() })
        .unwrap_or_else(|e| panic!("requête refusée : {}", e.one_line()))
        .into_json();
    serde_json::from_str(&reply).unwrap_or_else(|e| panic!("réponse illisible : {e}"))
}

/// L'avis montré avec le snapshot de l'onglet actif de la fenêtre active.
fn notice_of_active_tab(ws: &Mutex<Workspace>) -> Option<String> {
    let v = view(ws);
    let win = v.windows.iter().find(|w| w.label == v.focused).unwrap_or_else(|| panic!("fenêtre active absente"));
    tab(ws, &win.active, json!({"cmd": "snapshot"}))["notice"].as_str().map(str::to_owned)
}

/// Une copie de `original` à côté, sous un autre nom.
fn copy_of(data: &Data, original: &Path) -> PathBuf {
    let copy = data.0.join("copie.oku-slides.pdf");
    std::fs::copy(original, &copy).unwrap_or_else(|e| panic!("copie impossible : {e}"));
    copy
}

#[test]
fn same_deck_finds_the_tab_holding_that_deck() {
    let data = Data::new();
    let ws = resumed(&data);
    let original = saved_pdf(&data, "deck");
    open_in_first_window(&ws, &original);
    let id = view(&ws).windows[0].active.clone();
    let v = same_deck(&ws, &copy_of(&data, &original));
    assert_that!(v["tab"].as_str()).is_equal_to(Some(id.as_str()));
    assert_that!(v["title"].as_str()).is_equal_to(Some("deck"));
}

#[test]
fn same_deck_for_the_very_same_path_answers_none() {
    // OpenTab active déjà l'onglet de ce chemin : pas de question.
    let data = Data::new();
    let ws = resumed(&data);
    let original = saved_pdf(&data, "deck");
    open_in_first_window(&ws, &original);
    let v = same_deck(&ws, &original);
    assert_that!(v["tab"].is_null()).is_true();
    assert_that!(v["title"].is_null()).is_true();
}

#[test]
fn same_deck_for_another_deck_answers_none() {
    let data = Data::new();
    let ws = resumed(&data);
    let a = saved_pdf(&data, "a");
    let b = saved_pdf(&data, "b");
    open_in_first_window(&ws, &a);
    assert_that!(same_deck(&ws, &b)["tab"].is_null()).is_true();
}

#[test]
fn same_deck_for_an_unreadable_file_is_an_error() {
    let data = Data::new();
    let ws = resumed(&data);
    let absent = data.0.join("absent.oku-slides.pdf");
    let reply = Workspace::apply(&ws, WorkspaceRequest::SameDeck { path: absent });
    assert_that!(reply.is_err()).is_true();
}

#[test]
fn same_deck_after_resume_finds_the_dormant_tab() {
    let data = Data::new();
    let original = saved_pdf(&data, "deck");
    let id = {
        let ws = resumed(&data);
        open_in_first_window(&ws, &original);
        view(&ws).windows[0].active.clone()
    };
    let ws = resumed(&data);
    assert_that!(same_deck(&ws, &copy_of(&data, &original))["tab"].as_str()).is_equal_to(Some(id.as_str()));
}

#[test]
fn tab_info_keeps_deck_uid_across_resume() {
    let data = Data::new();
    let original = saved_pdf(&data, "deck");
    {
        let ws = resumed(&data);
        open_in_first_window(&ws, &original);
    }
    let ws = resumed(&data);
    let v = view(&ws);
    let uids: Vec<_> = v.windows[0].tabs.iter().filter_map(|t| t.info.deck_uid.clone()).collect();
    assert_that!(uids).has_length(1);
}

#[test]
fn open_files_of_a_deck_already_open_opens_apart_with_a_notice() {
    let data = Data::new();
    let ws = resumed(&data);
    let original = saved_pdf(&data, "deck");
    open_in_first_window(&ws, &original);
    Workspace::open_files(&ws, &[copy_of(&data, &original)]);
    assert_that!(view(&ws).windows[0].tabs).has_length(2);
    // Le nouvel onglet porte l'avis qui désigne l'onglet déjà ouvert.
    let notice = notice_of_active_tab(&ws).unwrap_or_else(|| panic!("avis attendu"));
    assert_that!(notice.contains("« deck »")).is_true();
}

#[test]
fn open_files_of_another_deck_sets_no_notice() {
    let data = Data::new();
    let ws = resumed(&data);
    let a = saved_pdf(&data, "a");
    let b = saved_pdf(&data, "b");
    open_in_first_window(&ws, &a);
    Workspace::open_files(&ws, &[b]);
    assert_that!(view(&ws).windows[0].tabs).has_length(2);
    assert_that!(notice_of_active_tab(&ws)).is_none();
}

/// Le fichier touché par un autre outil : même deck, avis
/// `notice-modified-elsewhere` à l'ouverture (la date `/ModDate` change, à
/// longueur égale).
fn touch_elsewhere(path: &Path) {
    let mut bytes = std::fs::read(path).unwrap_or_else(|e| panic!("lecture : {e}"));
    let at = bytes.windows(8).position(|w| w == b"/ModDate").unwrap_or_else(|| panic!("/ModDate absent"));
    let date = at + bytes[at..].windows(2).position(|w| w == b"D:").unwrap_or_else(|| panic!("date absente")) + 2;
    bytes[date..date + 4].copy_from_slice(b"2099");
    std::fs::write(path, bytes).unwrap_or_else(|e| panic!("écriture : {e}"));
}

/// L'avis posé par l'ouverture et celui du deck déjà ouvert s'ajoutent.
#[test]
fn open_files_of_a_copy_with_its_own_notice_keeps_both_notices() {
    let data = Data::new();
    let ws = resumed(&data);
    let original = saved_pdf(&data, "deck");
    open_in_first_window(&ws, &original);
    let copy = copy_of(&data, &original);
    touch_elsewhere(&copy);
    Workspace::open_files(&ws, &[copy]);
    let notice = notice_of_active_tab(&ws).unwrap_or_else(|| panic!("avis attendu"));
    let parts: Vec<&str> = notice.split("\n\n").collect();
    assert_that!(parts).has_length(2);
    assert_that!(parts[0]).contains("autre outil");
    assert_that!(parts[1]).contains("« deck »");
}

#[test]
fn same_deck_skips_a_tab_editing_templates() {
    let data = Data::new();
    let ws = resumed(&data);
    let original = saved_pdf(&data, "deck");
    open_in_first_window(&ws, &original);
    let id = view(&ws).windows[0].active.clone();
    tab(&ws, &id, json!({"cmd": "edit_templates", "set": oku_slides_session::DEFAULT_SET}));
    assert_that!(same_deck(&ws, &copy_of(&data, &original))["tab"].is_null()).is_true();
}

/// Un `workspace.json` d'avant `deck_uid` : l'onglet reprend, endormi, sans
/// `deck_uid` (lu comme absent, pas comme une entrée malformée relue).
#[test]
fn resume_with_tab_info_without_deck_uid_reads_it_as_none() {
    let data = Data::new();
    let original = saved_pdf(&data, "deck");
    let id = {
        let ws = resumed(&data);
        open_in_first_window(&ws, &original);
        view(&ws).windows[0].active.clone()
    };
    let bytes = std::fs::read(data.resume().join("workspace.json")).unwrap_or_else(|e| panic!("layout : {e}"));
    let mut layout: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("layout : {e}"));
    let info = layout["info"][&id].as_object_mut().unwrap_or_else(|| panic!("infos de {id} absentes"));
    assert_that!(info.remove("deck_uid").is_some()).is_true();
    write_layout(&data, layout);
    let v = view(&resumed(&data));
    let t = v.windows[0].tabs.iter().find(|t| t.id == id).unwrap_or_else(|| panic!("onglet {id} absent"));
    assert_that!(t.info.deck_uid).is_none();
    assert_that!(t.info.title.as_str()).is_equal_to("deck");
}
