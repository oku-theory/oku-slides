//! Tours du fil : numéros, rangement (dans le tour de l'appel), gel, éviction, expiration des suites.

use oku_slides_session::agent::receipt::{ReceiptSurfaces, fallback};
use oku_slides_session::agent::{KEPT_TURNS, Surface};

use super::{HubState, McpEvent, Shown, SurfaceHub};

/// Ce qui remplace une question du fil une fois répondue (ou annulée).
pub type Freeze = Box<dyn FnOnce(&super::Answer) -> Surface + Send>;

/// Quels tours peuvent encore recevoir une surface figée.
#[derive(Debug, Clone, Copy)]
enum Admit {
    /// Le tour ouvert seulement : surfaces des outils (`render`, `report`).
    Open,
    /// Tout tour encore dans le fil : question figée, reçu de repli (le tour vient de finir).
    InThread,
}

/// Surfaces figées gardées au plus par tour (`render`, questions répondues) : contre un agent qui en empile.
pub const STATIC_PER_TURN: usize = 8;

impl SurfaceHub {
    /// Opens a turn in the thread.
    ///
    /// It closes the oldest turns of the thread beyond `KEPT_TURNS`. Only this
    /// eviction counts (read-back turns included). The one of the agent's turn
    /// reducer (`TurnOut::Evict`) still calls `close_turn` on a turn already
    /// evicted here: redundant, but harmless (`close_turn` is idempotent).
    pub fn begin_turn(&self) -> u32 {
        let (n, old) = {
            let mut state = self.state();
            state.turns = state.turns.saturating_add(1);
            let n = state.turns;
            state.turn = Some(n);
            state.live_turns.insert(n);
            let excess = state.live_turns.len().saturating_sub(KEPT_TURNS);
            (n, state.live_turns.iter().copied().take(excess).collect::<Vec<_>>())
        };
        for t in old {
            self.close_turn(t);
        }
        self.expire_actions();
        n
    }

    /// Le tour ouvert, `None` entre deux tours : le jeton qu'un appel d'outil prend en arrivant.
    ///
    /// Une surface du fil créée par un outil est rangée dans le tour de son appel, jamais dans
    /// celui qui est ouvert quand elle arrive : `Tools::call` lit ce tour avant tout travail et
    /// le passe à `show_static`, `show_report`, `ask_then` ou `show_params`.
    pub fn turn(&self) -> Option<u32> {
        self.state().turn
    }

    /// Le tour `turn` peut-il encore recevoir une surface (verrou déjà pris) ?
    fn admits(state: &HubState, turn: u32, admit: Admit) -> bool {
        let open = match admit {
            Admit::Open => state.turn == Some(turn),
            Admit::InThread => true,
        };
        open && !state.closed && state.live_turns.contains(&turn)
    }

    /// Une composition `render` dans le tour `turn` de son appel.
    ///
    /// Refusée (`false`) si l'onglet est fermé ou si `turn` n'est plus le tour ouvert (appel
    /// tardif d'un tour annulé ou clos) : la décision et l'insertion se font sous le même verrou.
    pub fn show_static(&self, turn: u32, surface: Surface) -> bool {
        self.put_static(turn, surface, Admit::Open)
    }

    /// Une question répondue (ou annulée), figée dans son tour, tant que ce tour est dans le fil.
    pub(super) fn show_frozen(&self, turn: u32, surface: Surface) -> bool {
        self.put_static(turn, surface, Admit::InThread)
    }

    fn put_static(&self, turn: u32, mut surface: Surface, admit: Admit) -> bool {
        let oldest = {
            let mut state = self.state();
            if !Self::admits(&state, turn, admit) {
                return false;
            }
            surface.turn = Some(turn);
            surface.live = false;
            let plain = surface.has_actions().then(|| surface.without_actions());
            state
                .shows
                .insert(surface.id.clone(), Shown::Static { turn: Some(turn), plain, expired: false, report: false });
            // Au plus `STATIC_PER_TURN` surfaces figées par tour : la plus ancienne cède la place.
            let same: Vec<String> = state
                .open
                .iter()
                .filter(|s| s.turn == Some(turn) && s.id != surface.id && !s.live)
                // Le reçu du tour ne compte pas : il est unique et ne doit jamais céder sa place.
                .filter(|s| matches!(state.shows.get(&s.id), Some(Shown::Static { report: false, .. })))
                .map(|s| s.id.clone())
                .collect();
            let excess = (same.len() + 1).saturating_sub(STATIC_PER_TURN);
            same.into_iter().take(excess).collect::<Vec<_>>()
        };
        self.close_all(oldest);
        self.emit(McpEvent::Surface { surface });
        true
    }

    /// Fige la surface `id` : elle quitte le dock pour son tour, en lecture seule, avec ses dernières valeurs.
    pub fn freeze(&self, id: &str) {
        let frozen = {
            let mut state = self.state();
            let Some(open) = state.open.iter_mut().find(|s| s.id == id) else {
                tracing::debug!(surface = id, "surface à figer introuvable");
                return;
            };
            // En lecture seule : ses actions n'auraient plus de destinataire.
            let mut frozen = open.without_actions();
            frozen.live = false;
            *open = frozen.clone();
            let turn = frozen.turn;
            state.shows.insert(id.to_owned(), Shown::Static { turn, plain: None, expired: false, report: false });
            frozen
        };
        self.emit(McpEvent::Surface { surface: frozen });
    }

