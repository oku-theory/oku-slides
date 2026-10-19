//! Le panneau de l'agent : choisir l'agent ACP de l'app parmi ceux de la configuration.
//!
//! Surface A2UI pure, construite par la pile à la demande d'une vue (`AgentCommand::Picker`).

use std::collections::BTreeMap;

use oku_slides_a2ui::{Action, ChoiceOption, Component, ComponentKind, Dynamic, ServerMessage, TextVariant};
use oku_slides_i18n::Localizer;
use oku_slides_platform::config::AgentPreset;
use oku_slides_session::agent::{Place, Surface};
use serde_json::json;

/// Identifiant de la surface du panneau.
pub(crate) const PICKER_ID: &str = "agent-picker";

/// Ce que le panneau montre de l'agent local quand il est choisi et pas prêt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LocalPanel {
    /// Modèle absent : la raison (avec la taille) et le bouton Télécharger.
    NeedsDownload { reason: String },
    /// Téléchargement en cours : la progression et le bouton Annuler.
    Running { percent: u8 },
    /// Le dernier téléchargement a échoué : la raison, l'erreur et le bouton Télécharger.
    Failed { reason: String, error: String },
}

/// Le panneau : une pastille par agent, l'actif choisi.
///
/// `busy` : un tour est ouvert dans un onglet (il sera interrompu) ; `locked` : le choix
/// est imposé par l'environnement, les pastilles n'envoient rien ; `local` : l'état du
/// modèle de l'agent local, montré sous les pastilles.
pub(crate) fn picker_surface(
    presets: &BTreeMap<String, AgentPreset>,
    active: &str,
    busy: bool,
    locked: bool,
    local: Option<&LocalPanel>,
    l: &Localizer,
) -> Surface {
    let options = presets
        .iter()
        .map(|(id, p)| ChoiceOption {
            id: id.clone(),
            label: p.name.clone(),
            hint: p.builtin.is_none().then(|| {
                std::iter::once(p.command.as_str())
                    .chain(p.args.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
        })
        .collect();
    let action = (!locked).then(|| Action::event("select", [("id", Dynamic::path("/choice"))]));
    let mut children = vec!["title", "agents"];
    let mut components = vec![
        Component::text("title", l.tr("agent-picker-title"), Some(TextVariant::H4)),
        Component::new("agents", ComponentKind::ChoiceChips { options, value: Dynamic::path("/choice"), action }),
    ];
    // L'agent local, choisi et pas prêt : de quoi télécharger son modèle.
    let download =
        || Component::button("download", l.tr("agent-model-download"), true, Action::named("download_model"));
    match local {
        Some(LocalPanel::NeedsDownload { reason }) => {
            children.extend(["local-reason", "download"]);
            components.push(Component::text("local-reason", reason.as_str(), None));
            components.push(download());
        }
        Some(LocalPanel::Running { percent }) => {
            children.extend(["progress", "cancel-download"]);
            let text = l.tr_with("agent-model-downloading", &[("percent", (*percent).into())]);
            components.push(Component::text("progress", text, None));
            let cancel = Action::named("cancel_download");
            components.push(Component::button("cancel-download", l.tr("agent-model-cancel"), false, cancel));
        }
        Some(LocalPanel::Failed { reason, error }) => {
            children.extend(["local-reason", "download-error", "download"]);
            components.push(Component::text("local-reason", reason.as_str(), None));
            components.push(Component::text("download-error", error.as_str(), None));
            components.push(download());
        }
        None => {}
    }
    if busy {
        children.push("busy");
        components.push(Component::text("busy", l.tr("agent-picker-busy"), None));
    }
    if locked {
        children.push("locked");
        components.push(Component::text("locked", l.tr("agent-picker-locked"), None));
    }
    components.insert(0, Component::column("root", &children));
    Surface {
        id: PICKER_ID.to_owned(),
        place: Place::Sheet,
        anchor: None,
        turn: None,
        live: true,
        question: false,
        messages: vec![
            ServerMessage::create_surface(PICKER_ID),
            ServerMessage::update_components(PICKER_ID, components),
            ServerMessage::set_data_model(PICKER_ID, json!({ "choice": active })),
        ],
    }
}

#[cfg(test)]
#[path = "picker_tests.rs"]
mod picker_tests;
