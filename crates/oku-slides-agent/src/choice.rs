//! Le choix de l'agent de l'app : lequel est actif, est-il utilisable, ce que la boîte en montre.
//!
//! Partagé (`Arc`) entre la pile et la boîte des réglages (`AppVoice`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_mcp::AgentDiagnosis;
use oku_slides_platform::config::AgentPreset;
use oku_slides_platform::config_write::{ConfigWriteError, write_value};
use oku_slides_platform::find_program;
use oku_slides_session::{AgentPresetView, AgentsView, Platform};

use crate::error::{Reason, Unusable};
use crate::session_model::Offered;
use crate::{Launch, Link};

/// Pourquoi un agent ne peut pas être choisi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// Il n'est pas dans la configuration.
    UnknownAgent,
    /// L'environnement impose le choix.
    Locked,
}

/// Why a model of the active agent cannot be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModelRefusal {
    /// The active agent does not offer it, or offers no choice yet.
    Unknown,
    /// Its preset id cannot be written as a `config.toml` key.
    NotSaved,
}

/// L'agent actif de l'app (clé de `[agent.presets]`) et ce qui en dépend.
#[derive(Debug)]
pub(crate) struct AgentChoice {
    platform: Platform,
    link: Link,
    active: Mutex<String>,
    /// The model asked of each preset's sessions, by preset id; seeded from the settings.
    ///
    /// Taken after `active`, never before.
    models: Mutex<HashMap<String, String>>,
    /// Tenu pendant l'écriture de `agent.selected` ou d'un modèle : jamais avec le verrou d'état de la boîte.
    saving: Mutex<()>,
}

impl AgentChoice {
    /// Le choix des réglages de `platform`, sur la connexion `link`.
    pub(crate) fn new(platform: &Platform, link: Link) -> Self {
        let models =
            platform.settings.agent.presets().filter_map(|(id, p)| Some((id.to_owned(), p.model.clone()?))).collect();
        Self {
            platform: platform.clone(),
            link,
            active: Mutex::new(platform.settings.agent.selected().to_owned()),
            models: Mutex::new(models),
            saving: Mutex::new(()),
        }
    }

    /// Le lancement de `preset`.
    pub(crate) fn launch_of(preset: &AgentPreset, platform: &Platform) -> Launch {
        // Never the data folder itself: it holds config.toml and the guide approvals.
        let process_dir = platform.settings.paths.data().join(crate::config::AGENT_PROCESS_DIR);
        Launch { path: Some(platform.program_path.clone()), ..Launch::for_preset(preset, process_dir) }
    }

    /// L'agent choisi (clé de `[agent.presets]`).
    pub(crate) fn active(&self) -> String {
        self.lock_active().clone()
    }

    fn lock_active(&self) -> MutexGuard<'_, String> {
        self.active.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Pourquoi l'agent choisi est inutilisable, localisé ; `None` s'il semble utilisable.
    ///
    /// See [`AgentChoice::main_reason`]. Peut bloquer : lit le disque.
    pub(crate) fn availability(&self) -> Option<String> {
        self.main_reason().map(|r| r.localize(&self.platform.live_localizer.current()))
    }

    /// Why the selected agent is unusable; `None` if it seems usable.
    ///
    /// Its program not found (path, or name looked up in `PATH`); else the recent failure of
    /// its last launch. May block: reads the disk.
    pub(crate) fn main_reason(&self) -> Option<Reason> {
        let active = self.active();
        let Some(preset) = self.platform.settings.agent.preset(&active) else {
            // `check` ne laisse choisir qu'un agent de la liste, vérifiée au chargement.
            tracing::error!(active, "agent actif absent des réglages");
            return Some(Reason::UnknownAgent { id: active });
        };
        let path = self.platform.program_path.get();
        if find_program(&self.platform.storage, Path::new(&preset.command), path).is_none() {
            return Some(Unusable::NotFound { command: preset.command.clone() }.into());
        }
        self.link.failure().map(Reason::from)
    }

    /// The selected agent for the `diagnose` tool: preset, availability, stderr tail.
    ///
    /// May block: reads the disk (see [`AgentChoice::availability`]).
    pub(crate) fn diagnosis(&self) -> AgentDiagnosis {
        let selected = self.active();
        let name = self.platform.settings.agent.preset(&selected).map(|p| p.name.clone()).unwrap_or_default();
        AgentDiagnosis { name, unusable: self.availability(), stderr: self.link.stderr_tail(), selected }
    }

    /// Les agents tels que les montre la boîte ; `busy` et `pending` restent à remplir.
    pub(crate) fn agents_view(&self) -> AgentsView {
        let agent = &self.platform.settings.agent;
        let active = self.active();
        let models = self.link.offered_models().view(self.chosen_model(&active));
        AgentsView {
            presets: agent.presets().map(|(id, p)| preset_view(id, p)).collect(),
            active,
            busy: false,
            locked: agent.locked,
            pending: None,
            models,
        }
    }

    /// Advances each time what the active agent's sessions tell of models may change.
    pub(crate) fn models_revision(&self) -> tokio::sync::watch::Receiver<u64> {
        self.link.models_revision()
    }

