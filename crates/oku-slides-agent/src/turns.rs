//! Les tours d'un onglet : chaque évènement de l'agent met à jour la surface
//! de son tour (`turn-N`), émise par le hub comme toute surface. Remplace le
//! fil gardé : l'interface ne reçoit plus que l'état de l'agent.
//!
//! Une mise à jour ne part que pour les clés du modèle qui ont changé (en
//! général `/narration` seule pendant que l'agent écrit). Un tour dont la fin
//! s'est perdue (relais en retard) est clos par le retour de l'agent au repos,
//! ou interrompu par l'ouverture du tour suivant.
//!
//! Chaque évènement d'un tour porte son numéro : il ne met à jour que ce tour-là.
//! Un tour dont l'ouverture (`UserText`) s'est perdue s'ouvre, sans consigne, au
//! premier évènement qui le porte ; un évènement d'un tour clos ou évincé est ignoré.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::time::Instant;

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::turn_view::{
    TurnState, TurnView, first_line, turn_data, turn_id, turn_patches, turn_surface,
};
use oku_slides_session::agent::{AgentScope, NOTE_CHARS, ServerMessage, StepStatus, Surface, TimelineStep, WHY_MAX};
use serde_json::Value;

use crate::{AgentEvent, AgentState};

/// Tours gardés au plus par onglet ; au-delà, le plus ancien est fermé.
pub(crate) use oku_slides_session::agent::KEPT_TURNS;
/// Étapes gardées au plus par tour ; au-delà, les outils et notes ne comptent plus que dans le résumé.
pub(crate) const STEPS_MAX: usize = 60;

/// Ce que le relais doit faire après un évènement.
#[derive(Debug)]
pub(crate) enum TurnOut {
    /// Ouvrir (ou remplacer) la surface d'un tour.
    Show(Surface),
    /// Mettre à jour la surface d'un tour (une écriture par clé changée).
    Patch { id: String, messages: Vec<ServerMessage> },
    /// Fermer les surfaces du tour évincé.
    Evict(u32),
    /// Le tour est fini : reçu de repli si l'agent n'en a pas fait.
    Ended { turn: u32, narration: String },
}

#[derive(Debug)]
struct Turn {
    view: TurnView,
    /// La portée du tour : l'éviction se compte par portée.
    scope: AgentScope,
    started: Instant,
    /// Chaque outil : l'étape où il est rangé et son dernier statut.
    tools: HashMap<String, (usize, StepStatus)>,
    /// La dernière étape est-elle une étape d'outils ?
    in_tools: bool,
    /// L'étape du groupe d'outils en cours ; `None` si le plafond d'étapes l'a refusée (outils seulement comptés).
    group: Option<usize>,
    /// Le dernier texte complet de l'agent (la narration en cours, ou la dernière devenue note).
    last_text: String,
    /// Les données envoyées en dernier : seules les clés qui changent repartent.
    sent: Value,
}

/// Les tours de l'onglet, du plus ancien au plus récent.
#[derive(Debug, Default)]
pub(crate) struct Turns {
    turns: VecDeque<Turn>,
    /// Le plus grand numéro de tour ouvert, même oublié depuis (`Conversation`) : seul un
    /// tour plus récent peut encore s'ouvrir sans sa consigne.
    last: u32,
}

fn status(s: &str) -> StepStatus {
    match s {
        "in_progress" => StepStatus::InProgress,
        "completed" => StepStatus::Completed,
        "failed" => StepStatus::Failed,
        _ => StepStatus::Pending,
    }
}

/// Rang d'un statut dans une étape groupée : le plus bas l'emporte (un échec reste visible).
fn rank(st: StepStatus) -> u8 {
    match st {
        StepStatus::Failed => 0,
        StepStatus::InProgress => 1,
        StepStatus::Pending => 2,
        StepStatus::Completed => 3,
    }
}

impl Turn {
    /// Le texte en cours devient une note de la timeline (si la place le permet).
    fn close_narration(&mut self) {
        let text = self.view.narration.trim();
        if !text.is_empty() {
            self.last_text = text.to_owned();
            if self.view.steps.len() < STEPS_MAX {
                let note = first_line(text, NOTE_CHARS);
                self.view.steps.push(TimelineStep { label: note, status: StepStatus::Completed, count: None });
            }
            self.view.notes = self.view.notes.saturating_add(1);
        }
        self.view.narration.clear();
    }

