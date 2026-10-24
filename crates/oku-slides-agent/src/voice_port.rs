//! La dictée et l'agent de l'app pour la boîte des réglages et les outils.
//!
//! [`AppVoice`] est le seul écrivain des réglages de la dictée et du choix de l'agent : clics
//! de la boîte (`Origin::User`) et outils de l'agent (`Origin::Agent`) passent par lui.
//!
//! Verrous : `switch` (l'état, mémoire seulement) n'est jamais tenu pendant une entrée-sortie,
//! ni pendant `view`. `edit` couvre la lecture-écriture des réglages de la dictée (`Stt::configure`
//! écrit sur le disque) et `AgentChoice.saving` l'écriture de `agent.selected` ; aucun des deux
//! n'est pris sous `switch`. Ordre quand ils s'emboîtent : `saving` → `switch` → `AgentChoice.active`
//! → les verrous du `Link`.

use std::sync::{Arc, Mutex, PoisonError, Weak};

use oku_slides_i18n::Localizer;
use oku_slides_mcp::{ApiTarget, Checked, Origin, VoiceApplied, VoiceError, VoiceErrorKind, VoicePort};
use oku_slides_session::{
    ApiView, DICTATION_LANGUAGES, DictationLanguage, DictationPatch, DictationView, DownloadView, LocalModelView,
    ModelKind, ModelView, VoiceEvent, VoiceView,
};
use oku_slides_stt::{
    Config, Model, ModelInfo, SILENCE_RANGE, State, Status, Stt, normalized_url, url_host, valid_api_model,
    valid_key_env,
};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::agent_switch::{AgentSwitch, PromptGuard, Requested, Switch};
use crate::choice::{AgentChoice, Refusal};

/// La réponse d'un changement d'agent demandé quand rien ne tournait, selon son issue.
///
/// Une requête commencée entre-temps le met en attente ; un clic de l'utilisateur l'a rendu caduc.
pub(crate) fn applied_of(outcome: Switch) -> VoiceApplied {
    match outcome {
        Switch::Go => VoiceApplied::Now,
        Switch::Wait => VoiceApplied::WhenIdle,
        Switch::Stale => VoiceApplied::Unchanged,
    }
}

/// Un refus de sorte `kind`.
fn refused(kind: VoiceErrorKind) -> VoiceError {
    VoiceError::new(kind)
}

/// Annuler n'est permis que pendant un téléchargement : `Stt::cancel` couperait une dictée.
pub(crate) fn cancel_dictation_allowed(state: State) -> bool {
    state == State::Downloading
}

/// Une demande de l'agent non confirmée alors qu'il fallait confirmer : l'état a changé.
fn confirmed(checked: &Checked, origin: &Origin) -> Result<(), VoiceError> {
    match origin {
        Origin::Agent { confirmed: false, .. } if checked.needs_confirmation() => Err(refused(VoiceErrorKind::Changed)),
        Origin::User | Origin::Agent { .. } => Ok(()),
    }
}

/// Un modèle de la dictée, tel que la boîte le montre.
fn model_view(m: &ModelInfo) -> ModelView {
    let kind = match m.id {
        Model::Apple => ModelKind::Apple,
        Model::Api => ModelKind::Api,
        Model::Turbo | Model::Large | Model::Base => ModelKind::Local,
    };
    ModelView { id: m.id.id().into(), kind, size: m.size, installed: m.installed, available: m.available }
}

/// La dictée telle que la boîte la montre ; une langue vide vaut la détection.
fn dictation_view(s: &Status) -> DictationView {
    let config = &s.config;
    // « Installé » pour l'API : la variable de la clé est posée (sa valeur n'est jamais lue ici).
    let key_found = s.models.iter().any(|m| m.id == Model::Api && m.installed);
    DictationView {
        model: config.model.id().into(),
        models: s.models.iter().map(model_view).collect(),
        language: config.language.clone().filter(|c| !c.is_empty()),
        detected: s.detected.clone(),
        auto_stop: config.auto_stop,
        silence: config.silence,
        api: ApiView {
            url: config.api.url.clone(),
            model: config.api.model.clone(),
            key_env: config.api.key_env.clone(),
            key_found,
        },
        download: s.download.as_ref().map(|d| DownloadView {
            model: d.model.id().into(),
            received: d.received,
            total: d.total,
        }),
        idle: s.state == State::Idle,
    }
}

