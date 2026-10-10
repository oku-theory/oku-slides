//! La surface d'un tour de l'agent : consigne, timeline repliable, note en
//! cours et issue. Constructeurs purs ; le réducteur des tours
//! (`oku-slides-agent::turns`) tient l'état et émet par le hub.

use oku_slides_a2ui::{Component, ComponentKind, Dynamic, ServerMessage, TextVariant, TimelineStep};
use oku_slides_i18n::Localizer;
use serde_json::{Value, json};

use super::bounds::NOTE_CHARS;
use super::forms::{Place, Surface, surface};

/// Où en est un tour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnState {
    /// L'agent travaille.
    Running,
    /// Fini normalement (`end_turn`), en `secs` secondes.
    Done { secs: u64 },
    /// Fin autre que `end_turn` (`cancelled`, `max_tokens`, `interrupted`…).
    Stopped { stop: String, secs: u64 },
    /// Erreur (démarrage, connexion, agent arrêté) : `message` est déjà traduit.
    Failed { message: String, secs: u64 },
}

/// Ce que la surface d'un tour montre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnView {
    /// Numéro du tour (donné par `SurfaceHub::begin_turn`).
    pub n: u32,
    /// La consigne de l'utilisateur.
    pub prompt: String,
    /// Entrées du plan ACP, affichées avant les étapes.
    pub plan: Vec<TimelineStep>,
    /// Étapes : outils groupés et notes de l'agent, dans l'ordre.
    pub steps: Vec<TimelineStep>,
    /// Nombre d'appels d'outils du tour.
    pub tools: u32,
    /// Nombre de notes de l'agent devenues étapes.
    pub notes: u32,
    /// Texte de l'agent depuis la dernière étape (affiché en note pendant le tour).
    pub narration: String,
    /// Où en est le tour.
    pub state: TurnState,
}

/// Identifiant de la surface du tour `n`.
pub fn turn_id(n: u32) -> String {
    format!("turn-{n}")
}

/// Première ligne non vide de `text`, coupée à `max` caractères (« … » compris).
pub fn first_line(text: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let line = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or_default();
    if line.chars().count() <= max {
        return line.to_owned();
    }
    let mut cut: String = line.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

fn outcome(state: &TurnState, l: &Localizer) -> String {
    match state {
        TurnState::Running | TurnState::Done { .. } => String::new(),
        TurnState::Stopped { stop, .. } => match stop.as_str() {
            "cancelled" => l.tr("agent-turn-stopped"),
            "max_tokens" => l.tr("agent-turn-max-tokens"),
            "max_turn_requests" => l.tr("agent-turn-max-turns"),
            "refusal" => l.tr("agent-turn-refusal"),
            "interrupted" => l.tr("agent-turn-interrupted"),
            other => l.tr_with("agent-turn-ended", &[("stop", other.into())]),
        },
        TurnState::Failed { message, .. } => message.clone(),
    }
}

/// Le modèle de données de la surface du tour.
pub fn turn_data(v: &TurnView, l: &Localizer) -> Value {
    let secs = match &v.state {
        TurnState::Done { secs } | TurnState::Stopped { secs, .. } | TurnState::Failed { secs, .. } => *secs,
        TurnState::Running => 0,
    };
    let running = v.state == TurnState::Running;
    let mut parts = vec![l.tr_with("agent-turn-tools", &[("tools", v.tools.into())])];
    if v.notes > 0 {
        parts.push(l.tr_with("agent-turn-notes", &[("notes", v.notes.into())]));
    }
    // La durée ne se lit qu'une fois le tour fini : pendant, elle resterait à 0 s.
    if !running {
        parts.push(l.tr_with("agent-turn-secs", &[("secs", secs.into())]));
    }
    let summary = parts.join(" · ");
    let steps: Vec<&TimelineStep> = v.plan.iter().chain(&v.steps).collect();
    // Repliée dès l'ouverture : le fil ne s'allonge pas d'une ligne par outil ; un clic la déplie.
    json!({
        "steps": steps,
        "collapsed": true,
        // L'état du tour, lisible par l'hôte et les tests : sa fin ne se voit plus au repli.
        "running": running,
        "summary": summary,
        "narration": if running { first_line(&v.narration, NOTE_CHARS) } else { String::new() },
        "outcome": outcome(&v.state, l),
    })
}

/// La surface complète du tour (à son ouverture, ou pour une resynchronisation).
///
/// Sans consigne (son `UserText` perdu par un relais en retard), pas de bulle `prompt`.
pub fn turn_surface(v: &TurnView, l: &Localizer) -> Surface {
    let id = turn_id(v.n);
    let prompt = !v.prompt.is_empty();
    let children: &[&str] =
        if prompt { &["prompt", "timeline", "narration", "outcome"] } else { &["timeline", "narration", "outcome"] };
    let mut components = vec![
        Component::column("root", children),
        Component::new(
            "timeline",
            ComponentKind::Timeline {
                steps: Dynamic::path("/steps"),
                collapsed: Dynamic::path("/collapsed"),
                summary: Dynamic::path("/summary"),
            },
        ),
        Component::new(
            "narration",
            ComponentKind::Text { text: Dynamic::path("/narration"), variant: Some(TextVariant::Note) },
        ),
        Component::new(
            "outcome",
            ComponentKind::Text { text: Dynamic::path("/outcome"), variant: Some(TextVariant::Note) },
        ),
    ];
    if prompt {
        let bubble = Component::new("prompt", ComponentKind::Prompt { text: Dynamic::Literal(v.prompt.clone()) });
        components.insert(1, bubble);
    }
    let mut s = surface(&id, Place::Thread, None, components, turn_data(v, l));
    s.turn = Some(v.n);
    s.live = false;
    s
}

/// Les écritures qui font passer la surface du tour `n` de `sent` à `data` : une par clé
/// racine qui a changé (`/narration` seule pendant que l'agent écrit, pas tout le modèle).
pub fn turn_patches(n: u32, sent: &Value, data: &Value) -> Vec<ServerMessage> {
    let Value::Object(next) = data else { return vec![ServerMessage::set_data_model(turn_id(n), data.clone())] };
    next.iter()
        .filter(|(k, v)| sent.get(k.as_str()) != Some(*v))
        .map(|(k, v)| ServerMessage::update_data_model(turn_id(n), &format!("/{k}"), v.clone()))
        .collect()
}

#[cfg(test)]
#[path = "turn_view_tests.rs"]
mod turn_view_tests;
