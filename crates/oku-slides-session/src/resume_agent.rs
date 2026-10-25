//! La conversation de l'agent dans le dossier de reprise : `agent/conversation.json`.

use serde::Serialize;
use snafu::ResultExt;

use crate::Session;
use crate::agent::SavedConversation;
use crate::resume::{AccessSnafu, Result, Resume};
use crate::resume_writer::AgentWrite;

/// Sous-dossier de la conversation, dans le dossier de reprise de l'onglet.
pub(crate) const AGENT_DIR: &str = "agent";
/// Fichier de la conversation de l'onglet.
const CONVERSATION_FILE: &str = "conversation.json";
/// Version of the conversation file.
///
/// Version 1, one per scope, is no longer read back.
const CONVERSATION_VERSION: u32 = 2;

#[derive(Serialize)]
struct ConversationOut<'a> {
    version: u32,
    #[serde(flatten)]
    conversation: &'a SavedConversation,
}

impl Resume {
    pub(crate) fn write_conversation(&self, c: &SavedConversation) -> Result<()> {
        let out = ConversationOut { version: CONVERSATION_VERSION, conversation: c };
        self.write_json(&format!("{AGENT_DIR}/{CONVERSATION_FILE}"), &out)
    }

    pub(crate) fn delete_conversation(&self) -> Result<()> {
        let path = self.dir.join(AGENT_DIR).join(CONVERSATION_FILE);
        self.storage.delete(&path).context(AccessSnafu { path: &path })
    }

    /// La conversation relue ; illisible, journalisée et ignorée.
    pub(crate) fn read_conversation(&self) -> Option<SavedConversation> {
        // `read_json` vérifie la version à part ; le champ `version` est ignoré ici.
        match self.read_json::<SavedConversation>(&format!("{AGENT_DIR}/{CONVERSATION_FILE}"), CONVERSATION_VERSION) {
            Ok(found) => found,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "conversation illisible : ignorée");
                None
            }
        }
    }
}

impl Session {
    /// La conversation à écrire (`Some`) ou effacer (`None`), avec l'état courant.
    pub(crate) fn persist_agent(&mut self, saved: Option<SavedConversation>) {
        let write = match saved {
            Some(c) => AgentWrite::Write(c),
            None => AgentWrite::Delete,
        };
        self.persist_with(false, Some(write));
    }
}