/// Vérifie `patch` sur l'état `status` ; rend ce qu'il faut confirmer et les réglages qui en résultent.
fn check_patch(status: &Status, patch: &DictationPatch, origin: &Origin) -> Result<(Checked, Config), VoiceError> {
    let agent = matches!(origin, Origin::Agent { .. });
    if status.state != State::Idle {
        return Err(refused(VoiceErrorKind::Busy));
    }
    if agent && patch.touches_api_target() {
        return Err(refused(VoiceErrorKind::ApiReservedToUser));
    }
    let old = &status.config;
    let mut new = old.clone();
    if let Some(id) = &patch.model {
        new.model = Model::from_id(id).ok_or_else(|| refused(VoiceErrorKind::UnknownModel))?;
    }
    let info = status.models.iter().find(|m| m.id == new.model);
    if new.model != old.model && !info.is_some_and(|m| m.available) {
        return Err(refused(VoiceErrorKind::Unavailable));
    }
    match &patch.language {
        None => {}
        Some(DictationLanguage::Auto) => new.language = None,
        Some(DictationLanguage::Code(code)) => {
            let listed = DICTATION_LANGUAGES.iter().any(|(c, _)| c == code);
            if !listed && old.language.as_deref() != Some(code.as_str()) {
                return Err(refused(VoiceErrorKind::BadLanguage));
            }
            new.language = Some(code.clone());
        }
    }
    if let Some(auto_stop) = patch.auto_stop {
        new.auto_stop = auto_stop;
    }
    if let Some(silence) = patch.silence {
        if !SILENCE_RANGE.contains(&silence) {
            return Err(refused(VoiceErrorKind::BadSilence));
        }
        new.silence = silence;
    }
    if let Some(url) = &patch.api_url {
        new.api.url = normalized_url(url).ok_or_else(|| refused(VoiceErrorKind::BadUrl))?;
    }
    if let Some(key_env) = &patch.api_key_env {
        if !valid_key_env(key_env) {
            return Err(refused(VoiceErrorKind::BadKeyEnv));
        }
        new.api.key_env.clone_from(key_env);
    }
    if let Some(model) = &patch.api_model {
        if !valid_api_model(model) {
            return Err(refused(VoiceErrorKind::BadApiModel));
        }
        new.api.model.clone_from(model);
    }
    let mut checked = Checked { unchanged: new == *old, ..Checked::default() };
    // L'agent fait passer la dictée par l'API : l'adresse et la variable de l'utilisateur, revérifiées.
    if agent && new.model == Model::Api && old.model != Model::Api {
        let url = normalized_url(&old.api.url).ok_or_else(|| refused(VoiceErrorKind::BadUrl))?;
        if !valid_key_env(&old.api.key_env) {
            return Err(refused(VoiceErrorKind::BadKeyEnv));
        }
        let host = url_host(&url).ok_or_else(|| refused(VoiceErrorKind::BadUrl))?;
        checked.api = Some(ApiTarget { host, url, key_env: old.api.key_env.clone(), model: new.api.model.clone() });
    }
    if new.model != Model::Api
        && new.model != old.model
        && let Some(info) = info.filter(|m| !m.installed)
    {
        checked.download = Some((model_view(info), new.api.model.clone()));
    }
    Ok((checked, new))
}

/// Vérifie le téléchargement du modèle `id` ; rend ce qu'il faut confirmer et le modèle.
fn check_download(status: &Status, id: &str) -> Result<(Checked, Model), VoiceError> {
    let model = Model::from_id(id).ok_or_else(|| refused(VoiceErrorKind::UnknownModel))?;
    if model == Model::Api {
        return Err(refused(VoiceErrorKind::NotDownloadable));
    }
    let info = status
        .models
        .iter()
        .find(|m| m.id == model && m.available)
        .ok_or_else(|| refused(VoiceErrorKind::Unavailable))?;
    if info.installed {
        return Err(refused(VoiceErrorKind::Installed));
    }
    if status.state != State::Idle {
        return Err(refused(VoiceErrorKind::Busy));
    }
    let download = Some((model_view(info), status.config.api.model.clone()));
    Ok((Checked { download, ..Checked::default() }, model))
}

