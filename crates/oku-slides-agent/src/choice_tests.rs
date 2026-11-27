use agent_client_protocol::schema::v1::SessionId;
use oku_slides_i18n::Localizer;
use oku_slides_platform::config::{AgentPreset, AgentSettings};
use oku_slides_session::{AgentModelsView, Platform, Settings};
use speculoos::prelude::*;

use super::{AgentChoice, ModelRefusal, Refusal};
use crate::session_model::{ModelOptions, ModelValue};
use crate::{Launch, Link};

fn choice(platform: &Platform) -> AgentChoice {
    let launch = Launch {
        command: String::new(),
        args: Vec::new(),
        path: None,
        process_dir: platform.settings.paths.data().join(crate::config::AGENT_PROCESS_DIR),
        ready_within: std::time::Duration::from_secs(1),
    };
    AgentChoice::new(platform, Link::new(launch, platform.storage.clone()))
}

fn fr() -> Localizer {
    Localizer::new(Some("fr"))
}

/// Une plateforme sur `data` à deux agents, `faux` (choisi) et `bis` ; `locked` : choix imposé.
fn two_agents(data: &std::path::Path, locked: bool) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    let presets = [
        ("faux".to_owned(), AgentPreset::new("Faux", "fake", vec!["--script".into(), "s".into()])),
        ("bis".to_owned(), AgentPreset::new("Bis", "true", vec![])),
    ]
    .into_iter()
    .collect();
    settings.agent = AgentSettings::new("faux", presets).unwrap_or_else(|e| panic!("agent : {e}"));
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
fn agents_view_gives_each_preset_its_command_line() {
    let dir = tempfile::tempdir().unwrap();
    let platform = two_agents(dir.path(), false);
    let commands: Vec<(String, String)> =
        choice(&platform).agents_view().presets.into_iter().map(|p| (p.id, p.command)).collect();
    assert_that!(commands)
        .is_equal_to(vec![("bis".to_owned(), "true".to_owned()), ("faux".to_owned(), "fake --script s".to_owned())]);
}

/// Settings on `data` with the external preset `id`, active, asking `model`.
fn external_settings(data: &std::path::Path, id: &str, model: Option<&str>) -> Settings {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    let mut preset = AgentPreset::new("Codex", "/bin/codex", Vec::new());
    preset.model = model.map(str::to_owned);
    settings.agent.insert_preset(id, preset).unwrap_or_else(|e| panic!("preset: {e}"));
    settings.agent.select(id).unwrap_or_else(|e| panic!("agent: {e}"));
    settings.i18n.locale = Some("fr".into());
    settings
}

/// A platform on `data` whose active agent is the external preset `codex`, asking `model`.
fn codex_platform(data: &std::path::Path, model: Option<&str>) -> Platform {
    Platform::with_settings(external_settings(data, "codex", model)).unwrap_or_else(|e| panic!("platform: {e}"))
}

/// A session of the current launch offered `fast` (in use) and `deep`.
fn offer(choice: &AgentChoice) {
    let values =
        ["fast", "deep"].map(|id| ModelValue { id: id.to_owned(), name: id.to_owned(), description: None }).to_vec();
    let options = ModelOptions { config_id: "model".into(), current: "fast".into(), values };
    choice.link.models().told(1, 0, &SessionId::new("s"), Some(options));
}

#[test]
fn new_seeds_the_chosen_model_from_the_preset() {
    let dir = tempfile::tempdir().unwrap();
    let choice = choice(&codex_platform(dir.path(), Some("deep")));
    assert_that!(choice.chosen_model("codex")).is_equal_to(Some("deep".to_owned()));
}

#[test]
fn agents_view_external_agent_before_any_session_shows_the_chosen_model_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let choice = choice(&codex_platform(dir.path(), Some("deep")));
    let expected = AgentModelsView::Unknown { chosen: Some("deep".into()) };
    assert_that!(choice.agents_view().models).is_equal_to(Some(expected));
}

#[test]
fn check_model_before_any_session_is_refused_as_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let choice = choice(&codex_platform(dir.path(), None));
    assert_that!(choice.check_model("deep")).is_equal_to(Err(ModelRefusal::Unknown));
}

#[test]
fn check_model_not_offered_is_refused_as_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let choice = choice(&codex_platform(dir.path(), None));
    offer(&choice);
    assert_that!(choice.check_model("huge")).is_equal_to(Err(ModelRefusal::Unknown));
}

#[rstest::rstest]
#[case::already_chosen(Some("deep"), true)]
#[case::new_choice(None, false)]
fn check_model_offered_tells_whether_it_is_unchanged(#[case] chosen: Option<&str>, #[case] unchanged: bool) {
    let dir = tempfile::tempdir().unwrap();
    let choice = choice(&codex_platform(dir.path(), chosen));
    offer(&choice);
    assert_that!(choice.check_model("deep")).is_equal_to(Ok(("codex".to_owned(), unchanged)));
}

