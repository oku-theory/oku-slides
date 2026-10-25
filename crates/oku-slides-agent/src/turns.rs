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

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::Instant;

use oku_slides_i18n::Localizer;
use oku_slides_session::agent::turn_view::{
    TurnState, TurnView, first_line, turn_data, turn_id, turn_patches, turn_surface,
};
use oku_slides_session::agent::{
    LABEL_MAX, NOTE_CHARS, PromptImage, ServerMessage, StepStatus, Surface, TimelineStep, ToolCallDetail, WHY_MAX, clip,
};
use serde_json::Value;

use crate::{AgentEvent, AgentState};

/// Tours gardés au plus par onglet ; au-delà, le plus ancien est fermé.
pub(crate) use oku_slides_session::agent::KEPT_TURNS;
/// Étapes gardées au plus par tour ; au-delà, les outils et notes ne comptent plus que dans le résumé.
pub(crate) const STEPS_MAX: usize = 60;
/// Appels listés au plus par tour, les plus récents.
///
/// Les plus anciens sortent de la liste et ne font plus que compter.
pub(crate) const CALLS_MAX: usize = 60;

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

/// Un appel d'outil du tour, suivi tant qu'il est listé ou encore ouvert.
#[derive(Debug)]
struct Tool {
    /// L'étape où il est rangé.
    step: usize,
    /// Son dernier statut.
    status: StepStatus,
    /// Son rang parmi les appels de l'étape, depuis le premier (sorti de la liste ou non).
    seq: usize,
}

/// Le décompte des appels d'une étape d'outils, tenu à chaque évènement.
#[derive(Debug, Default)]
struct Tally {
    /// Appels de l'étape, listés ou non.
    calls: usize,
    /// Appels de l'étape sortis de la liste : toujours les plus anciens.
    unlisted: usize,
    /// Appels par statut, dans l'ordre de [`rank`].
    by_status: [usize; 4],
}

impl Tally {
    /// Le statut de l'étape : le pire de ses appels.
    fn worst(&self) -> Option<StepStatus> {
        RANKED.into_iter().zip(self.by_status).find(|(_, n)| *n > 0).map(|(st, _)| st)
    }

    /// Un appel de l'étape passe de `from` (`None` : nouvel appel) à `to`.
    fn moved(&mut self, from: Option<StepStatus>, to: StepStatus) {
        if let Some(slot) = from.and_then(|st| self.by_status.get_mut(rank(st))) {
            *slot = slot.saturating_sub(1);
        }
        if let Some(slot) = self.by_status.get_mut(rank(to)) {
            *slot += 1;
        }
    }
}

/// Un appel encore ouvert : il peut changer le statut de son étape.
fn is_open(st: StepStatus) -> bool {
    matches!(st, StepStatus::Pending | StepStatus::InProgress)
}

/// Ce qu'une annonce ou une mise à jour change d'un appel (`None` : inchangé).
#[derive(Debug, Default, Clone, Copy)]
struct CallFields<'a> {
    title: Option<&'a str>,
    input: Option<&'a str>,
    output: Option<&'a str>,
}