/// La dictée et l'agent de l'app ; un seul pour toute l'app, partagé par les hubs des onglets.
#[derive(Debug)]
pub(crate) struct AppVoice {
    /// La dictée de l'hôte ; `None` sans moteur de dictée.
    stt: Option<Stt>,
    /// L'agent choisi.
    choice: Arc<AgentChoice>,
    /// Les requêtes en cours et le changement d'agent en attente : le verrou d'état.
    switch: AgentSwitch,
    /// Avance à chaque changement que la boîte montre.
    revision: watch::Sender<u64>,
    /// Tenu pendant la lecture-écriture des réglages de la dictée.
    edit: Mutex<()>,
    /// Le runtime où part un changement d'agent différé.
    rt: Handle,
    /// Soi-même, pour les gardes et les fils.
    me: Weak<AppVoice>,
}

impl AppVoice {
    /// La dictée `stt` et le choix `choice` ; la veille de la dictée démarre aussitôt.
    pub(crate) fn new(stt: Option<Stt>, choice: Arc<AgentChoice>, rt: Handle) -> Arc<Self> {
        let voice = Arc::new_cyclic(|me| Self {
            stt: stt.clone(),
            choice,
            switch: AgentSwitch::default(),
            revision: watch::channel(0).0,
            edit: Mutex::new(()),
            rt,
            me: me.clone(),
        });
        if let Some(stt) = stt {
            crate::stt_watch::spawn(stt, Arc::downgrade(&voice));
        }
        voice
    }

    /// La révision : elle avance à chaque changement de ce que la boîte montre.
    pub(crate) fn revision(&self) -> watch::Receiver<u64> {
        self.revision.subscribe()
    }

    /// Fait avancer la révision.
    pub(crate) fn bump(&self) {
        self.revision.send_modify(|r| *r = r.wrapping_add(1));
    }

    /// Une requête à l'agent commence dans `tab` ; la garde rendue la termine.
    pub(crate) fn prompt_started(&self, tab: &str) -> PromptGuard {
        let (id, began) = self.switch.lock().prompt_started(tab);
        if began {
            self.bump();
        }
        PromptGuard::new(self.me.clone(), id)
    }

    /// La requête de la garde `id` est finie ; la dernière applique le choix en attente.
    pub(crate) fn prompt_ended(&self, id: u64) {
        let (idle, take) = self.switch.lock().prompt_ended(id);
        if idle {
            self.bump();
        }
        if let Some((tab, agent, epoch)) = take {
            let me = self.me.clone();
            // `persist` écrit `config.toml` : hors des fils du runtime.
            drop(self.rt.spawn_blocking(move || {
                if let Some(voice) = me.upgrade() {
                    // Personne n'attend la réponse : l'issue se voit dans la boîte.
                    let _outcome = voice.switch_now(&tab, &agent, epoch);
                }
            }));
        }
    }

    /// Passe à l'agent `id` demandé par `tab` à l'époque `epoch`, si rien n'a changé depuis.
    ///
    /// Revérifié sous le verrou d'état, et la bascule y est faite : aucune requête ne commence
    /// entre les deux. Puis l'agent vraiment actif est enregistré. Écrit sur le disque.
    /// Rend l'issue : basculé, resté en attente ou caduc.
    pub(crate) fn switch_now(&self, tab: &str, id: &str, epoch: u64) -> Switch {
        let outcome = {
            let mut state = self.switch.lock();
            let outcome = state.may_switch(tab, id, epoch);
            if outcome == Switch::Go {
                self.choice.switch_to(id);
            }
            outcome
        };
        match outcome {
            Switch::Stale => return outcome,
            Switch::Wait => {}
            Switch::Go => {
                if let Err(e) = self.choice.persist() {
                    tracing::warn!(error = %snafu::Report::from_error(&e), "agent actif non enregistré");
                }
            }
        }
        self.bump();
        outcome
    }

    /// L'onglet `tab` est fermé : son changement d'agent en attente tombe.
    pub(crate) fn tab_closed(&self, tab: &str) {
        if self.switch.lock().tab_closed(tab) {
            self.bump();
        }
    }

    /// Fin de l'app : plus aucun changement d'agent.
    pub(crate) fn shutdown(&self) {
        self.switch.lock().shutdown();
    }

    /// La langue courante de l'interface.
    pub(crate) fn localizer(&self) -> Arc<Localizer> {
        self.choice.localizer()
    }

