//! Les conversations de l'agent gardées par la session.
//!
//! Une par portée, écrite dans `agent/<portée>.json` du dossier de reprise. La
//! session est le seul écrivain ; dans l'agent, le gardien de l'onglet le seul appelant.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{AgentScope, Surface};
use crate::widgets_edit::renamed_source;
use crate::{Mode, Session};

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

/// Les widgets du pack en cours avant une étape d'annulation : de quoi reconnaître
/// un renommage défait ou refait (voir `Session::follow_widget_rename`).
pub(crate) struct WidgetsBefore {
    pack: uuid::Uuid,
    /// Sources des widgets du pack en cours, par nom.
    sources: BTreeMap<String, String>,
    /// Noms des widgets des fichiers du pack tenus par la session (collision ou
    /// conversation périmée ; la bibliothèque est lue après, au besoin).
    known: BTreeSet<String>,
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
    ///
    /// Une conversation déjà sous `to` est périmée si aucun widget `to` n'existait
    /// avant (`to_existed`, dans un fichier connu du pack) : effacée, celle de `from`
    /// (s'il en a une) prend sa place ; le widget renommé n'hérite jamais de celle
    /// du widget disparu. Sinon, collision : rien ne bouge (ni conversation, ni
    /// déplacement noté), un diagnostic est journalisé.
    pub(crate) fn conversation_renamed(&mut self, pack: uuid::Uuid, from: &str, to: &str, to_existed: bool) {
        let old = AgentScope::Widget { pack, name: from.to_owned() };
        let new = AgentScope::Widget { pack, name: to.to_owned() };
        if self.conversations.by_scope.contains_key(&new) {
            if to_existed {
                tracing::warn!(?old, ?new, "le nouveau nom a déjà une conversation : rien n'est déplacé");
                return;
            }
            tracing::debug!(?new, "conversation périmée d'un widget disparu : effacée");
            self.conversations.by_scope.remove(&new);
            self.persist_agent(new.clone(), None);
        }
        if let Some(mut c) = self.conversations.by_scope.remove(&old) {
            c.scope = new.clone();
            self.conversations.by_scope.insert(new.clone(), c);
            self.persist_agent(old.clone(), None);
            self.persist_conversation(&new);
        }
        self.conversations.moves.push(ScopeMove::Renamed { from: old, to: new });
    }

    /// Avant annuler ou rétablir : le pack en cours, les sources de ses widgets et
    /// les noms connus de ses fichiers. `None` : ni deck ni modèles.
    pub(crate) fn widgets_before(&self) -> Option<WidgetsBefore> {
        let pack = self.widgets_pack()?;
        // Sans lire la bibliothèque (pas d'accès disque à chaque annulation) : sa
        // version courante, qu'annuler ne change pas, est lue en cas de conflit seulement.
        let known = self.pack_files_held(pack);
        let known = known.iter().flat_map(|f| f.widgets().keys().map(|n| n.as_str().to_owned())).collect();
        Some(WidgetsBefore { pack, sources: self.widget_sources(), known })
    }

    /// Le pack dont les widgets sont en cours : celui de l'édition, sinon le pack
    /// propre du deck (`deck`) ; `None` sans lui, dans l'aide et l'historique.
    fn widgets_pack(&self) -> Option<uuid::Uuid> {
        match &self.mode {
            Mode::Templates { edit, .. } => Some(edit.set),
            Mode::Deck => self.deck_pins().get(&oku_slides_core::Namespace::deck()).map(|pin| pin.id),
            Mode::Help { .. } | Mode::History { .. } => None,
        }
    }

    /// Les sources des widgets du pack en cours, par nom (sans `_commun`).
    fn widget_sources(&self) -> BTreeMap<String, String> {
        let widgets = self.pack_files_in_use().map(|f| f.widgets().iter());
        widgets
            .into_iter()
            .flatten()
            .filter(|(n, _)| !n.is_common())
            .map(|(n, source)| (n.as_str().to_owned(), source.clone()))
            .collect()
    }

    /// Après annuler ou rétablir : un renommage défait ou refait, la conversation suit,
    /// comme pour `rename_widget`.
    ///
    /// Reconnu seulement si un nom `from` a disparu, un seul `to` est apparu dans le
    /// même pack, et la source de `to` est celle de `from` renommée comme le fait
    /// `rename_widget`. Un renommage fondu avec une autre modification n'est pas
    /// reconnu : mieux vaut ne rien déplacer que ranger une conversation sous un
    /// autre widget.
    pub(crate) fn follow_widget_rename(&mut self, before: Option<WidgetsBefore>) {
        let Some(before) = before else { return };
        let Some(pack) = self.widgets_pack().filter(|p| *p == before.pack) else { return };
        let after = self.widget_sources();
        let mut gone = before.sources.iter().filter(|(n, _)| !after.contains_key(*n));
        let mut added = after.iter().filter(|(n, _)| !before.sources.contains_key(*n));
        let (Some((from, source)), None, Some((to, now)), None) =
            (gone.next(), gone.next(), added.next(), added.next())
        else {
            return;
        };
        if renamed_source(source, from, to) != *now {
            tracing::debug!(from, to, "un widget retiré, un autre ajouté : pas un renommage");
            return;
        }
        let (from, to) = (from.clone(), to.clone());
        let taken = self.conversations.by_scope.contains_key(&AgentScope::Widget { pack, name: to.clone() });
        let to_existed = taken
            && (before.known.contains(&to)
                || self
                    .library
                    .current_version(pack)
                    .is_some_and(|v| v.files.widgets().keys().any(|n| n.as_str() == to)));
        self.conversation_renamed(pack, &from, &to, to_existed);
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