    /// Statut d'une étape d'outils : un échec reste visible, sinon « en cours » tant qu'un outil l'est.
    fn settle_step(&mut self, step: usize) {
        let worst = self.tools.values().filter(|(s, _)| *s == step).map(|(_, st)| *st).min_by_key(|st| rank(*st));
        if let (Some(worst), Some(row)) = (worst, self.view.steps.get_mut(step)) {
            row.status = worst;
        }
    }

    /// Ferme le tour : les outils restés en cours sont soldés selon l'issue ; rend le dernier texte.
    fn finish(&mut self, state: TurnState) -> String {
        let leftover = if matches!(state, TurnState::Done { .. }) { StepStatus::Completed } else { StepStatus::Failed };
        for (_, st) in self.tools.values_mut() {
            if matches!(st, StepStatus::Pending | StepStatus::InProgress) {
                *st = leftover;
            }
        }
        let steps: BTreeSet<usize> = self.tools.values().map(|(s, _)| *s).collect();
        for step in steps {
            self.settle_step(step);
        }
        let narration = self.view.narration.trim();
        let text = if narration.is_empty() { self.last_text.clone() } else { narration.to_owned() };
        self.view.state = state;
        text
    }

    fn secs(&self, now: Instant) -> u64 {
        now.saturating_duration_since(self.started).as_secs()
    }

    /// Les écritures des clés qui ont changé depuis le dernier envoi ; rien si rien n'a changé.
    fn patch(&mut self, l: &Localizer) -> Option<TurnOut> {
        let data = turn_data(&self.view, l);
        let messages = turn_patches(self.view.n, &self.sent, &data);
        self.sent = data;
        (!messages.is_empty()).then(|| TurnOut::Patch { id: turn_id(self.view.n), messages })
    }
}