    /// Pourquoi l'agent choisi est inutilisable, localisé. Peut bloquer : lit le disque.
    pub(crate) fn availability(&self) -> Option<String> {
        self.choice.availability()
    }

    /// Lance le téléchargement du modèle de l'agent local (`apply(DownloadAgentModel)`).
    ///
    /// Chaque pourcent et la fin font avancer la révision : la boîte de chaque onglet se
    /// rafraîchit (`voice_refresh`). Peut bloquer : lit le disque.
    fn download_agent_model(&self, origin: &Origin) -> Result<VoiceApplied, VoiceError> {
        confirmed(&self.check_agent_model()?, origin)?;
        let me = self.me.clone();
        let started = self.choice.start_download(move || {
            if let Some(voice) = me.upgrade() {
                voice.bump();
            }
        });
        if !started {
            return Err(refused(VoiceErrorKind::NothingToDownload));
        }
        self.bump();
        Ok(VoiceApplied::Now)
    }

    fn stt(&self) -> Result<&Stt, VoiceError> {
        self.stt.as_ref().ok_or_else(|| refused(VoiceErrorKind::NoDictation))
    }

    fn check_agent(&self, id: &str) -> Result<Checked, VoiceError> {
        self.choice.check(id).map_err(|r| match r {
            Refusal::UnknownAgent => refused(VoiceErrorKind::UnknownAgent),
            Refusal::Locked => refused(VoiceErrorKind::AgentLocked),
        })?;
        let unchanged = {
            let state = self.switch.lock();
            state.pending().is_none() && self.choice.active() == id
        };
        let agent = if unchanged { None } else { self.choice.preset_view(id) };
        Ok(Checked { unchanged, agent, ..Checked::default() })
    }

    fn check_agent_model(&self) -> Result<Checked, VoiceError> {
        match self.choice.local_panel() {
            Some(LocalModelView::NeedsDownload { size } | LocalModelView::Failed { size, .. }) => {
                Ok(Checked { agent_model_size: Some(size), ..Checked::default() })
            }
            Some(LocalModelView::Running { .. }) | None => Err(refused(VoiceErrorKind::NothingToDownload)),
        }
    }

    fn check_cancel_agent_model(&self) -> Result<Checked, VoiceError> {
        match self.choice.local_panel() {
            Some(LocalModelView::Running { .. }) => Ok(Checked::default()),
            Some(LocalModelView::NeedsDownload { .. } | LocalModelView::Failed { .. }) | None => {
                Err(refused(VoiceErrorKind::NothingToCancel))
            }
        }
    }

    fn check_cancel_dictation(&self) -> Result<(&Stt, Checked), VoiceError> {
        let stt = self.stt()?;
        if !cancel_dictation_allowed(stt.status().state) {
            return Err(refused(VoiceErrorKind::NothingToCancel));
        }
        Ok((stt, Checked::default()))
    }

    fn apply_patch(&self, patch: &DictationPatch, origin: &Origin) -> Result<VoiceApplied, VoiceError> {
        let stt = self.stt()?;
        let applied = {
            let _edit = self.edit.lock().unwrap_or_else(PoisonError::into_inner);
            let (checked, config) = check_patch(&stt.status(), patch, origin)?;
            confirmed(&checked, origin)?;
            if checked.unchanged {
                return Ok(VoiceApplied::Unchanged);
            }
            stt.configure(config)
        };
        if !applied {
            return Err(refused(VoiceErrorKind::Busy));
        }
        self.bump();
        Ok(VoiceApplied::Now)
    }

    fn apply_download(&self, id: &str, origin: &Origin) -> Result<VoiceApplied, VoiceError> {
        let stt = self.stt()?;
        let (checked, model) = check_download(&stt.status(), id)?;
        confirmed(&checked, origin)?;
        if !stt.download(model, false) {
            let busy = stt.status().state != State::Idle;
            return Err(refused(if busy { VoiceErrorKind::Busy } else { VoiceErrorKind::Installed }));
        }
        self.bump();
        Ok(VoiceApplied::Now)
    }

