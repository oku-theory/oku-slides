//! Commandes de l'interface et réactions aux événements des fenêtres.
//!
//! Le travail (rendu, lecture de fichiers, fenêtres) tourne hors du fil
//! principal (`blocking`, `spawn_blocking`) : le fil principal ne verrouille
//! jamais l'espace de travail, ni ne fabrique ou détruit de fenêtre lui-même.

use std::path::PathBuf;
use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use oku_slides_agent::AgentCommand;
use oku_slides_session::{Request, Workspace, WorkspaceRequest};
use oku_slides_stt::{Status, Stt, SttRequest};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager};

use crate::state::{self, AppState, CLOSING};
use crate::windows;

/// Travail bloquant hors du fil principal ; sa panique devient une erreur.
///
/// `f` reçoit l'`AppHandle` (pour `windows::sync` et consorts) et l'état de
/// l'app, tous deux prêts sur ce fil dédié.
pub async fn blocking<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle, &AppState) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.try_state::<AppState>().ok_or_else(|| "app not ready".to_owned())?;
        f(&app, &state)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// `changes` : les fenêtres sont à resynchroniser après la requête.
/// `focus` : la fenêtre visée doit passer au premier plan.
///
/// Exhaustif à dessein : une variante ajoutée à `WorkspaceRequest` sans
/// mise à jour ici ne compile plus.
fn classify(req: &WorkspaceRequest) -> (bool, bool) {
    match req {
        // `SameDeck` et `CarryAssets` ne changent pas la vue : leur réponse est rendue telle quelle ;
        // `SetLocale` et `SetScheme` ne touchent qu'aux réglages, pas aux onglets.
        WorkspaceRequest::View
        | WorkspaceRequest::Messages
        | WorkspaceRequest::SetLocale { .. }
        | WorkspaceRequest::Scheme
        | WorkspaceRequest::SetScheme { .. }
        | WorkspaceRequest::SameDeck { .. }
        | WorkspaceRequest::CarryAssets { .. }
        | WorkspaceRequest::Clip
        | WorkspaceRequest::SetClip { .. }
        | WorkspaceRequest::PackOpener { .. } => (false, false),
        WorkspaceRequest::NewTab { .. }
        | WorkspaceRequest::Activate { .. }
        | WorkspaceRequest::Close { .. }
        | WorkspaceRequest::Reorder { .. }
        | WorkspaceRequest::CloseWindow { .. }
        | WorkspaceRequest::DetachAgent
        | WorkspaceRequest::AttachAgent
        | WorkspaceRequest::ImportPicked { .. } => (true, false),
        WorkspaceRequest::OpenTab { .. }
        | WorkspaceRequest::ImportTab { .. }
        | WorkspaceRequest::OpenHelp { .. }
        | WorkspaceRequest::NewWindow
        | WorkspaceRequest::MoveTab { .. }
        | WorkspaceRequest::OpenPack { .. }
        | WorkspaceRequest::OpenTheme { .. } => (true, true),
    }
}

/// L'évènement diffusé à toutes les fenêtres avec la réponse, s'il y en a un.
///
/// Le schéma et la langue valent pour l'app entière : chaque fenêtre (celle de
/// l'agent comprise) les suit ; la langue part avec ses messages (`messages`).
/// Seule une requête sans resynchronisation des fenêtres (`classify`) est diffusée.
///
/// Exhaustif à dessein, comme `classify` : un réglage ajouté doit dire s'il vaut
/// pour toutes les fenêtres.
fn broadcast(req: &WorkspaceRequest) -> Option<&'static str> {
    match req {
        WorkspaceRequest::SetScheme { .. } => Some("scheme"),
        WorkspaceRequest::SetLocale { .. } => Some("messages"),
        // L'élément copié vaut pour toutes les fenêtres : celle qui l'envoie ignore son écho.
        WorkspaceRequest::SetClip { .. } => Some("clip"),
        WorkspaceRequest::View
        | WorkspaceRequest::Messages
        | WorkspaceRequest::Scheme
        | WorkspaceRequest::SameDeck { .. }
        | WorkspaceRequest::CarryAssets { .. }
        | WorkspaceRequest::Clip
        | WorkspaceRequest::NewTab { .. }
        | WorkspaceRequest::Activate { .. }
        | WorkspaceRequest::Close { .. }
        | WorkspaceRequest::Reorder { .. }
        | WorkspaceRequest::CloseWindow { .. }
        | WorkspaceRequest::DetachAgent
        | WorkspaceRequest::AttachAgent
        | WorkspaceRequest::OpenTab { .. }
        | WorkspaceRequest::ImportTab { .. }
        | WorkspaceRequest::OpenHelp { .. }
        | WorkspaceRequest::NewWindow
        | WorkspaceRequest::MoveTab { .. }
        | WorkspaceRequest::ImportPicked { .. }
        | WorkspaceRequest::OpenPack { .. }
        | WorkspaceRequest::OpenTheme { .. }
        | WorkspaceRequest::PackOpener { .. } => None,
    }
}