#[derive(Debug)]
struct Turn {
    view: TurnView,
    started: Instant,
    /// Les appels listés ou encore ouverts, par identifiant ; un appel soldé et sorti de la liste est oublié.
    tools: HashMap<String, Tool>,
    /// Le décompte de chaque étape d'outils, par indice d'étape.
    tallies: BTreeMap<usize, Tally>,
    /// Les appels listés du tour, du plus ancien au plus récent (au plus [`CALLS_MAX`]).
    listed: VecDeque<String>,
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

/// Le nom lisible d'un outil, borné et filtré : `mcp__oku-slides__widgets` devient `widgets`.
fn tool_label(title: &str) -> String {
    let name = title
        .strip_prefix("mcp__")
        .and_then(|rest| rest.split_once("__"))
        .map(|(_, name)| name)
        .filter(|name| !name.is_empty())
        .unwrap_or(title);
    clip(name, LABEL_MAX)
}

/// Les statuts, du plus grave au moins grave.
const RANKED: [StepStatus; 4] =
    [StepStatus::Failed, StepStatus::InProgress, StepStatus::Pending, StepStatus::Completed];

/// Rang d'un statut dans une étape groupée : le plus bas l'emporte (un échec reste visible).
fn rank(st: StepStatus) -> usize {
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
                // Ce que le libellé ne montre pas : la suite, s'il montre la première ligne
                // entière ; sinon le texte entier. Sans caractères invisibles, comme un outil.
                let detail = (note != text).then(|| match text.split_once('\n') {
                    Some((first, rest)) if first.trim() == note => clip(rest.trim(), WHY_MAX),
                    _ => clip(text, WHY_MAX),
                });
                self.view.steps.push(TimelineStep { detail, ..TimelineStep::new(note, StepStatus::Completed) });
            }
            self.view.notes = self.view.notes.saturating_add(1);
        }
        self.view.narration.clear();
    }

    /// Statut d'une étape d'outils : un échec reste visible, sinon « en cours » tant qu'un outil l'est.
    fn settle_step(&mut self, step: usize) {
        let worst = self.tallies.get(&step).and_then(Tally::worst);
        if let (Some(worst), Some(row)) = (worst, self.view.steps.get_mut(step)) {
            row.status = worst;
        }
    }

    /// Ferme le tour : les outils restés en cours sont soldés selon l'issue ; rend le dernier texte.
    fn finish(&mut self, state: TurnState) -> String {
        let leftover = if matches!(state, TurnState::Done { .. }) { StepStatus::Completed } else { StepStatus::Failed };
        let open: Vec<String> =
            self.tools.iter().filter(|(_, x)| is_open(x.status)).map(|(id, _)| id.clone()).collect();
        for id in open {
            self.update_call(&id, Some(leftover), CallFields::default());
        }
        let steps: Vec<usize> = self.tallies.keys().copied().collect();
        for step in steps {
            self.settle_step(step);
        }
        let narration = self.view.narration.trim();
        let text = if narration.is_empty() { self.last_text.clone() } else { narration.to_owned() };
        self.view.state = state;
        text
    }

    /// Ouvre l'appel `id` dans le groupe d'outils en cours (une nouvelle étape s'il n'y en a
    /// pas) ; au-delà de [`CALLS_MAX`] appels listés, le plus ancien sort de la liste.
    fn add_call(&mut self, id: &str, status: StepStatus, f: CallFields<'_>) {
        let label = tool_label(f.title.unwrap_or_default());
        if !self.in_tools {
            self.close_narration();
            self.group = None;
            if self.view.steps.len() < STEPS_MAX {
                self.view.steps.push(TimelineStep { count: Some(1), ..TimelineStep::new(label.clone(), status) });
                self.group = Some(self.view.steps.len() - 1);
            }
            self.in_tools = true;
        }
        self.view.tools = self.view.tools.saturating_add(1);
        let Some(step) = self.group else { return };
        let Some(row) = self.view.steps.get_mut(step) else { return };
        let tally = self.tallies.entry(step).or_default();
        let seq = tally.calls;
        tally.calls += 1;
        tally.moved(None, status);
        row.label.clone_from(&label);
        row.count = Some(u32::try_from(tally.calls).unwrap_or(u32::MAX));
        row.calls.push(ToolCallDetail {
            label,
            status,
            input: f.input.map(str::to_owned),
            output: f.output.map(str::to_owned),
        });
        self.tools.insert(id.to_owned(), Tool { step, status, seq });
        self.listed.push_back(id.to_owned());
        if self.listed.len() > CALLS_MAX
            && let Some(oldest) = self.listed.pop_front()
        {
            self.unlist(&oldest);
        }
        self.settle_step(step);
    }

    /// Retire l'appel `id` de la liste de son étape (il y est le premier : le plus ancien) ;
    /// soldé, il est oublié, sinon il reste suivi jusqu'à sa fin.
    fn unlist(&mut self, id: &str) {
        let Some(tool) = self.tools.get(id) else { return };
        let (step, open) = (tool.step, is_open(tool.status));
        if let Some(tally) = self.tallies.get_mut(&step) {
            tally.unlisted += 1;
        }
        if let Some(row) = self.view.steps.get_mut(step)
            && !row.calls.is_empty()
        {
            row.calls.remove(0);
        }
        if !open {
            self.tools.remove(id);
        }
    }

    /// La place de l'appel `seq` de l'étape dans ses appels listés ; `None` s'il en est sorti.
    fn listed_at(&self, step: usize, seq: usize) -> Option<usize> {
        seq.checked_sub(self.tallies.get(&step)?.unlisted)
    }

    /// Met à jour l'appel `id` (statut et champs donnés) ; rend son étape.
    fn update_call(&mut self, id: &str, status: Option<StepStatus>, f: CallFields<'_>) -> Option<usize> {
        let tool = self.tools.get_mut(id)?;
        let (step, seq, before) = (tool.step, tool.seq, tool.status);
        if let Some(st) = status {
            tool.status = st;
            if let Some(tally) = self.tallies.get_mut(&step) {
                tally.moved(Some(before), st);
            }
        }
        let call = self.listed_at(step, seq);
        // Sorti de la liste et soldé : plus rien de lui ne s'affiche, il est oublié.
        if call.is_none() && status.is_some_and(|st| !is_open(st)) {
            self.tools.remove(id);
        }
        let row = self.view.steps.get_mut(step)?;
        let label = f.title.map(tool_label);
        if let Some(label) = &label {
            row.label.clone_from(label);
        }
        if let Some(c) = call.and_then(|i| row.calls.get_mut(i)) {
            if let Some(st) = status {
                c.status = st;
            }
            if let Some(label) = label {
                c.label = label;
            }
            if let Some(input) = f.input {
                c.input = Some(input.to_owned());
            }
            if let Some(output) = f.output {
                c.output = Some(output.to_owned());
            }
        }
        Some(step)
    }

    fn secs(&self, now: Instant) -> u64 {
        now.saturating_duration_since(self.started).as_secs()
    }

    /// Les écritures minimales de ce qui a changé depuis le dernier envoi ; rien si rien n'a changé.
    fn patch(&mut self, l: &Localizer) -> Option<TurnOut> {
        let data = turn_data(&self.view, l);
        let messages = turn_patches(self.view.n, &self.sent, &data);
        self.sent = data;
        (!messages.is_empty()).then(|| TurnOut::Patch { id: turn_id(self.view.n), messages })
    }
}