    /// Un clic de l'utilisateur : `agent.selected` écrit d'abord, puis la bascule, sans autre écriture entre.
    ///
    /// Un échec d'écriture ne bascule pas : l'agent montré reste celui qui est enregistré.
    fn user_select(&self, id: &str) -> Result<VoiceApplied, VoiceError> {
        {
            let mut state = self.switch.lock();
            if self.choice.active() == id {
                let had_pending = state.user_select();
                drop(state);
                if had_pending {
                    self.bump();
                }
                return Ok(VoiceApplied::Unchanged);
            }
        }
        let saved = self.choice.save_then(id, || {
            let mut state = self.switch.lock();
            state.user_select();
            self.choice.switch_to(id);
        });
        if let Err(e) = saved {
            tracing::warn!(error = %snafu::Report::from_error(&e), "choix d'agent non enregistré : rien ne bascule");
            return Err(refused(VoiceErrorKind::AgentNotSaved));
        }
        self.bump();
        Ok(VoiceApplied::Now)
    }

    /// Une demande de l'agent de `tab` : tout de suite si rien ne tourne, sinon en attente.
    fn agent_select(&self, tab: &str, id: &str) -> VoiceApplied {
        let (requested, pending_changed) = {
            let mut state = self.switch.lock();
            let before = state.pending();
            let requested = state.request(tab, id, &self.choice.active());
            (requested, before != state.pending())
        };
        match requested {
            Requested::Now(epoch) => applied_of(self.switch_now(tab, id, epoch)),
            Requested::WhenIdle => {
                self.bump();
                VoiceApplied::WhenIdle
            }
            Requested::Unchanged => {
                if pending_changed {
                    self.bump();
                }
                VoiceApplied::Unchanged
            }
        }
    }

    /// L'époque des changements d'agent.
    #[cfg(test)]
    pub(crate) fn switch_epoch(&self) -> u64 {
        self.switch.lock().epoch()
    }

    /// L'époque du lancement de l'agent : elle avance à chaque bascule.
    #[cfg(test)]
    pub(crate) fn link_epoch(&self) -> u64 {
        self.choice.launch_epoch()
    }
}

impl VoicePort for AppVoice {
    fn view(&self) -> VoiceView {
        let dictation = self.stt.as_ref().map(|stt| dictation_view(&stt.status()));
        let (busy, pending) = {
            let state = self.switch.lock();
            (state.busy(), state.pending())
        };
        let mut agents = self.choice.agents_view();
        agents.busy = busy;
        agents.pending = pending;
        VoiceView { dictation, agents }
    }

    fn check(&self, event: &VoiceEvent, origin: &Origin) -> Result<Checked, VoiceError> {
        match event {
            VoiceEvent::Dictation(patch) => check_patch(&self.stt()?.status(), patch, origin).map(|(c, _)| c),
            VoiceEvent::DownloadDictation { model } => check_download(&self.stt()?.status(), model).map(|(c, _)| c),
            VoiceEvent::CancelDictationDownload => self.check_cancel_dictation().map(|(_, c)| c),
            VoiceEvent::SelectAgent { id } => self.check_agent(id),
            VoiceEvent::DownloadAgentModel => self.check_agent_model(),
            VoiceEvent::CancelAgentModelDownload => self.check_cancel_agent_model(),
        }
    }

    fn apply(&self, event: VoiceEvent, origin: Origin) -> Result<VoiceApplied, VoiceError> {
        match event {
            VoiceEvent::Dictation(patch) => self.apply_patch(&patch, &origin),
            VoiceEvent::DownloadDictation { model } => self.apply_download(&model, &origin),
            VoiceEvent::CancelDictationDownload => {
                let (stt, _) = self.check_cancel_dictation()?;
                stt.cancel();
                self.bump();
                Ok(VoiceApplied::Now)
            }
            VoiceEvent::SelectAgent { id } => {
                confirmed(&self.check_agent(&id)?, &origin)?;
                match origin {
                    Origin::User => self.user_select(&id),
                    Origin::Agent { tab, .. } => Ok(self.agent_select(&tab, &id)),
                }
            }
            VoiceEvent::DownloadAgentModel => self.download_agent_model(&origin),
            VoiceEvent::CancelAgentModelDownload => {
                self.check_cancel_agent_model()?;
                self.choice.cancel_download();
                self.bump();
                Ok(VoiceApplied::Now)
            }
        }
    }
}

#[cfg(test)]
#[path = "voice_port_tests.rs"]
pub(crate) mod voice_port_tests;
