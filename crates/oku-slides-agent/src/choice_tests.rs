use std::path::PathBuf;

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::config::{AgentPreset, Builtin};
use oku_slides_platform::local_agent::{AGENT_MODEL_SIZE, Target, Unavailable, agent_model_dir};
use oku_slides_session::{AgentPresetView, LocalModelView, Platform, Settings};
use speculoos::prelude::*;

use super::{AgentChoice, Refusal};
use crate::error::Unusable;
use crate::{Launch, Link};

/// Une plateforme sur `data`, l'agent « local » choisi ; `program` : le binaire de l'app.
fn platform(data: &std::path::Path, program: Option<PathBuf>) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    settings.agent.selected = "local".into();
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    match program {
        Some(p) => platform.with_program(p),
        None => platform,
    }
}

fn choice(platform: &Platform) -> AgentChoice {
    let launch = Launch {
        command: String::new(),
        args: Vec::new(),
        env: Vec::new(),
        fallback_cwd: platform.settings.paths.data(),
        ready_within: std::time::Duration::from_secs(1),
    };
    AgentChoice::new(platform, Link::new(launch))
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

/// Ce que dit `availability` d'un modèle absent sur cette machine.
fn expected_without_model() -> Option<Unusable> {
    match Target::current() {
        Target::AppleSilicon => {
            Some(Unavailable::NeedsDownload { size: oku_slides_platform::local_agent::AGENT_MODEL_SIZE }.into())
        }
        Target::IntelMac => Some(Unusable::LocalIntelMac),
        Target::Other => Some(Unusable::LocalUnsupportedOs),
    }
}

#[test]
fn availability_local_without_model_needs_download() {
    let dir = tempfile::tempdir().unwrap();
    let platform = platform(dir.path(), Some(PathBuf::from("/app")));
    let expected = expected_without_model().map(|u| u.localize(&fr()));
    assert_that!(choice(&platform).availability()).is_equal_to(expected);
}

#[test]
fn availability_local_with_complete_marker_is_ready() {
    let dir = tempfile::tempdir().unwrap();
    let platform = platform(dir.path(), Some(PathBuf::from("/app")));
    let models = agent_model_dir(&platform.settings);
    std::fs::create_dir_all(&models).unwrap();
    std::fs::write(models.join(oku_slides_platform::models::COMPLETE), b"").unwrap();
    let expected = match Target::current() {
        Target::AppleSilicon => None,
        _ => expected_without_model().map(|u| u.localize(&fr())),
    };
    assert_that!(choice(&platform).availability()).is_equal_to(expected);
}

#[test]
fn availability_after_successful_download_forgets_a_stale_link_failure() {
    let dir = tempfile::tempdir().unwrap();
    let platform = platform(dir.path(), Some(PathBuf::from("/app")));
    let models = agent_model_dir(&platform.settings);
    std::fs::create_dir_all(&models).unwrap();
    std::fs::write(models.join(oku_slides_platform::models::COMPLETE), b"").unwrap();
    let choice = choice(&platform);
    // Un prompt lancé avant le téléchargement : le fils est sorti « indisponible ».
    choice.link.remember_at(0, Some(Unusable::NoHttp), std::time::Instant::now());
    let stale = Some(Unusable::NoHttp.localize(&fr()));
    let expected = match Target::current() {
        Target::AppleSilicon => None,
        _ => expected_without_model().map(|u| u.localize(&fr())),
    };
    if Target::current() == Target::AppleSilicon {
        assert_that!(choice.availability()).is_equal_to(stale);
    }
    choice.download_changed();
    assert_that!(choice.availability()).is_equal_to(expected);
}

#[cfg(debug_assertions)]
#[test]
fn availability_local_fake_engine_is_ready() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.path().to_path_buf());
    settings.paths.decks = Some(dir.path().join("decks"));
    settings.agent.selected = "local".into();
    settings.i18n.locale = Some("fr".into());
    settings.local_agent.engine = oku_slides_platform::config::LocalEngine::Fake;
    let platform = Platform::with_settings(settings).unwrap().with_program(PathBuf::from("/app"));
    assert_that!(choice(&platform).availability()).is_none();
}

