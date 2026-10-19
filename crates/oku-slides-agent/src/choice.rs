//! Le choix de l'agent de l'app : lequel est actif, est-il utilisable, le panneau qui le propose.
//!
//! Partagé (`Arc`) entre la pile et le fil de téléchargement du modèle local, qui réémet
//! la disponibilité et le panneau quand l'état du modèle change.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use oku_slides_i18n::Localize;
use oku_slides_platform::config_write::write_value;
use oku_slides_platform::find_program;
use oku_slides_platform::local_agent::{
    AGENT_MODEL_FILES, AGENT_MODEL_SIZE, Unavailable, agent_model_dir, local_engine,
};
use oku_slides_session::Platform;
use oku_slides_session::agent::Surface;
use snafu::{OptionExt, ResultExt, ensure};

use crate::command_error::{AgentLockedSnafu, AgentNotSavedSnafu, CommandError, UnknownAgentSnafu};
use crate::error::Unusable;
use crate::model_download::{DownloadState, ModelDownload, failure_text, percent};
use crate::picker::{LocalPanel, picker_surface};
use crate::{Launch, Link};

/// L'agent actif de l'app (clé de `[agent.presets]`) et ce qui en dépend.
#[derive(Debug)]
pub(crate) struct AgentChoice {
    platform: Platform,
    link: Link,
    active: Mutex<String>,
    /// Le téléchargement du modèle de l'agent local.
    download: ModelDownload,
}

impl AgentChoice {
    /// Le choix des réglages de `platform`, sur la connexion `link`.
    pub(crate) fn new(platform: &Platform, link: Link) -> Self {
        Self {
            platform: platform.clone(),
            link,
            active: Mutex::new(platform.settings.agent.selected.clone()),
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
        let Some(preset) = self.platform.settings.agent.presets.get(&active) else {
            // `select` ne choisit qu'un agent de la liste, vérifiée au chargement.
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

    /// Le panneau des agents avec l'agent actif ; `busy` : un tour est ouvert dans un onglet.
    ///
    /// Peut bloquer : lit le disque (état du modèle local).
    pub(crate) fn picker(&self, busy: bool) -> Surface {
        let agent = &self.platform.settings.agent;
        let local = self.local_panel();
        picker_surface(
            &agent.presets,
            &self.active(),
            busy,
            agent.locked,
            local.as_ref(),
            &self.platform.live_localizer.current(),
        )
    }

    /// Ce que le panneau montre du modèle local : `None` sauf pour l'agent intégré sans modèle.
    ///
    /// Peut bloquer : lit le disque.
    pub(crate) fn local_panel(&self) -> Option<LocalPanel> {
        let preset = self.platform.settings.agent.presets.get(&self.active())?;
        preset.builtin.as_ref()?;
        let Err(unavailable @ Unavailable::NeedsDownload { .. }) =
            local_engine(&self.platform.settings, &self.platform.storage)
        else {
            return None;
        };
        let l = self.platform.live_localizer.current();
        let reason = Unusable::from(unavailable).localize(&l);
        Some(match self.download.state() {
            DownloadState::Running { received, total } => LocalPanel::Running { percent: percent(received, total) },
            DownloadState::Failed(kind) => LocalPanel::Failed { reason, error: failure_text(kind, &l) },
            DownloadState::Idle => LocalPanel::NeedsDownload { reason },
        })
    }

    /// Lance le téléchargement du modèle local ; faux si le panneau ne le propose pas.
    ///
    /// `on_change` reçoit le choix à chaque pourcent et à la fin, sur le fil du téléchargement ;
    /// il n'est plus appelé une fois le choix lâché. Peut bloquer : lit le disque.
    pub(crate) fn start_download(self: &Arc<Self>, on_change: impl Fn(&AgentChoice) + Send + 'static) -> bool {
        if !matches!(self.local_panel(), Some(LocalPanel::NeedsDownload { .. } | LocalPanel::Failed { .. })) {
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
                    on_change(&c);
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

    /// Choisit l'agent `id` pour toute l'app : enregistré dans `config.toml`, puis bascule.
    ///
    /// Rien ne change si le choix est refusé (inconnu, imposé, non enregistré).
    pub(crate) fn select(&self, id: &str) -> Result<(), CommandError> {
        let agent = &self.platform.settings.agent;
        ensure!(!agent.locked, AgentLockedSnafu);
        let preset = agent.presets.get(id).context(UnknownAgentSnafu { id })?;
        // Gardé jusqu'à la bascule : deux choix croisés s'appliquent l'un après l'autre.
        let mut active = self.active.lock().unwrap_or_else(PoisonError::into_inner);
        if *active == id {
            return Ok(());
        }
        write_value(
            &self.platform.storage,
            &self.platform.settings.config_file(),
            "agent.selected",
            &serde_json::Value::from(id),
        )
        .context(AgentNotSavedSnafu)?;
        self.link.switch(Self::launch_of(preset, &self.platform));
        id.clone_into(&mut active);
        Ok(())
    }
}

#[cfg(test)]
#[path = "choice_tests.rs"]
mod choice_tests;
