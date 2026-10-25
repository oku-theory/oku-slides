//! La conversation de l'agent gardée par la session : une par onglet.
//!
//! Écrite dans `agent/conversation.json` du dossier de reprise. La session est
//! le seul écrivain ; dans l'agent, le gardien de l'onglet le seul appelant.

use serde::{Deserialize, Serialize};

use super::Surface;
use crate::Session;

/// Une conversation : son fil et sa session ACP.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SavedConversation {
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

impl Session {
    /// La conversation de l'onglet, si elle existe.
    pub fn conversation(&self) -> Option<&SavedConversation> {
        self.conversation.as_ref()
    }

    /// Le fil (déjà sans surface vivante ni action) ; écrit pour la reprise.
    pub fn keep_thread(&mut self, thread: Vec<Surface>) {
        self.conversation.get_or_insert_with(SavedConversation::default).thread = thread;
        self.persist_conversation();
    }

    /// La session ACP et l'empreinte de son agent ; écrites pour la reprise.
    pub fn keep_agent_session(&mut self, session: String, agent: String) {
        let c = self.conversation.get_or_insert_with(SavedConversation::default);
        c.session = Some(session);
        c.agent = Some(agent);
        self.persist_conversation();
    }

    /// Nouvelle conversation : oubliée, son fichier effacé ; rend sa session.
    pub fn forget_conversation(&mut self) -> Option<String> {
        let gone = self.conversation.take()?;
        self.persist_agent(None);
        gone.session
    }

    fn persist_conversation(&mut self) {
        let saved = self.conversation.clone();
        self.persist_agent(saved);
    }
}

#[cfg(test)]
#[path = "conversation_tests.rs"]
mod conversation_tests;