/// Diffuse `json` (une réponse déjà sérialisée) à toutes les fenêtres sous `event`.
fn emit_json(app: &AppHandle, event: &str, json: &str) {
    use tauri::Emitter;
    let sent = serde_json::from_str::<serde_json::Value>(json)
        .map_err(|e| e.to_string())
        .and_then(|value| app.emit(event, value).map_err(|e| e.to_string()));
    if let Err(e) = sent {
        tracing::warn!(event, error = %e, "réponse non diffusée");
    }
}

/// Une requête pour l'onglet `tab`.
#[tauri::command]
pub async fn request(app: AppHandle, tab: String, req: Request) -> Result<Response, String> {
    let reply = blocking(&app, move |app, state| {
        if state.closing.load(Ordering::SeqCst) {
            return Err(CLOSING.to_owned());
        }
        let changes_deck = req.changes_deck();
        let (reply, changed) = Workspace::request(&state.workspace, &tab, req);
        if changed {
            windows::emit_view(app);
        }
        if reply.is_ok() && changes_deck {
            // Le deck a changé : les post-its orphelins de l'agent se ferment.
            state.stack.after_change(&tab);
        }
        // A deck-pack tab's save reached its owner deck: the view, then the owner's notes.
        Workspace::drain_touched(&state.workspace, || windows::emit_view(app), |owner| state.stack.after_change(owner));
        Ok(reply.map_err(|e| state.refuse(&e)))
    })
    .await?;
    reply.map(Response::new)
}

/// Une requête de l'espace de travail ; les fenêtres suivent.
///
/// Fermer la dernière fenêtre ne la ferme pas (voir `Workspace::apply` et
/// `WorkspaceOutcome::LastWindow`) : elle quitte l'app en gardant tout, la
/// règle vit dans la session, pas seulement dans l'interface.
#[tauri::command]
pub async fn workspace(app: AppHandle, req: WorkspaceRequest) -> Result<Response, String> {
    let (changes, focus) = classify(&req);
    let event = broadcast(&req);
    let reply = blocking(&app, move |app, state| {
        if state.closing.load(Ordering::SeqCst) {
            return Err(CLOSING.to_owned());
        }
        let (outcome, closed) = Workspace::closed_by(&state.workspace, |ws| Workspace::apply(ws, req));
        // Onglets fermés (ou vierge remplacé) : leur agent s'arrête.
        state.stop_closed_agents(&closed);
        let outcome = outcome.map_err(|e| state.refuse(&e))?;
        // Un onglet importé à fine-tuner : son tour part dès que l'onglet existe (la vue suit).
        if let Some(tab) = outcome.tune_tab() {
            state.tune(tab);
        }
        // A tab's deck changed: the agent's orphaned notes close, as after a deck request.
        if let Some(tab) = outcome.after_change_tab() {
            state.stack.after_change(tab);
        }
        // A close's flush reached owner decks: the view, then their notes.
        Workspace::drain_touched(&state.workspace, || windows::emit_view(app), |owner| state.stack.after_change(owner));
        if outcome.is_last_window() {
            state::shutdown(app);
            return Ok(outcome.into_json());
        }
        if changes {
            let view = windows::sync(app, None, focus);
            serde_json::to_string(&view).map_err(|e| e.to_string())
        } else {
            let json = outcome.into_json();
            if let Some(event) = event {
                emit_json(app, event, &json);
            }
            Ok(json)
        }
    })
    .await?;
    Ok(Response::new(reply))
}

/// Quitter en gardant tout (repli de l'interface pour la dernière fenêtre).
#[tauri::command]
pub async fn quit(app: AppHandle) {
    let app2 = app.clone();
    let _ = tauri::async_runtime::spawn_blocking(move || state::shutdown(&app2)).await;
}

/// Dictée, hors du fil principal : les requêtes au moteur d'Apple bloquent.
///
/// `Start` (ouverture du flux) interroge Speech, jusqu'à 5 s ; téléchargement,
/// micro et Whisper ont leurs fils.
#[tauri::command]
pub async fn stt(engine: tauri::State<'_, Stt>, req: SttRequest) -> Result<Status, String> {
    let engine = Stt::clone(&engine);
    tauri::async_runtime::spawn_blocking(move || engine.handle(req)).await.map_err(|e| e.to_string())
}

/// Agent d'un onglet : consigne, annulation, nouvelle conversation, action
/// d'une surface, reprise du fil (chaque commande nomme son onglet).
///
/// Hors du fil principal (`blocking`) : une action de surface (réglage,
/// variante) applique au deck par `AppSessions`, qui verrouille l'espace de
/// travail et redessine les barres d'onglets (`windows::emit_view`).
#[tauri::command]
pub async fn agent(app: AppHandle, req: AgentCommand) -> Result<(), String> {
    blocking(&app, move |_, state| {
        if state.closing.load(Ordering::SeqCst) {
            return Err(CLOSING.to_owned());
        }
        state.stack.handle(req)
    })
    .await
}

