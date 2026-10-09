//! Les conversations de l'agent gardées par la session.
//!
//! Une par portée, écrite dans `agent/<portée>.json` du dossier de reprise. La
//! session est le seul écrivain ; dans l'agent, le gardien de l'onglet le seul appelant.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{AgentScope, Surface};
use crate::Session;

/// Une conversation : son fil et sa session ACP.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedConversation {
    /// La portée de la conversation.
    pub scope: AgentScope,
    /// Empreinte de l'agent qui tient la session (`Link::agent_id`) ; opaque ici.
    #[serde(default)]
    pub agent: Option<String>,
    /// Identifiant de la session ACP.
    #[serde(default)]
    pub session: Option<String>,
    /// Les surfaces figées du fil, sans actions.
    #[serde(default)]
    pub thread: Vec<Surface>,
}

impl SavedConversation {
    fn empty(scope: AgentScope) -> Self {
        Self { scope, agent: None, session: None, thread: Vec::new() }
    }
}

/// Une conversation déplacée par la session ; le gardien la fait suivre au hub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeMove {
    /// Le widget a été renommé.
    Renamed {
        /// L'ancienne portée.
        from: AgentScope,
        /// La nouvelle portée.
        to: AgentScope,
    },
}

/// Les conversations d'une session et les déplacements pas encore lus.
#[derive(Debug, Default)]
pub(crate) struct Conversations {
    pub(crate) by_scope: BTreeMap<AgentScope, SavedConversation>,
    pub(crate) moves: Vec<ScopeMove>,
}

impl Session {
    /// Les conversations gardées.
    pub fn conversations(&self) -> Vec<SavedConversation> {
        self.conversations.by_scope.values().cloned().collect()
    }

    /// La conversation de `scope`, si elle existe.
    pub fn conversation(&self, scope: &AgentScope) -> Option<&SavedConversation> {
        self.conversations.by_scope.get(scope)
    }

    /// Le fil de `scope` (déjà sans surface vivante ni action) ; écrit pour la reprise.
    pub fn keep_thread(&mut self, scope: AgentScope, thread: Vec<Surface>) {
        if !self.writable(&scope) {
            tracing::warn!(?scope, "portée inconnue : conversation non enregistrée");
            return;
        }
        self.conversation_mut(&scope).thread = thread;
        self.persist_conversation(&scope);
    }

    /// La session ACP de `scope` et l'empreinte de son agent ; écrites pour la reprise.
    pub fn keep_agent_session(&mut self, scope: AgentScope, session: String, agent: String) {
        if !self.writable(&scope) {
            tracing::warn!(?scope, "portée inconnue : conversation non enregistrée");
            return;
        }
        let c = self.conversation_mut(&scope);
        c.session = Some(session);
        c.agent = Some(agent);
        self.persist_conversation(&scope);
    }

    /// Nouvelle conversation dans `scope` : oubliée, son fichier effacé ; rend sa session.
    pub fn forget_conversation(&mut self, scope: &AgentScope) -> Option<String> {
        let gone = self.conversations.by_scope.remove(scope)?;
        self.persist_agent(scope.clone(), None);
        gone.session
    }

    /// Les déplacements de conversations depuis le dernier appel.
    pub fn take_scope_moves(&mut self) -> Vec<ScopeMove> {
        std::mem::take(&mut self.conversations.moves)
    }

    /// Une portée qu'on peut écrire : les slides, un atelier, ou un widget connu de son pack.
    fn writable(&self, scope: &AgentScope) -> bool {
        match scope {
            AgentScope::Slides | AgentScope::Atelier { .. } => true,
            AgentScope::Widget { pack, name } => self.widget_known(*pack, name),
        }
    }

    /// Le widget `from` du pack `pack` s'appelle désormais `to` : sa conversation le suit.
    pub(crate) fn conversation_renamed(&mut self, pack: uuid::Uuid, from: &str, to: &str) {
        let old = AgentScope::Widget { pack, name: from.to_owned() };
        let new = AgentScope::Widget { pack, name: to.to_owned() };
        if let Some(mut c) = self.conversations.by_scope.remove(&old) {
            c.scope = new.clone();
            self.conversations.by_scope.insert(new.clone(), c);
            self.persist_agent(old.clone(), None);
            self.persist_conversation(&new);
        }
        self.conversations.moves.push(ScopeMove::Renamed { from: old, to: new });
    }

    /// Efface les conversations dont le widget manque à tous les fichiers trouvés de son pack.
    ///
    /// Prudent : un pack dont aucun fichier n'est trouvé ne fait rien effacer.
    pub(crate) fn prune_conversations(&mut self) {
        let gone: Vec<AgentScope> = self
            .conversations
            .by_scope
            .keys()
            .filter(|scope| match scope {
                AgentScope::Widget { pack, name } => {
                    !self.pack_files_known(*pack).is_empty() && !self.widget_known(*pack, name)
                }
                AgentScope::Slides | AgentScope::Atelier { .. } => false,
            })
            .cloned()
            .collect();
        for scope in gone {
            tracing::debug!(?scope, "conversation d'un widget disparu : effacée");
            self.forget_conversation(&scope);
        }
    }

    fn conversation_mut(&mut self, scope: &AgentScope) -> &mut SavedConversation {
        self.conversations.by_scope.entry(scope.clone()).or_insert_with(|| SavedConversation::empty(scope.clone()))
    }

    pub(crate) fn persist_conversation(&mut self, scope: &AgentScope) {
        let saved = self.conversations.by_scope.get(scope).cloned();
        self.persist_agent(scope.clone(), saved);
    }
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod conversation_tests;
