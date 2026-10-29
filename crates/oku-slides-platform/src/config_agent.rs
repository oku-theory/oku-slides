//! `[agent]` settings: the offered ACP agents and the active one.

use std::collections::BTreeMap;
use std::ops::Bound;

use serde::Deserialize;
use snafu::{Backtrace, Snafu};

/// The offered ACP agents and the one ⌘⇧D launches.
///
/// The active agent is always one of the presets: every constructor and setter checks it,
/// and so does loading (`[agent]` keeps its TOML shape: `selected` and `presets.ID`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawAgentSettings")]
pub struct AgentSettings {
    /// Key of the active agent, kept out of `others`.
    selected: String,
    /// The active agent's preset.
    active: AgentPreset,
    /// Every other preset, by id.
    others: BTreeMap<String, AgentPreset>,
    /// `OKU_SLIDES_AGENT__SELECTED` forces the choice: the panel does not change it.
    pub locked: bool,
}

/// `[agent]` as written in TOML, checked into an [`AgentSettings`].
#[derive(Deserialize)]
pub(super) struct RawAgentSettings {
    /// The active agent: a key of `presets`.
    selected: String,
    /// The offered agents, by id.
    presets: BTreeMap<String, AgentPreset>,
}

impl RawAgentSettings {
    /// Checks the raw shape, as [`AgentSettings::new`] does.
    pub(super) fn check(self) -> Result<AgentSettings, AgentSettingsError> {
        AgentSettings::new(self.selected, self.presets)
    }
}

impl TryFrom<RawAgentSettings> for AgentSettings {
    type Error = AgentSettingsError;

    fn try_from(raw: RawAgentSettings) -> Result<Self, Self::Error> {
        raw.check()
    }
}

/// Agent settings that would break their invariant.
///
/// [`oku_slides_i18n::Localize`] translates it for tools and tests that edit settings; a load
/// error keeps the English `Display` text inside [`super::ConfigError`].
#[derive(Debug, Snafu)]
pub struct AgentSettingsError(AgentSettingsInner);

/// The reasons behind an [`AgentSettingsError`].
#[derive(Debug, Snafu)]
enum AgentSettingsInner {
    /// The selected id names no preset.
    #[snafu(display("agent.selected = {selected:?} names no [agent.presets] entry (known: {})", known.join(", ")))]
    UnknownSelected { selected: String, known: Vec<String>, backtrace: Backtrace },
    /// A preset has both or neither of `command` and `builtin`.
    #[snafu(display("[agent.presets.{id}] needs exactly one of command or builtin"))]
    PresetLaunch { id: String, backtrace: Backtrace },
}

impl oku_slides_i18n::Localize for AgentSettingsError {
    fn localize(&self, l: &oku_slides_i18n::Localizer) -> String {
        match &self.0 {
            AgentSettingsInner::UnknownSelected { selected, known, .. } => l.tr_with(
                "agent-settings-unknown-selected",
                &[("selected", selected.as_str().into()), ("known", known.join(", ").into())],
            ),
            AgentSettingsInner::PresetLaunch { id, .. } => {
                l.tr_with("agent-settings-preset-launch", &[("id", id.as_str().into())])
            }
        }
    }
}

/// An agent provided by Oku Slides itself, without an external program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    /// The local fallback agent (`oku-slides-local-agent`), launched by this binary.
    Local,
}

/// An offered agent: its display name and how to launch it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[non_exhaustive]
pub struct AgentPreset {
    /// Proper name shown as is (not translated).
    pub name: String,
    /// Launched program (looked up in `PATH`); empty for a builtin agent.
    #[serde(default)]
    pub command: String,
    /// Program arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Builtin agent: exclusive with `command`.
    #[serde(default)]
    pub builtin: Option<Builtin>,
    /// Model asked of each session, a value of the agent's `model` option; absent: its default.
    #[serde(default)]
    pub model: Option<String>,
}

impl AgentPreset {
    /// An offered agent launched by a program (tests, tools).
    pub fn new(name: impl Into<String>, command: impl Into<String>, args: Vec<String>) -> Self {
        Self { name: name.into(), command: command.into(), args, builtin: None, model: None }
    }

