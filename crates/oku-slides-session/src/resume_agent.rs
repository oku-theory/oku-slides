//! Les conversations de l'agent dans le dossier de reprise : `agent/<portée>.json`.

use std::collections::BTreeMap;

use serde::Serialize;
use snafu::ResultExt;

use crate::Session;
use crate::agent::{AgentScope, SavedConversation};
use crate::resume::{AccessSnafu, Result, Resume};

/// Sous-dossier des conversations, dans le dossier de reprise de l'onglet.
pub(crate) const AGENT_DIR: &str = "agent";
/// Version des fichiers `agent/*.json`.
pub(crate) const CONVERSATION_VERSION: u32 = 1;

#[derive(Serialize)]
struct ConversationOut<'a> {
    version: u32,
    #[serde(flatten)]
    conversation: &'a SavedConversation,
}

impl Resume {
    pub(crate) fn write_conversation(&self, c: &SavedConversation) -> Result<()> {
        let out = ConversationOut { version: CONVERSATION_VERSION, conversation: c };
        self.write_json(&format!("{AGENT_DIR}/{}", c.scope.file_name()), &out)
    }

    pub(crate) fn delete_conversation(&self, scope: &AgentScope) -> Result<()> {
        let path = self.dir.join(AGENT_DIR).join(scope.file_name());
        self.storage.delete(&path).context(AccessSnafu { path: &path })
    }

    /// Les conversations relues ; un fichier illisible ou mal nommé est journalisé et sauté.
    pub(crate) fn read_conversations(&self) -> BTreeMap<AgentScope, SavedConversation> {
        let dir = self.dir.join(AGENT_DIR);
        let files = match self.storage.list_files(&dir) {
            Ok(files) => files,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "conversations de l'agent illisibles");
                Vec::new()
            }
        };
        let mut out = BTreeMap::new();
        for path in files {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if name.starts_with('.') || !name.ends_with(".json") {
                continue;
            }
            // `read_json` vérifie la version à part ; le champ `version` est ignoré ici.
            match self.read_json::<SavedConversation>(&format!("{AGENT_DIR}/{name}"), CONVERSATION_VERSION) {
                Ok(Some(c)) if c.scope.file_name() == name => {
                    out.insert(c.scope.clone(), c);
                }
                Ok(Some(_)) => tracing::warn!(file = name, "conversation sous un autre nom : ignorée"),
                Ok(None) => tracing::debug!(file = name, "conversation disparue"),
                Err(e) => {
                    tracing::warn!(file = name, error = %oku_slides_platform::errors::one_line(&e), "conversation illisible : ignorée")
                }
            }
        }
        out
    }
}

impl Session {
    /// Une conversation à écrire (`Some`) ou effacer (`None`), avec l'état courant.
    pub(crate) fn persist_agent(&mut self, scope: AgentScope, saved: Option<SavedConversation>) {
        self.persist_with(false, BTreeMap::from([(scope, saved)]));
    }
}
