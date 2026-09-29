//! t-224 : les modifications de l'agent — `Session::apply`, `Session::open`,
//! `Session::new_deck`, appelés directement par `Workspace::with_session`
//! (voir `oku-slides-mcp`), hors de `Session::handle` — n'étaient jamais
//! écrites pour la reprise. Un crash après un tour de l'agent perdait le
//! travail qu'il venait de faire.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_core::Op;
use oku_slides_session::{Fonts, Platform, Session, Settings, Workspace};
use speculoos::prelude::*;

/// Un dossier de données neuf, supprimé à la fin du test.
struct Data(PathBuf);

impl Data {
    /// Créé atomiquement sous un nom aléatoire (t-264) : jamais le dossier
    /// d'un lancement précédent, même quand macOS recycle le PID.
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("oku-slides-issue-0224-")
            .tempdir()
            .unwrap_or_else(|e| panic!("dossier de données impossible : {e}"));
        Self(dir.keep())
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

fn platform(data: &Path) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages par défaut invalides : {e}"));
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme indisponible : {e}"))
}

fn resumed(data: &Data) -> Mutex<Workspace> {
    Mutex::new(
        Workspace::resume(Fonts::default(), &platform(&data.0), data.resume())
            .unwrap_or_else(|e| panic!("reprise impossible : {}", e.one_line())),
    )
}

#[test]
fn agent_apply_outside_handle_is_written_for_resume() {
    let data = Data::new();
    let ws = resumed(&data);
    let tab_id = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();

    // L'agent modifie la session directement, sans passer par `handle`.
    let (applied, _) = Workspace::with_session(&ws, &tab_id, |s: &mut Session| {
        s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }], None)
    });
    let applied = applied.unwrap_or_else(|e| panic!("onglet inconnu : {}", e.one_line()));
    assert_that!(applied.map(|_| ())).is_ok();

    Workspace::settle(&ws);
    drop(ws);

    // Une reprise ultérieure retrouve la modification de l'agent.
    let again = resumed(&data);
    let (theme, _) = Workspace::with_session(&again, &tab_id, |s: &mut Session| s.deck().theme.clone());
    let theme = theme.unwrap_or_else(|e| panic!("onglet inconnu après reprise : {}", e.one_line()));
    assert_that!(theme.as_deref()).is_equal_to(Some("pro"));
}

fn fixture() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))
}

#[test]
fn agent_open_outside_handle_is_written_for_resume() {
    let data = Data::new();
    let ws = resumed(&data);
    let tab_id = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();

    let (opened, _) = Workspace::with_session(&ws, &tab_id, |s: &mut Session| s.open(&fixture()));
    let opened = opened.unwrap_or_else(|e| panic!("onglet inconnu : {}", e.one_line()));
    assert_that!(opened.map_err(|e| e.one_line())).is_ok();

    Workspace::settle(&ws);
    drop(ws);

    let again = resumed(&data);
    let (found, _) = Workspace::with_session(&again, &tab_id, |s: &mut Session| s.deck().slide("s-constat").is_some());
    let found = found.unwrap_or_else(|e| panic!("onglet inconnu après reprise : {}", e.one_line()));
    assert_that!(found).is_true();
}

#[test]
fn agent_new_deck_outside_handle_is_written_for_resume() {
    let data = Data::new();
    let ws = resumed(&data);
    let tab_id = Workspace::lock(&ws).view().windows[0].tabs[0].id.clone();

    // Un document modifié, puis remplacé par un deck vierge par l'agent.
    let (applied, _) = Workspace::with_session(&ws, &tab_id, |s: &mut Session| {
        s.apply("Thème", vec![Op::SetTheme { theme: Some("pro".into()) }], None)
    });
    let applied = applied.unwrap_or_else(|e| panic!("onglet inconnu : {}", e.one_line()));
    assert_that!(applied.map(|_| ())).is_ok();
    let (blank, _) = Workspace::with_session(&ws, &tab_id, |s: &mut Session| s.new_deck());
    let blank = blank.unwrap_or_else(|e| panic!("onglet inconnu : {}", e.one_line()));
    assert_that!(blank.map_err(|e| e.one_line())).is_ok();

    Workspace::settle(&ws);
    drop(ws);

    let again = resumed(&data);
    let (theme, _) = Workspace::with_session(&again, &tab_id, |s: &mut Session| s.deck().theme.clone());
    let theme = theme.unwrap_or_else(|e| panic!("onglet inconnu après reprise : {}", e.one_line()));
    // Le deck vierge, pas le document modifié qu'il a remplacé.
    assert_that!(theme.as_deref()).is_not_equal_to(Some("pro"));
}