impl Turns {
    /// Applique `e` ; rend ce que le relais doit émettre.
    pub fn apply(&mut self, e: &AgentEvent, now: Instant, l: &Localizer) -> Vec<TurnOut> {
        let n = match e {
            AgentEvent::UserText { turn, scope, text } => return self.open(*turn, scope.clone(), text, now, l),
            // Le hub a déjà été vidé par l'acteur (`clear_thread`, synchrone) : on oublie les tours de la portée.
            AgentEvent::Conversation { scope } => {
                self.turns.retain(|t| t.scope != *scope);
                return Vec::new();
            }
            // Le repos ne porte pas de tour : il clôt le dernier s'il tourne encore (fin perdue).
            AgentEvent::Status { state: AgentState::Idle, .. } => match self.turns.back() {
                Some(t) => t.view.n,
                None => return Vec::new(),
            },
            AgentEvent::Status { .. } | AgentEvent::Draft { .. } | AgentEvent::Picker { .. } => return Vec::new(),
            AgentEvent::Chunk { turn, .. }
            | AgentEvent::ToolCall { turn, .. }
            | AgentEvent::ToolUpdate { turn, .. }
            | AgentEvent::Plan { turn, .. }
            | AgentEvent::TurnEnded { turn, .. }
            | AgentEvent::Error { turn, .. } => *turn,
        };
        // L'ouverture de ce tour s'est perdue (relais en retard) : il s'ouvre ici, sans consigne.
        let mut out = if n > self.last {
            let scope = self.turns.back().map_or(AgentScope::Slides, |t| t.scope.clone());
            self.open(n, scope, "", now, l)
        } else {
            Vec::new()
        };
        let Some(t) = self.turns.iter_mut().find(|t| t.view.n == n) else { return out };
        if t.view.state != TurnState::Running {
            return out;
        }
        let mut ended = None;
        match e {
            AgentEvent::Chunk { text, .. } => {
                // Au-delà de `WHY_MAX` caractères, la suite ne s'affiche plus nulle part : on ne la garde pas
                // (un seul morceau trop long est coupé, lui aussi).
                let room = WHY_MAX.saturating_sub(t.view.narration.chars().count());
                t.view.narration.extend(text.chars().take(room));
                // Un morceau vide (`"\n\n"` entre deux outils) ne sépare pas un groupe d'outils.
                if !text.trim().is_empty() {
                    t.in_tools = false;
                }
            }
            AgentEvent::ToolCall { id, title, status: s, .. } => {
                if t.tools.contains_key(id) {
                    // Un même appel annoncé deux fois : traité comme une mise à jour.
                    let step = t.tools[id].0;
                    t.tools.insert(id.clone(), (step, status(s)));
                    t.settle_step(step);
                } else {
                    if !t.in_tools {
                        t.close_narration();
                        t.group = None;
                        if t.view.steps.len() < STEPS_MAX {
                            t.view.steps.push(TimelineStep { label: title.clone(), status: status(s), count: Some(1) });
                            t.group = Some(t.view.steps.len() - 1);
                        }
                        t.in_tools = true;
                    }
                    t.view.tools = t.view.tools.saturating_add(1);
                    if let Some(step) = t.group {
                        t.tools.insert(id.clone(), (step, status(s)));
                        let count = t.tools.values().filter(|(s, _)| *s == step).count();
                        if let Some(row) = t.view.steps.get_mut(step) {
                            row.label.clone_from(title);
                            row.count = Some(u32::try_from(count).unwrap_or(u32::MAX));
                        }
                        t.settle_step(step);
                    }
                }
            }
            AgentEvent::ToolUpdate { id, title, status: s, .. } => {
                let Some((step, st)) = t.tools.get_mut(id) else { return out };
                let step = *step;
                if let Some(s) = s {
                    *st = status(s);
                }
                if let (Some(title), Some(row)) = (title, t.view.steps.get_mut(step)) {
                    row.label.clone_from(title);
                }
                t.settle_step(step);
            }
            AgentEvent::Plan { entries, .. } => {
                t.view.plan = entries
                    .iter()
                    .take(STEPS_MAX)
                    .map(|p| TimelineStep {
                        label: first_line(&p.content, NOTE_CHARS),
                        status: status(&p.status),
                        count: None,
                    })
                    .collect();
            }
            AgentEvent::TurnEnded { stop, .. } => {
                let secs = t.secs(now);
                let state = if stop == "end_turn" {
                    TurnState::Done { secs }
                } else {
                    TurnState::Stopped { stop: stop.clone(), secs }
                };
                ended = Some(t.finish(state));
            }
            // L'agent est revenu au repos sans fin de tour (évènement perdu) : le tour est fini.
            AgentEvent::Status { state: AgentState::Idle, .. } => {
                let secs = t.secs(now);
                ended = Some(t.finish(TurnState::Done { secs }));
            }
            AgentEvent::Error { message, .. } => {
                let secs = t.secs(now);
                t.finish(TurnState::Failed { message: message.clone(), secs });
                // Pas de reçu de repli pour un échec, mais le hub doit savoir que le tour est clos.
                ended = Some(String::new());
            }
            AgentEvent::Status { .. }
            | AgentEvent::UserText { .. }
            | AgentEvent::Conversation { .. }
            | AgentEvent::Draft { .. }
            | AgentEvent::Picker { .. } => return out,
        }
        out.extend(t.patch(l));
        if let Some(narration) = ended {
            out.push(TurnOut::Ended { turn: n, narration });
        }
        out
    }

    fn open(&mut self, n: u32, scope: AgentScope, prompt: &str, now: Instant, l: &Localizer) -> Vec<TurnOut> {
        let mut out = Vec::new();
        self.last = self.last.max(n);
        // Un tour précédent resté ouvert (fin perdue) est clos comme interrompu.
        if let Some(prev) = self.turns.back_mut()
            && prev.view.state == TurnState::Running
        {
            let secs = prev.secs(now);
            prev.finish(TurnState::Stopped { stop: "interrupted".into(), secs });
            let k = prev.view.n;
            out.extend(prev.patch(l));
            out.push(TurnOut::Ended { turn: k, narration: String::new() });
        }
        let view = TurnView {
            n,
            prompt: prompt.to_owned(),
            plan: Vec::new(),
            steps: Vec::new(),
            tools: 0,
            notes: 0,
            narration: String::new(),
            state: TurnState::Running,
        };
        let sent = turn_data(&view, l);
        out.push(TurnOut::Show(turn_surface(&view, l)));
        self.turns.push_back(Turn {
            view,
            scope: scope.clone(),
            started: now,
            tools: HashMap::new(),
            in_tools: false,
            group: None,
            last_text: String::new(),
            sent,
        });
        let same = self.turns.iter().filter(|t| t.scope == scope).count();
        if same > KEPT_TURNS
            && let Some(i) = self.turns.iter().position(|t| t.scope == scope)
            && let Some(old) = self.turns.remove(i)
        {
            out.push(TurnOut::Evict(old.view.n));
        }
        out
    }
}

#[cfg(test)]
#[path = "turns_tests.rs"]
mod turns_tests;