/// La vue d'un tour qui s'ouvre : sa consigne et ses images, rien d'autre encore.
fn opening(n: u32, prompt: &str, images: &[PromptImage]) -> TurnView {
    TurnView {
        n,
        prompt: prompt.to_owned(),
        plan: Vec::new(),
        steps: Vec::new(),
        tools: 0,
        notes: 0,
        narration: String::new(),
        images: images.to_vec(),
        state: TurnState::Running,
    }
}

impl Turns {
    /// Applique `e` ; rend ce que le relais doit émettre.
    pub fn apply(&mut self, e: &AgentEvent, now: Instant, l: &Localizer) -> Vec<TurnOut> {
        let n = match e {
            AgentEvent::UserText { turn, text, images } => return self.open(opening(*turn, text, images), now, l),
            // Le hub a déjà été vidé par le gardien (`clear_thread`) : on oublie tous les tours.
            AgentEvent::Conversation => {
                self.turns.clear();
                return Vec::new();
            }
            // Le repos ne porte pas de tour : il clôt le dernier s'il tourne encore (fin perdue).
            AgentEvent::Status { state: AgentState::Idle, .. } => match self.turns.back() {
                Some(t) => t.view.n,
                None => return Vec::new(),
            },
            AgentEvent::Status { .. }
            | AgentEvent::Draft { .. }
            | AgentEvent::Availability { .. }
            | AgentEvent::Voice => return Vec::new(),
            AgentEvent::Chunk { turn, .. }
            | AgentEvent::ToolCall { turn, .. }
            | AgentEvent::ToolUpdate { turn, .. }
            | AgentEvent::Plan { turn, .. }
            | AgentEvent::TurnEnded { turn, .. }
            | AgentEvent::Error { turn, .. } => *turn,
        };
        // L'ouverture de ce tour s'est perdue (relais en retard) : il s'ouvre ici, sans consigne.
        let mut out = if n > self.last { self.open(opening(n, "", &[]), now, l) } else { Vec::new() };
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
            AgentEvent::ToolCall { id, title, status: given, input, output, .. } => {
                let fields = CallFields { title: Some(title), input: input.as_deref(), output: output.as_deref() };
                // Un même appel annoncé deux fois : traité comme une mise à jour.
                if t.tools.contains_key(id) {
                    if let Some(step) = t.update_call(id, Some(status(given)), fields) {
                        t.settle_step(step);
                    }
                } else {
                    t.add_call(id, status(given), fields);
                }
            }
            AgentEvent::ToolUpdate { id, title, status: given, input, output, .. } => {
                let fields = CallFields { title: title.as_deref(), input: input.as_deref(), output: output.as_deref() };
                let Some(step) = t.update_call(id, given.as_deref().map(status), fields) else { return out };
                t.settle_step(step);
            }
            AgentEvent::Plan { entries, .. } => {
                t.view.plan = entries
                    .iter()
                    .take(STEPS_MAX)
                    .map(|entry| TimelineStep::new(first_line(&entry.content, NOTE_CHARS), status(&entry.status)))
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
            | AgentEvent::Conversation
            | AgentEvent::Draft { .. }
            | AgentEvent::Availability { .. }
            | AgentEvent::Voice => return out,
        }
        out.extend(t.patch(l));
        if let Some(narration) = ended {
            out.push(TurnOut::Ended { turn: n, narration });
        }
        out
    }

    fn open(&mut self, view: TurnView, now: Instant, l: &Localizer) -> Vec<TurnOut> {
        let n = view.n;
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
        let sent = turn_data(&view, l);
        out.push(TurnOut::Show(turn_surface(&view, l)));
        self.turns.push_back(Turn {
            view,
            started: now,
            tools: HashMap::new(),
            tallies: BTreeMap::new(),
            listed: VecDeque::new(),
            in_tools: false,
            group: None,
            last_text: String::new(),
            sent,
        });
        if self.turns.len() > KEPT_TURNS
            && let Some(old) = self.turns.pop_front()
        {
            out.push(TurnOut::Evict(old.view.n));
        }
        out
    }
}

#[cfg(test)]
#[path = "turns_tests.rs"]
mod turns_tests;