#[test]
fn availability_local_without_program_is_no_program() {
    let dir = tempfile::tempdir().unwrap();
    let platform = platform(dir.path(), None);
    let expected = Some(Unusable::NoProgram.localize(&fr()));
    assert_that!(choice(&platform).availability()).is_equal_to(expected);
}

#[test]
fn local_panel_command_preset_is_none_and_start_download_refused() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.path().to_path_buf());
    settings.paths.decks = Some(dir.path().join("decks"));
    settings.agent.selected = "claude".into();
    settings.i18n.locale = Some("fr".into());
    let platform = Platform::with_settings(settings).unwrap();
    let choice = std::sync::Arc::new(choice(&platform));
    assert_that!(choice.local_panel()).is_none();
    assert_that!(choice.start_download(|| {})).is_false();
}

#[test]
fn local_panel_local_without_model_offers_the_download_on_apple_silicon() {
    let dir = tempfile::tempdir().unwrap();
    let platform = platform(dir.path(), Some(PathBuf::from("/app")));
    let expected = match Target::current() {
        Target::AppleSilicon => Some(LocalModelView::NeedsDownload { size: AGENT_MODEL_SIZE }),
        _ => None,
    };
    assert_that!(choice(&platform).local_panel()).is_equal_to(expected);
}

/// Une plateforme sur `data` à deux agents, `faux` (choisi) et `bis` ; `locked` : choix imposé.
fn two_agents(data: &std::path::Path, locked: bool) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    settings.agent.presets = [
        ("faux".to_owned(), AgentPreset::new("Faux", "fake", vec!["--script".into(), "s".into()])),
        ("bis".to_owned(), AgentPreset::new("Bis", "true", vec![])),
    ]
    .into_iter()
    .collect();
    settings.agent.selected = "faux".into();
    settings.agent.locked = locked;
    settings.i18n.locale = Some("fr".into());
    Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"))
}

#[test]
fn check_unknown_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let platform = two_agents(dir.path(), false);
    assert_that!(choice(&platform).check("nope").map(|p| p.name.clone())).is_err_containing(Refusal::UnknownAgent);
}

#[test]
fn check_locked_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let platform = two_agents(dir.path(), true);
    assert_that!(choice(&platform).check("bis").map(|p| p.name.clone())).is_err_containing(Refusal::Locked);
}

#[test]
fn switch_to_the_active_agent_keeps_the_link() {
    let dir = tempfile::tempdir().unwrap();
    let platform = two_agents(dir.path(), false);
    let choice = choice(&platform);
    let before = choice.link.launch_epoch();
    choice.switch_to("faux");
    assert_that!(choice.link.launch_epoch()).is_equal_to(before);
}

#[test]
fn persist_writes_the_active_agent() {
    let dir = tempfile::tempdir().unwrap();
    let platform = two_agents(dir.path(), false);
    let choice = choice(&platform);
    choice.switch_to("bis");
    choice.persist().unwrap();
    let toml = std::fs::read_to_string(dir.path().join("config.toml")).unwrap_or_default();
    assert_that!(toml).contains("selected = \"bis\"");
}

#[test]
fn agents_view_lists_presets_with_the_active_one() {
    let dir = tempfile::tempdir().unwrap();
    let platform = two_agents(dir.path(), false);
    let view = choice(&platform).agents_view();
    let ids: Vec<&str> = view.presets.iter().map(|p| p.id.as_str()).collect();
    assert_that!((ids, view.active.as_str())).is_equal_to((vec!["bis", "faux"], "faux"));
}

#[test]
fn agents_view_builtin_has_no_hint() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(dir.path().to_path_buf());
    settings.paths.decks = Some(dir.path().join("decks"));
    settings.agent.presets =
        [("local".to_owned(), AgentPreset::builtin("Local", Builtin::Local))].into_iter().collect();
    settings.agent.selected = "local".into();
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    let view = choice(&platform).agents_view();
    let expected = AgentPresetView { id: "local".into(), name: "Local".into(), hint: None };
    assert_that!(view.presets).is_equal_to(vec![expected]);
}