/// `candidate` est-il plus récent que le dernier focus appliqué ?
///
/// Extraite pour le test : les numéros ne se recroisent jamais (compteur
/// entier, jamais remis à zéro), donc « plus récent » est un simple `>`.
fn is_newer(candidate: u64, last_applied: u64) -> bool {
    candidate > last_applied
}

/// Une fenêtre passe au premier plan.
///
/// Le numéro est pris ici, sur le fil principal, à l'arrivée de
/// l'événement : l'ordre d'arrivée fait foi. Le travail (verrou de l'espace
/// de travail, jamais abandonné s'il est occupé, pour ne jamais perdre un
/// focus) tourne ensuite hors du fil principal (`spawn_blocking`) ; comme
/// plusieurs tâches peuvent se chevaucher et finir dans le désordre, chacune
/// n'applique son focus que s'il est plus récent que le dernier appliqué
/// (`is_newer`, sous le même verrou que `Workspace::focus`).
pub fn focused(app: &AppHandle, label: &str) {
    // La fenêtre de l'agent n'est pas une fenêtre de l'espace de travail : la cible ne change pas.
    if !oku_slides_session::is_window_label(label) {
        return;
    }
    let Some(state) = app.try_state::<AppState>() else { return };
    let seq = state.focus_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    let label = label.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(state) = app.try_state::<AppState>() else { return };
        let mut ws = Workspace::lock(&state.workspace);
        let mut last = state.focus_applied.lock().unwrap_or_else(PoisonError::into_inner);
        if !is_newer(seq, *last) {
            return;
        }
        *last = seq;
        drop(last);
        let before = ws.focused().to_owned();
        if let Err(e) = ws.focus(&label) {
            // Une fenêtre en cours de création n'y est pas encore.
            tracing::debug!(label, error = %e.one_line(), "focus ignoré");
            return;
        }
        let changed = ws.focused() != before;
        // `emit_view` prend le verrou de `sync` puis celui de l'espace de travail : rendre celui-ci d'abord.
        drop(ws);
        if changed {
            // La fenêtre de l'agent suit l'onglet actif de la fenêtre au premier plan.
            windows::emit_view(&app);
        }
    });
}

/// Fermeture native demandée (bouton, ⌘W du système, Alt+F4).
///
/// La dernière fenêtre quitte l'app en gardant tout, sans passer par la
/// page ; une autre prévient sa page, qui pose la question pour ses onglets
/// modifiés. Espace de travail occupé (`try_workspace` refuse) : on
/// transmet à la page par prudence — si c'était bien la dernière fenêtre,
/// `workspace` (ci-dessus) le redécouvrira atomiquement à la prochaine
/// `close_window` et quittera alors (même prédicat, `Workspace::is_only_window`,
/// utilisé ici en aperçu hors verrou et là-bas sous verrou).
pub fn close_requested(app: &AppHandle, label: &str) {
    use tauri::Emitter;
    let last = app.try_state::<AppState>().and_then(|state| Some(state.try_workspace()?.is_only_window(label)));
    if last == Some(true) {
        let app2 = app.clone();
        tauri::async_runtime::spawn_blocking(move || state::shutdown(&app2));
        return;
    }
    if let Err(e) = app.emit_to(label, "close-requested", ()) {
        tracing::warn!(label, error = %e, "fermeture non transmise");
    }
}

/// Ouvre des fichiers reçus, hors du fil principal ; la fenêtre active passe devant.
///
/// `AppState::closing` est vérifié une fois ici, avant de verrouiller quoi
/// que ce soit : une économie (rien engagé pour rien juste avant de quitter),
/// pas la garantie — une seconde instance ou macOS (`RunEvent::Opened`)
/// peuvent transmettre des fichiers pendant que `Workspace::settle` avance
/// entre-temps. La garantie vient de `Workspace` lui-même, qui refuse chaque
/// fichier sous son propre verrou une fois `closed` posé (voir
/// `Workspace::open_files` et `apply` sur `OpenTab`).
pub fn open_files(app: &AppHandle, files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let synced = blocking(&app, move |app, state| {
            if state.closing.load(Ordering::SeqCst) {
                return Err(CLOSING.to_owned());
            }
            let ((), closed) = Workspace::closed_by(&state.workspace, |ws| Workspace::open_files(ws, &files));
            // Un onglet vierge remplacé par un fichier : son agent s'arrête.
            state.stop_closed_agents(&closed);
            Ok(windows::sync(app, None, true))
        })
        .await;
        if let Err(e) = synced {
            tracing::warn!(error = %e, "fichiers reçus non ouverts");
        }
    });
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod commands_tests;