    /// An agent built into Oku Slides.
    pub fn builtin(name: impl Into<String>, builtin: Builtin) -> Self {
        Self { name: name.into(), command: String::new(), args: Vec::new(), builtin: Some(builtin), model: None }
    }
}

impl AgentSettings {
    /// Agent settings whose active agent is `selected`, among `presets`.
    ///
    /// With [`Self::select`] and [`Self::insert_preset`], the checked way for tools and tests
    /// to edit agent settings.
    ///
    /// # Errors
    ///
    /// If a preset has not exactly one of `command` and `builtin`, or if `selected` names
    /// no preset.
    pub fn new(
        selected: impl Into<String>,
        mut presets: BTreeMap<String, AgentPreset>,
    ) -> Result<Self, AgentSettingsError> {
        for (id, preset) in &presets {
            check_launch(id, preset)?;
        }
        let selected = selected.into();
        let Some(active) = presets.remove(&selected) else {
            let known: Vec<String> = presets.into_keys().collect();
            return Err(UnknownSelectedSnafu { selected, known }.build().into());
        };
        Ok(Self { selected, active, others: presets, locked: false })
    }

    /// Id of the active agent, a key of [`Self::presets`].
    #[must_use]
    pub fn selected(&self) -> &str {
        &self.selected
    }

    /// The active agent as loaded.
    ///
    /// The live choice of a running app is `AgentChoice::active` in `oku-slides-agent`.
    #[must_use]
    pub fn active(&self) -> &AgentPreset {
        &self.active
    }

    /// The preset `id`, if offered.
    #[must_use]
    pub fn preset(&self, id: &str) -> Option<&AgentPreset> {
        if id == self.selected { Some(&self.active) } else { self.others.get(id) }
    }

    /// Every offered preset with its id, in id order (the display order).
    pub fn presets(&self) -> impl Iterator<Item = (&str, &AgentPreset)> {
        let selected = self.selected.as_str();
        let before = self.others.range::<str, _>((Bound::<&str>::Unbounded, Bound::Excluded(selected)));
        let after = self.others.range::<str, _>((Bound::Excluded(selected), Bound::<&str>::Unbounded));
        before
            .map(|(id, p)| (id.as_str(), p))
            .chain(std::iter::once((selected, &self.active)))
            .chain(after.map(|(id, p)| (id.as_str(), p)))
    }

    /// Makes the preset `id` the active agent.
    ///
    /// A checked edit, like [`Self::new`] and [`Self::insert_preset`].
    ///
    /// # Errors
    ///
    /// If `id` names no preset; the active agent is then unchanged.
    pub fn select(&mut self, id: &str) -> Result<(), AgentSettingsError> {
        if id == self.selected {
            return Ok(());
        }
        let Some(active) = self.others.remove(id) else {
            let known: Vec<String> = self.presets().map(|(id, _)| id.to_owned()).collect();
            return Err(UnknownSelectedSnafu { selected: id, known }.build().into());
        };
        let previous = std::mem::replace(&mut self.active, active);
        let previous_id = std::mem::replace(&mut self.selected, id.to_owned());
        self.others.insert(previous_id, previous);
        Ok(())
    }

    /// Adds the preset `id`, or replaces it (the active one included).
    ///
    /// A checked edit, like [`Self::new`] and [`Self::select`].
    ///
    /// # Errors
    ///
    /// If `preset` has not exactly one of `command` and `builtin`; nothing changes then.
    pub fn insert_preset(&mut self, id: impl Into<String>, preset: AgentPreset) -> Result<(), AgentSettingsError> {
        let id = id.into();
        check_launch(&id, &preset)?;
        if id == self.selected {
            self.active = preset;
        } else {
            self.others.insert(id, preset);
        }
        Ok(())
    }
}

/// Checks that `preset` has exactly one of `command` and `builtin`.
fn check_launch(id: &str, preset: &AgentPreset) -> Result<(), AgentSettingsError> {
    if preset.command.is_empty() == preset.builtin.is_none() {
        return Err(PresetLaunchSnafu { id }.build().into());
    }
    Ok(())
}