    /// Le reçu du tour `turn` de son appel ; remplace un reçu déjà montré pour ce tour.
    ///
    /// Refusé (`false`) comme `show_static` : onglet fermé, ou `turn` n'est plus le tour ouvert.
    /// Un appel tardif ne touche donc jamais au reçu (de repli ou non) d'un autre tour.
    pub fn show_report(&self, turn: u32, report: ReceiptSurfaces) -> bool {
        self.put_report(turn, report, Admit::Open)
    }

    fn put_report(&self, turn: u32, report: ReceiptSurfaces, admit: Admit) -> bool {
        let ReceiptSurfaces { mut surface, plain } = report;
        let old: Vec<String> = {
            let mut state = self.state();
            if !Self::admits(&state, turn, admit) {
                return false;
            }
            surface.turn = Some(turn);
            surface.live = false;
            let old: Vec<String> = state
                .shows
                .iter()
                .filter(|(_, s)| matches!(s, Shown::Static { turn: t, report: true, .. } if *t == Some(turn)))
                .map(|(id, _)| id.clone())
                .collect();
            for id in &old {
                state.shows.remove(id);
            }
            state.reported.insert(turn);
            let plain = plain.map(|mut p| {
                p.turn = Some(turn);
                p.live = false;
                p
            });
            state
                .shows
                .insert(surface.id.clone(), Shown::Static { turn: Some(turn), plain, expired: false, report: true });
            old
        };
        for id in old {
            self.emit(McpEvent::CloseSurface { id });
        }
        self.emit(McpEvent::Surface { surface });
        true
    }

    /// Le tour `turn` a-t-il son reçu ?
    pub fn has_report(&self, turn: u32) -> bool {
        self.state().reported.contains(&turn)
    }

    /// Fin du tour `turn` : plus de tour en cours, et reçu de repli si l'agent n'en a pas fait.
    pub fn end_turn(&self, turn: u32, narration: &str) {
        {
            // Plus de tour ouvert : un appel tardif de ce tour est refusé (voir `turn`).
            let mut state = self.state();
            if state.turn == Some(turn) {
                state.turn = None;
            }
        }
        if self.has_report(turn) || narration.trim().is_empty() {
            return;
        }
        let id = self.next_id("report");
        if let Some(surface) = fallback(&id, narration, &self.localizer.current()) {
            // Le tour n'est plus ouvert, mais son reçu de repli y va tant qu'il est dans le fil.
            self.put_report(turn, ReceiptSurfaces { surface, plain: None }, Admit::InThread);
        }
    }

    /// Ferme les surfaces du tour `n` (tour évincé du fil).
    ///
    /// Toutes, vivantes comprises : des réglages (`show_params`) encore au dock quand leur tour
    /// est évincé sont **fermés, pas figés** (leur tour n'est plus dans le fil pour les recevoir),
    /// et une action en retard sur eux est refusée (`unknown surface`). Cas rare : il faut
    /// `KEPT_TURNS` tours sans nouveaux réglages ni fermeture. Une question en attente n'y survit
    /// pas non plus, mais elle est déjà relâchée par le tour suivant (`cancel_asks`).
    pub fn close_turn(&self, n: u32) {
        let ids = {
            let mut state = self.state();
            state.live_turns.remove(&n);
            state.reported.remove(&n);
            Self::thread_ids(&state, |t| t == n)
        };
        self.close_all(ids);
    }

    /// Nouvelle conversation : les questions du fil relâchées, ses tours et surfaces fermés.
    ///
    /// Appelée **par le gardien de l'onglet** (ordre `Forget`, dans l'ordre de sa
    /// file), **avant la consigne suivante** : l'acteur attend la réponse de l'oubli,
    /// donc jamais en concurrence avec `begin_turn`. Tout est décidé sous un seul verrou.
    pub fn clear_thread(&self) {
        let ids = {
            let mut state = self.state();
            let turns = std::mem::take(&mut state.live_turns);
            let ids = Self::thread_ids(&state, |t| turns.contains(&t));
            if state.turn.is_some_and(|t| turns.contains(&t)) {
                state.turn = None;
            }
            for t in &turns {
                state.reported.remove(t);
            }
            ids
        };
        // Après l'effacement : une question relâchée ici ne trouve plus son tour et se ferme au lieu de se figer.
        self.cancel_asks_where(|id| ids.iter().any(|x| x == id));
        self.close_all(ids);
    }

    /// Les surfaces rangées dans un tour que `which` désigne.
    fn thread_ids(state: &HubState, which: impl Fn(u32) -> bool) -> Vec<String> {
        state.open.iter().filter(|s| s.turn.is_some_and(&which)).map(|s| s.id.clone()).collect()
    }

    /// Ferme `ids` (sans verrou tenu par l'appelant).
    pub(super) fn close_all(&self, ids: Vec<String>) {
        for id in ids {
            self.state().shows.remove(&id);
            self.emit(McpEvent::CloseSurface { id });
        }
    }

    /// Les suites des tours passés expirent : leur surface est remplacée par sa version sans actions.
    fn expire_actions(&self) {
        let plains: Vec<Surface> = {
            let mut state = self.state();
            let current = state.turn;
            state
                .shows
                .values_mut()
                .filter_map(|shown| match shown {
                    Shown::Static { turn, plain, expired, .. } if !*expired && *turn != current => {
                        *expired = true;
                        plain.take()
                    }
                    _ => None,
                })
                .collect()
        };
        for surface in plains {
            self.emit(McpEvent::Surface { surface });
        }
    }
}
