//! Le fil gardé d'un onglet, rejoué à l'interface par `resync`.
//!
//! Il suit le réducteur de l'interface (`ui/src/agent/thread.ts`) : les morceaux
//! d'une réponse se fusionnent, une mise à jour d'outil se replie dans son appel,
//! seul le dernier plan reste, et l'état courant est rejoué en dernier.

use std::collections::VecDeque;

use crate::AgentEvent;

/// Entrées du fil gardées au plus : l'interface rechargée retrouve la fin du fil.
const KEPT: usize = 200;

/// Les dernières entrées du fil d'un onglet, depuis la dernière conversation.
#[derive(Debug, Default)]
pub(crate) struct ThreadLog {
    entries: VecDeque<AgentEvent>,
    /// Le dernier `Status`, hors du fil : jamais évincé par un long tour.
    status: Option<AgentEvent>,
}

impl ThreadLog {
    /// Garde `e` ; `Conversation` vide d'abord le fil (puis se garde).
    pub fn push(&mut self, e: &AgentEvent) {
        match e {
            AgentEvent::Status { .. } => self.status = Some(e.clone()),
            AgentEvent::Thread { .. } => {}
            AgentEvent::Chunk { text } => {
                if let Some(AgentEvent::Chunk { text: last }) = self.entries.back_mut() {
                    last.push_str(text);
                } else {
                    self.append(e.clone());
                }
            }
            AgentEvent::ToolUpdate { id, title, status } => {
                // Sans appel gardé, la mise à jour est ignorée, comme dans l'interface.
                for entry in &mut self.entries {
                    if let AgentEvent::ToolCall { id: call, title: t, status: s } = entry
                        && call == id
                    {
                        if let Some(title) = title {
                            t.clone_from(title);
                        }
                        if let Some(status) = status {
                            s.clone_from(status);
                        }
                    }
                }
            }
            AgentEvent::Plan { .. } => match self.entries.iter_mut().find(|x| matches!(x, AgentEvent::Plan { .. })) {
                Some(plan) => *plan = e.clone(),
                None => self.append(e.clone()),
            },
            AgentEvent::Conversation => {
                self.entries.clear();
                self.append(e.clone());
            }
            _ => self.append(e.clone()),
        }
    }

    fn append(&mut self, e: AgentEvent) {
        if self.entries.len() == KEPT {
            self.entries.pop_front();
        }
        self.entries.push_back(e);
    }

    /// Le fil à rejouer, l'état courant en dernier.
    pub fn entries(&self) -> Vec<AgentEvent> {
        self.entries.iter().chain(&self.status).cloned().collect()
    }
}

#[cfg(test)]
#[path = "thread_log_tests.rs"]
mod thread_log_tests;
