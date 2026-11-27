//! Le changement d'agent demandé par l'agent : requêtes en cours, attente, époque.
//!
//! Une seule connexion ACP sert tous les onglets, et changer d'agent la coupe : un changement
//! demandé par un outil attend qu'aucune requête ne tourne. Chaque onglet qui interroge l'agent
//! tient une [`PromptGuard`] ; la dernière lâchée applique le choix en attente, revérifié sous
//! le verrou d'état d'`AppVoice` avec l'époque (un clic de l'utilisateur la fait avancer).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError, Weak};

use crate::voice_port::AppVoice;

/// Tenue par un onglet tant qu'il interroge l'agent ; la lâcher le dit à `AppVoice`.
#[derive(Debug)]
pub(crate) struct PromptGuard {
    voice: Weak<AppVoice>,
    id: u64,
}

impl PromptGuard {
    /// La garde `id` d'`voice`.
    pub(crate) fn new(voice: Weak<AppVoice>, id: u64) -> Self {
        Self { voice, id }
    }
}

impl Drop for PromptGuard {
    fn drop(&mut self) {
        if let Some(voice) = self.voice.upgrade() {
            voice.prompt_ended(self.id);
        }
    }
}

/// Ce que décide une demande de changement d'agent venue d'un outil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Requested {
    /// C'est déjà l'agent actif : le choix en attente est effacé.
    Unchanged,
    /// Rien ne tourne : à appliquer tout de suite, à cette époque.
    Now(u64),
    /// Une requête tourne : en attente.
    WhenIdle,
}

/// Ce que peut faire un changement différé quand vient son tour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Switch {
    /// Rien ne tourne et rien n'a changé : basculer.
    Go,
    /// Caduc : l'époque a avancé ou l'app s'arrête.
    Stale,
    /// Une requête vient de commencer : le choix reste en attente.
    Wait,
}

/// Les requêtes en cours, le choix en attente et l'époque, sous le verrou d'état.
#[derive(Debug, Default)]
pub(crate) struct SwitchState {
    /// Les gardes vivantes : identifiant → onglet.
    running: HashMap<u64, String>,
    /// Le dernier identifiant de garde donné.
    next: u64,
    /// L'onglet qui a demandé et l'agent demandé, en attente.
    pending: Option<(String, String)>,
    /// Avance à chaque demande et à chaque clic : un changement d'une époque passée est caduc.
    epoch: u64,
    /// L'app s'arrête : plus aucun changement.
    shut: bool,
}

impl SwitchState {
    /// Une requête commence dans `tab` ; vrai si plus rien ne tournait avant elle.
    pub(crate) fn prompt_started(&mut self, tab: &str) -> (u64, bool) {
        self.next += 1;
        let began = self.running.is_empty();
        self.running.insert(self.next, tab.to_owned());
        (self.next, began)
    }

    /// La requête `id` est finie ; vrai si plus rien ne tourne, avec le choix à appliquer alors.
    pub(crate) fn prompt_ended(&mut self, id: u64) -> (bool, Option<(String, String, u64)>) {
        if self.running.remove(&id).is_none() || !self.running.is_empty() {
            return (false, None);
        }
        let take = if self.shut { None } else { self.pending.take() };
        (true, take.map(|(tab, agent)| (tab, agent, self.epoch)))
    }

    /// L'onglet `tab` est fermé : son choix en attente tombe ; vrai si c'en était un.
    pub(crate) fn tab_closed(&mut self, tab: &str) -> bool {
        let dropped = self.pending.as_ref().is_some_and(|(t, _)| t == tab);
        if dropped {
            self.pending = None;
        }
        dropped
    }

    /// Fin de l'app : le choix en attente tombe, plus rien ne change.
    pub(crate) fn shutdown(&mut self) {
        self.shut = true;
        self.pending = None;
    }

    /// L'onglet `tab` demande l'agent `id` ; `active` est l'agent actif.
    pub(crate) fn request(&mut self, tab: &str, id: &str, active: &str) -> Requested {
        self.epoch += 1;
        if id == active {
            self.pending = None;
            Requested::Unchanged
        } else if self.running.is_empty() {
            Requested::Now(self.epoch)
        } else {
            self.pending = Some((tab.to_owned(), id.to_owned()));
            Requested::WhenIdle
        }
    }

    /// Un clic de l'utilisateur : il l'emporte sur tout choix en attente ; vrai s'il y en avait un.
    pub(crate) fn user_select(&mut self) -> bool {
        self.epoch += 1;
        self.pending.take().is_some()
    }

    /// Le changement vers `id` de l'époque `epoch` peut-il se faire maintenant ?
    ///
    /// Caduc après la fin de l'app ou si l'époque a avancé ; tant qu'une requête tourne,
    /// le choix reste en attente.
    pub(crate) fn may_switch(&mut self, tab: &str, id: &str, epoch: u64) -> Switch {
        if self.shut || epoch != self.epoch {
            return Switch::Stale;
        }
        if !self.running.is_empty() {
            self.pending = Some((tab.to_owned(), id.to_owned()));
            return Switch::Wait;
        }
        Switch::Go
    }

    /// Une requête tourne dans un onglet.
    pub(crate) fn busy(&self) -> bool {
        !self.running.is_empty()
    }

    /// L'agent en attente.
    pub(crate) fn pending(&self) -> Option<String> {
        self.pending.as_ref().map(|(_, id)| id.clone())
    }

    /// L'époque courante.
    #[cfg(test)]
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }
}

/// Le verrou d'état d'`AppVoice` : mémoire seulement, jamais d'entrée-sortie dessous.
#[derive(Debug, Default)]
pub(crate) struct AgentSwitch {
    state: Mutex<SwitchState>,
}

impl AgentSwitch {
    /// L'état, même si un fil a paniqué en le tenant.
    pub(crate) fn lock(&self) -> MutexGuard<'_, SwitchState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
#[path = "agent_switch_tests.rs"]
mod agent_switch_tests;