    /// The model asked of the sessions of preset `id`, if one is chosen.
    pub(crate) fn chosen_model(&self, id: &str) -> Option<String> {
        self.models.lock().unwrap_or_else(PoisonError::into_inner).get(id).cloned()
    }

    /// Whether model `model` of the active agent can be chosen.
    ///
    /// Gives the preset checked, to save the choice for, and whether `model` is already chosen.
    pub(crate) fn check_model(&self, model: &str) -> Result<(String, bool), ModelRefusal> {
        let active = self.active();
        let Offered::Models(offered) = self.link.offered_models() else { return Err(ModelRefusal::Unknown) };
        if !offered.offers(model) {
            return Err(ModelRefusal::Unknown);
        }
        // `write_value` splits its key on dots: such a preset id would name another key.
        if active.contains('.') {
            return Err(ModelRefusal::NotSaved);
        }
        let unchanged = self.chosen_model(&active).as_deref() == Some(model);
        Ok((active, unchanged))
    }

    /// Writes `model` for preset `id` in `config.toml`, then asks it of its sessions.
    ///
    /// A failed write changes nothing: the model shown stays the saved one. Applies from
    /// the next prompt of each tab if `id` is still the active agent. Writes to the disk.
    pub(crate) fn choose_model(&self, id: &str, model: &str) -> Result<(), Box<ConfigWriteError>> {
        let _saving = self.saving.lock().unwrap_or_else(PoisonError::into_inner);
        let (storage, file) = (&self.platform.storage, self.platform.settings.config_file());
        let key = format!("agent.presets.{id}.model");
        write_value(storage, &file, &key, &serde_json::Value::from(model)).map_err(Box::new)?;
        let active = self.lock_active();
        self.models.lock().unwrap_or_else(PoisonError::into_inner).insert(id.to_owned(), model.to_owned());
        if *active == id {
            self.link.set_wanted_model(Some(model.to_owned()));
        }
        Ok(())
    }

    /// La langue courante de l'interface.
    pub(crate) fn localizer(&self) -> Arc<Localizer> {
        self.platform.live_localizer.current()
    }

    /// L'agent `id`, s'il peut être choisi : connu et pas imposé.
    pub(crate) fn check(&self, id: &str) -> Result<&AgentPreset, Refusal> {
        let agent = &self.platform.settings.agent;
        if agent.locked {
            return Err(Refusal::Locked);
        }
        agent.preset(id).ok_or(Refusal::UnknownAgent)
    }

    /// La vue de l'agent `id` montrée dans une confirmation ; `None` s'il est inconnu.
    pub(crate) fn preset_view(&self, id: &str) -> Option<AgentPresetView> {
        self.platform.settings.agent.preset(id).map(|p| preset_view(id, p))
    }

    /// Switches to agent `id`; nothing if it is already the active one, served. Fast: nothing is written.
    ///
    /// The agent was checked by [`AgentChoice::check`]; unknown here, the refusal is logged.
    pub(crate) fn switch_to(&self, id: &str) {
        let mut active = self.lock_active();
        if *active == id {
            return;
        }
        let Some(preset) = self.platform.settings.agent.preset(id) else {
            tracing::error!(id, "agent absent des réglages : bascule refusée");
            return;
        };
        self.link.switch(id, Self::launch_of(preset, &self.platform), self.chosen_model(id));
        id.clone_into(&mut active);
    }

    /// Écrit l'agent actif dans `agent.selected` de `config.toml`.
    pub(crate) fn persist(&self) -> Result<(), Box<ConfigWriteError>> {
        let _saving = self.saving.lock().unwrap_or_else(PoisonError::into_inner);
        self.write_selected(&self.active())
    }

    /// Écrit `id` dans `agent.selected`, puis lance `then` avant qu'aucune autre écriture ne passe.
    ///
    /// Un échec d'écriture ne lance pas `then` : l'agent montré reste celui qui est enregistré.
    pub(crate) fn save_then(&self, id: &str, then: impl FnOnce()) -> Result<(), Box<ConfigWriteError>> {
        let _saving = self.saving.lock().unwrap_or_else(PoisonError::into_inner);
        self.write_selected(id)?;
        then();
        Ok(())
    }

    /// L'époque du lancement de l'agent : elle avance à chaque bascule.
    #[cfg(test)]
    pub(crate) fn launch_epoch(&self) -> u64 {
        self.link.launch_epoch()
    }

    /// Écrit `id` ; l'erreur est boxée : elle grossirait tout `Result` qui la porte.
    fn write_selected(&self, id: &str) -> Result<(), Box<ConfigWriteError>> {
        let (storage, file) = (&self.platform.storage, self.platform.settings.config_file());
        write_value(storage, &file, "agent.selected", &serde_json::Value::from(id)).map_err(Box::new)
    }
}

/// La vue d'un agent : sa commande.
fn preset_view(id: &str, p: &AgentPreset) -> AgentPresetView {
    let command =
        std::iter::once(p.command.as_str()).chain(p.args.iter().map(String::as_str)).collect::<Vec<_>>().join(" ");
    AgentPresetView { id: id.to_owned(), name: p.name.clone(), command }
}

#[cfg(test)]
#[path = "choice_tests.rs"]
mod choice_tests;
