//! Suit la révision de la dictée et de l'agent pour tous les onglets.

use std::sync::{Mutex, PoisonError, Weak};

use oku_slides_mcp::VoicePort;
use oku_slides_session::{DictationView, VoicePanel};
use tokio::runtime::Handle;
use tokio::sync::mpsc;

use crate::events::AgentEvent;
use crate::tab_entry::{Out, emit};
use crate::table::Table;
use crate::voice_port::AppVoice;

/// Ce que la tâche a déjà dit aux onglets.
#[derive(Clone, Default)]
struct Seen {
    dictation: Option<DictationView>,
    availability: Option<Option<String>>,
}

/// Ce que voit la boîte au départ : rien n'est envoyé pour lui, seul un changement part.
fn seed(voice: &Weak<AppVoice>) -> Seen {
    voice
        .upgrade()
        .map(|v| Seen { dictation: v.view().dictation, availability: Some(v.availability()) })
        .unwrap_or_default()
}

/// Suit `voice` : une vue par changement, la boîte de chaque onglet, le micro relu.
///
/// La tâche async ne garde rien de fort ; tout le travail bloquant passe par le pool.
pub(crate) fn spawn(rt: &Handle, voice: Weak<AppVoice>, table: Weak<Mutex<Table>>, out: mpsc::UnboundedSender<Out>) {
    let Some(mut revision) = voice.upgrade().map(|v| v.revision()) else { return };
    rt.spawn(async move {
        let start = voice.clone();
        let mut seen = match tokio::task::spawn_blocking(move || seed(&start)).await {
            Ok(seen) => seen,
            Err(e) => {
                tracing::warn!(error = %e, "état de départ de la boîte non lu : le premier changement part en entier");
                Seen::default()
            }
        };
        while revision.changed().await.is_ok() {
            let (voice, table, out, previous) = (voice.clone(), table.clone(), out.clone(), seen.clone());
            let done = tokio::task::spawn_blocking(move || {
                let (Some(voice), Some(table)) = (voice.upgrade(), table.upgrade()) else { return previous };
                let view = voice.view();
                let panel = VoicePanel::build(&view, &voice.localizer());
                let availability = voice.availability();
                let links = table.lock().unwrap_or_else(PoisonError::into_inner).links();
                for (tab, link) in links {
                    link.hub.refresh_settings_with(&panel);
                    if view.dictation != previous.dictation {
                        emit(&out, "agent", &tab, &AgentEvent::Voice);
                    }
                    if Some(&availability) != previous.availability.as_ref() {
                        emit(&out, "agent", &tab, &AgentEvent::Availability { unusable: availability.clone() });
                    }
                }
                Seen { dictation: view.dictation, availability: Some(availability) }
            })
            .await;
            match done {
                Ok(next) => seen = next,
                Err(e) => tracing::warn!(error = %e, "boîte des réglages non rafraîchie"),
            }
        }
    });
}
