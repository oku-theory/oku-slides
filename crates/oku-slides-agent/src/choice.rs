//! Le choix de l'agent de l'app : lequel est actif, est-il utilisable, ce que la boîte en montre.
//!
//! Partagé (`Arc`) entre la pile, la boîte des réglages (`AppVoice`) et le fil de téléchargement
//! du modèle local, qui fait avancer la révision de la boîte quand l'état du modèle change.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::config::AgentPreset;
use oku_slides_platform::config_write::{ConfigWriteError, write_value};
use oku_slides_platform::find_program;
use oku_slides_platform::local_agent::{
    AGENT_MODEL_FILES, AGENT_MODEL_SIZE, Unavailable, agent_model_dir, local_engine,
};
use oku_slides_platform::models::DownloadErrorKind;
use oku_slides_session::{AgentPresetView, AgentsView, LocalFailure, LocalModelView, Platform};

use crate::command_error::{CommandError, UnknownAgentSnafu};
use crate::error::Unusable;
use crate::model_download::{DownloadState, ModelDownload, percent};
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
    /// Le téléchargement du modèle de l'agent local.
    download: ModelDownload,
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
            download: ModelDownload::default(),
        }
    }

    /// Le lancement de `preset`. Un agent intégré sans binaire d'app a une commande vide :
    /// un prompt échoue au lancement comme un programme absent, et `availability` en donne la raison.
    pub(crate) fn launch_of(preset: &oku_slides_platform::config::AgentPreset, platform: &Platform) -> Launch {
        let cwd = platform.settings.paths.data();
        Launch::for_preset(preset, platform.program.as_deref(), cwd.clone()).unwrap_or(Launch {
            command: String::new(),
            args: Vec::new(),
            env: Vec::new(),
            fallback_cwd: cwd,
            ready_within: crate::READY_WITHIN,
        })
    }

    /// L'agent choisi (clé de `[agent.presets]`).
    pub(crate) fn active(&self) -> String {
        self.active.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Pourquoi l'agent choisi est inutilisable, localisé ; `None` s'il semble utilisable.
    ///
    /// Son programme introuvable (chemin, ou nom cherché dans `PATH`) ; pour l'agent intégré,
    /// l'état du moteur local (modèle à télécharger, machine non prise en charge) ; sinon
    /// l'échec récent de son dernier lancement. Peut bloquer : lit le disque.
    pub(crate) fn availability(&self) -> Option<String> {
        let l = self.platform.live_localizer.current();
        let active = self.active();
        let Some(preset) = self.platform.settings.agent.preset(&active) else {
            // `check` ne laisse choisir qu'un agent de la liste, vérifiée au chargement.
            tracing::error!(active, "agent actif absent des réglages");
            return Some(CommandError::from(UnknownAgentSnafu { id: active }.build()).localize(&l));
        };
        if preset.builtin.is_some() {
            if self.platform.program.is_none() {
                return Some(Unusable::NoProgram.localize(&l));
            }
            return match local_engine(&self.platform.settings, &self.platform.storage) {
                Err(unavailable) => Some(Unusable::from(unavailable).localize(&l)),
                Ok(_) => self.link.failure().map(|u| u.localize(&l)),
            };
        }
        let path = std::env::var_os("PATH");
        if find_program(&self.platform.storage, Path::new(&preset.command), path.as_deref()).is_none() {
            return Some(Unusable::NotFound { command: preset.command.clone() }.localize(&l));
        }
        self.link.failure().map(|u| u.localize(&l))
    }

    /// Ce que la boîte montre du modèle local : `None` sauf pour l'agent intégré sans modèle.
    ///
    /// Peut bloquer : lit le disque.
    pub(crate) fn local_panel(&self) -> Option<LocalModelView> {
        let preset = self.platform.settings.agent.preset(&self.active())?;
        preset.builtin.as_ref()?;
        let Err(Unavailable::NeedsDownload { size }) = local_engine(&self.platform.settings, &self.platform.storage)
        else {
            return None;
        };
        Some(match self.download.state() {
            DownloadState::Running { received, total } => LocalModelView::Running { percent: percent(received, total) },
            DownloadState::Failed(kind) => LocalModelView::Failed { size, failure: local_failure(kind) },
            DownloadState::Idle => LocalModelView::NeedsDownload { size },
        })
    }

    /// Les agents tels que les montre la boîte ; `busy` et `pending` restent à remplir.
    ///
    /// Peut bloquer : lit le disque (état du modèle local).
    pub(crate) fn agents_view(&self) -> AgentsView {
        let agent = &self.platform.settings.agent;
        let active = self.active();
        let external = agent.preset(&active).is_some_and(|p| p.builtin.is_none());
        let models = if external { self.link.offered_models().view(self.chosen_model(&active)) } else { None };
        AgentsView {
            presets: agent.presets().map(|(id, p)| preset_view(id, p)).collect(),
            active,
            busy: false,
            locked: agent.locked,
            local: self.local_panel(),
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
        let external = self.platform.settings.agent.preset(&active).is_some_and(|p| p.builtin.is_none());
        if !external || !offered.offers(model) {
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
        let active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
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

    /// Lance le téléchargement du modèle local ; faux si le panneau ne le propose pas.
    ///
    /// `on_change` est appelé à chaque pourcent et à la fin, sur le fil du téléchargement ;
    /// il ne l'est plus une fois le choix lâché. Peut bloquer : lit le disque.
    pub(crate) fn start_download(self: &Arc<Self>, on_change: impl Fn() + Send + 'static) -> bool {
        if !matches!(self.local_panel(), Some(LocalModelView::NeedsDownload { .. } | LocalModelView::Failed { .. })) {
            return false;
        }
        let settings = &self.platform.settings;
        // Référence faible : le fil du téléchargement ne garde pas la pile en vie.
        let choice = Arc::downgrade(self);
        self.download.start(
            &AGENT_MODEL_FILES,
            agent_model_dir(settings),
            AGENT_MODEL_SIZE,
            settings.models.clone(),
            self.platform.storage.clone(),
            move || {
                if let Some(c) = choice.upgrade() {
                    c.download_changed();
                    on_change();
                }
            },
        )
    }

    /// Le téléchargement a progressé ou fini ; fini avec succès (`Idle`), un échec de lancement
    /// retenu avant (modèle absent) est périmé : on l'oublie, sinon il resterait affiché.
    fn download_changed(&self) {
        if self.download.state() == DownloadState::Idle {
            self.link.forget_failure();
        }
    }

    /// Arrête le téléchargement du modèle en cours, s'il y en a un.
    pub(crate) fn cancel_download(&self) {
        self.download.cancel();
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

    /// Bascule sur l'agent `id` ; rien si c'est déjà l'actif. Rapide : rien n'est écrit.
    ///
    /// L'agent a été vérifié par [`AgentChoice::check`] ; inconnu ici, le refus est journalisé.
    pub(crate) fn switch_to(&self, id: &str) {
        let mut active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        if *active == id {
            return;
        }
        let Some(preset) = self.platform.settings.agent.preset(id) else {
            tracing::error!(id, "agent absent des réglages : bascule refusée");
            return;
        };
        // The built-in agent offers no model choice: none is asked of it.
        let model = preset.builtin.is_none().then(|| self.chosen_model(id)).flatten();
        self.link.switch(Self::launch_of(preset, &self.platform), model);
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

/// La vue d'un agent : sa commande pour un agent externe, rien pour un agent intégré.
fn preset_view(id: &str, p: &AgentPreset) -> AgentPresetView {
    let hint = p.builtin.is_none().then(|| {
        std::iter::once(p.command.as_str()).chain(p.args.iter().map(String::as_str)).collect::<Vec<_>>().join(" ")
    });
    AgentPresetView { id: id.to_owned(), name: p.name.clone(), hint }
}

/// La cause d'un téléchargement échoué, telle que la boîte la montre.
fn local_failure(kind: DownloadErrorKind) -> LocalFailure {
    match kind {
        DownloadErrorKind::Network | DownloadErrorKind::Write => LocalFailure::Failed,
        DownloadErrorKind::Integrity => LocalFailure::Integrity,
        DownloadErrorKind::Incomplete { got, expected } => LocalFailure::Incomplete { got, expected },
        DownloadErrorKind::Cancelled => LocalFailure::Cancelled,
    }
}

#[cfg(test)]
#[path = "choice_tests.rs"]
mod choice_tests;