#[test]
fn check_model_of_a_dotted_preset_id_is_refused_as_not_saved() {
    let dir = tempfile::tempdir().unwrap();
    let settings = external_settings(dir.path(), "co.dex", None);
    let choice = choice(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}")));
    offer(&choice);
    assert_that!(choice.check_model("deep")).is_equal_to(Err(ModelRefusal::NotSaved));
}

#[test]
fn choose_model_writes_the_preset_model_key() {
    let dir = tempfile::tempdir().unwrap();
    let platform = codex_platform(dir.path(), None);
    choice(&platform).choose_model("codex", "deep").unwrap_or_else(|e| panic!("write: {e}"));
    let toml = std::fs::read_to_string(platform.settings.config_file()).unwrap();
    assert_that!(toml.contains("[agent.presets.codex]") && toml.contains("model = \"deep\"")).is_true();
}

#[test]
fn choose_model_of_the_active_agent_asks_it_of_the_link() {
    let dir = tempfile::tempdir().unwrap();
    let choice = choice(&codex_platform(dir.path(), None));
    choice.choose_model("codex", "deep").unwrap_or_else(|e| panic!("write: {e}"));
    assert_that!(choice.link.wanted_model()).is_equal_to(Some("deep".to_owned()));
}

#[test]
fn choose_model_unsaved_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let platform = codex_platform(dir.path(), None);
    // `config.toml` cannot be written: a directory stands in its place.
    std::fs::create_dir_all(platform.settings.config_file()).unwrap();
    let choice = choice(&platform);
    assert_that!(choice.choose_model("codex", "deep")).is_err();
    assert_that!((choice.chosen_model("codex"), choice.link.wanted_model())).is_equal_to((None, None));
}

#[test]
fn switch_to_asks_the_chosen_model_of_the_new_agent() {
    let dir = tempfile::tempdir().unwrap();
    let mut settings = external_settings(dir.path(), "codex", None);
    let mut bis = AgentPreset::new("Bis", "/bin/bis", Vec::new());
    bis.model = Some("deep".into());
    settings.agent.insert_preset("bis", bis).unwrap_or_else(|e| panic!("preset: {e}"));
    let choice = choice(&Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}")));
    choice.switch_to("bis");
    assert_that!(choice.link.wanted_model()).is_equal_to(Some("deep".to_owned()));
}

/// A platform whose active agent runs the bare command `agent-acp`, looked up on `path`.
fn bare_command_platform(data: &std::path::Path, path: Option<std::ffi::OsString>) -> Platform {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("settings: {e}"));
    settings.paths.data = Some(data.to_path_buf());
    settings.paths.decks = Some(data.join("decks"));
    settings
        .agent
        .insert_preset("bare", AgentPreset::new("Bare", "agent-acp", Vec::new()))
        .unwrap_or_else(|e| panic!("preset: {e}"));
    settings.agent.select("bare").unwrap_or_else(|e| panic!("agent: {e}"));
    settings.i18n.locale = Some("fr".into());
    let mut platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("platform: {e}"));
    platform.program_path = oku_slides_platform::ProgramPath::fixed(path);
    platform
}

#[test]
fn availability_finds_a_bare_command_on_the_program_path() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("agent-acp"), b"#!/bin/sh\n").unwrap();
    let platform = bare_command_platform(dir.path(), Some(bin.into_os_string()));
    assert_that!(choice(&platform).availability()).is_none();
}

#[test]
fn availability_of_a_command_off_the_program_path_says_where_to_fix_it() {
    let dir = tempfile::tempdir().unwrap();
    let platform = bare_command_platform(dir.path(), Some(dir.path().as_os_str().to_owned()));
    let expected = fr().tr_with("agent-not-found", &[("command", "agent-acp".into())]);
    assert_that!(choice(&platform).availability()).is_equal_to(Some(expected));
    assert_that!(fr().tr("agent-not-found")).contains("PATH");
}

#[test]
fn launch_of_carries_the_program_path() {
    let dir = tempfile::tempdir().unwrap();
    let platform = bare_command_platform(dir.path(), Some("/login/bin".into()));
    let launch = AgentChoice::launch_of(&AgentPreset::new("Bare", "agent-acp", Vec::new()), &platform);
    let path = launch.path.as_ref().and_then(|p| p.get()).map(std::ffi::OsStr::to_owned);
    assert_that!(path).is_equal_to(Some("/login/bin".into()));
}

#[test]
fn launch_of_runs_the_agent_in_the_agent_process_dir() {
    let dir = tempfile::tempdir().unwrap();
    let platform = bare_command_platform(dir.path(), None);
    let launch = AgentChoice::launch_of(&AgentPreset::new("Bare", "agent-acp", Vec::new()), &platform);
    assert_that!(launch.process_dir).is_equal_to(platform.settings.paths.data().join("agent-process"));
}
