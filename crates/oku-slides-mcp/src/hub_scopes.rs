//! Portées du fil : surfaces montrées, restauration et image du fil.
//!
//! Le hub ne touche jamais à la session : le gardien de l'onglet (agent) lit et écrit.

use std::collections::{BTreeSet, HashSet};

use oku_slides_session::agent::{AgentScope, KEPT_TURNS, Place, SavedConversation, Surface};

use super::{HubState, McpEvent, SurfaceHub};

/// Octets au plus du fil d'une portée enregistré (sous la limite de lecture de 1 Mio).
pub const THREAD_BYTES: usize = 768 * 1024;
/// Plus grand numéro de tour ou d'identifiant accepté à la relecture.
const MAX_RESTORED: u64 = 1_000_000;

/// Le numéro d'un identifiant `préfixe-N`.
fn id_number(id: &str) -> Option<u64> {
    id.rsplit_once('-').and_then(|(_, n)| n.parse().ok())
}

/// Une surface relue, prête à remettre : du fil, numérotée, figée, sans actions.
fn cleaned(s: Surface) -> Option<Surface> {
    let ok = s.place == Place::Thread
        && s.turn.is_some_and(|t| u64::from(t) <= MAX_RESTORED)
        && id_number(&s.id).is_some_and(|n| n <= MAX_RESTORED);
    ok.then(|| {
        let mut s = s.without_actions();
        s.live = false;
        s.question = false;
        s
    })
}

impl SurfaceHub {
    /// Montrée : vivante, hors tour, ou dans le fil vu.
    pub(super) fn shown(state: &HubState, live: bool, turn: Option<u32>) -> bool {
        live || turn.is_none_or(|t| state.turn_scopes.get(&t) == Some(&state.viewing))
    }

    pub(super) fn visible(state: &HubState) -> Vec<Surface> {
        state.open.iter().filter(|s| Self::shown(state, s.live, s.turn)).cloned().collect()
    }

    /// Les surfaces que l'interface montre, dans l'ordre d'ouverture.
    pub fn visible_surfaces(&self) -> Vec<Surface> {
        Self::visible(&self.state())
    }

    /// La portée du tour `turn`, s'il est encore dans le fil.
    pub fn turn_scope(&self, turn: u32) -> Option<AgentScope> {
        self.state().turn_scopes.get(&turn).cloned()
    }

    /// Le widget renommé : ses tours, et la portée vue, le suivent ; piloté par le gardien.
    ///
    /// Aucune liste n'est renvoyée : ce que l'interface montre ne change pas. Si le widget renommé est
    /// celui qui est montré, la nouvelle portée vue est annoncée (`McpEvent::Viewing`).
    pub fn rename_scope(&self, from: &AgentScope, to: &AgentScope) {
        let mut state = self.state();
        for s in state.turn_scopes.values_mut().filter(|s| *s == from) {
            *s = to.clone();
        }
        if state.viewing == *from {
            state.viewing = to.clone();
            self.record_and_send(&mut state, McpEvent::Viewing { scope: to.clone() });
        }
    }

    /// Remet les fils relus, nettoyés ; piloté par le gardien, avant tout tour.
    ///
    /// Un tour déjà pris (tour ouvert, autre portée) est ignoré ; les suivants sont numérotés après.
    pub fn restore(&self, saved: Vec<SavedConversation>) {
        {
            let mut state = self.state();
            let mut seen: HashSet<String> = state.open.iter().map(|s| s.id.clone()).collect();
            for c in saved {
                let mut thread: Vec<Surface> = c.thread.into_iter().filter_map(cleaned).collect();
                let turns: BTreeSet<u32> = thread.iter().filter_map(|s| s.turn).collect();
                let first = turns.iter().rev().nth(KEPT_TURNS - 1).copied().unwrap_or(0);
                thread.retain(|s| s.turn.is_some_and(|t| t >= first));
                for s in thread {
                    let Some(t) = s.turn else { continue };
                    let taken = state.turn_scopes.get(&t).is_some_and(|x| *x != c.scope)
                        || (state.live_turns.contains(&t) && !state.turn_scopes.contains_key(&t))
                        || state.turn == Some(t);
                    if taken || !seen.insert(s.id.clone()) {
                        tracing::warn!(surface = %s.id, turn = t, "surface relue en conflit : ignorée");
                        continue;
                    }
                    state.turn_scopes.insert(t, c.scope.clone());
                    state.live_turns.insert(t);
                    state.turns = state.turns.max(t);
                    state.next = state.next.max(id_number(&s.id).unwrap_or(0));
                    state.open.push(s);
                }
            }
        }
        self.emit(McpEvent::Surfaces { list: Vec::new() });
    }

    /// Le fil de `scope` à enregistrer : derniers tours clos, figés, sans actions, dans le budget.
    ///
    /// `None` si rien ne tient dans le budget : l'historique enregistré reste tel quel.
    pub fn thread_snapshot(&self, scope: &AgentScope) -> Option<Vec<Surface>> {
        let state = self.state();
        let ours = |s: &&Surface| {
            !s.live
                && !s.question
                && s.place == Place::Thread
                && s.turn.is_some_and(|t| state.turn != Some(t))
                && s.turn.and_then(|t| state.turn_scopes.get(&t)).is_some_and(|x| x == scope)
        };
        let turns: BTreeSet<u32> = state.open.iter().filter(ours).filter_map(|s| s.turn).collect();
        if turns.is_empty() {
            return Some(Vec::new());
        }
        let size = |s: &Surface| serde_json::to_vec(&s.without_actions()).map_or(0, |b| b.len());
        let (mut first, mut bytes) = (None, 0usize);
        for &t in turns.iter().rev().take(KEPT_TURNS) {
            let turn_bytes: usize = state.open.iter().filter(ours).filter(|s| s.turn == Some(t)).map(size).sum();
            if bytes + turn_bytes > THREAD_BYTES {
                break;
            }
            bytes += turn_bytes;
            first = Some(t);
        }
        let first = first?;
        Some(
            state
                .open
                .iter()
                .filter(ours)
                .filter(|s| s.turn.is_some_and(|t| t >= first))
                .map(Surface::without_actions)
                .collect(),
        )
    }
}

#[cfg(test)]
#[path = "hub_scopes_tests.rs"]
mod hub_scopes_tests;
