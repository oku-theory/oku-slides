//! `just agent-smoke <deck> "<consigne>"` : lance la vraie pile (agent
//! configuré, outils MCP) sur un deck, envoie la consigne, affiche le fil,
//! répond « oui » / première option à toute question, puis enregistre une
//! copie du deck en JSON à côté (`<deck>.agent.json`).
//!
//! Un seul onglet (`"t-1"`) : la pile de l'agent est par onglet, mais cet
//! exemple n'en a besoin que d'un.

use std::sync::{Arc, Mutex};

use oku_slides_agent::{AgentCommand, Stack};
use oku_slides_session::agent::{AgentScope, AgentUi, Sessions};
use oku_slides_session::{Fonts, Platform, Session};
use serde_json::{Map, Value};
use tokio::sync::mpsc;

/// L'onglet unique de cet exemple.
const TAB: &str = "t-1";

/// Sync : `Stack` (comme `Platform`, dont il garde une copie) doit s'arrêter
/// hors d'un contexte async si c'est sa dernière copie (`Storage` possède un
/// runtime tokio ; le lâcher depuis l'intérieur ferait paniquer Tokio).
fn main() -> color_eyre::Result<()> {
    // Relancé en processus de rendu par `Platform::load` : sert, puis sort.
    oku_slides_render_worker::exit_if_worker();
    color_eyre::install()?;
    oku_slides_platform::telemetry::init();
    let mut args = std::env::args().skip(1);
    let (Some(deck), Some(prompt)) = (args.next(), args.next()) else {
        color_eyre::eyre::bail!("usage : smoke <deck.json|.oku-slides.pdf> \"<consigne>\"");
    };
    let platform = Platform::load()?;
    let mut session = Session::new(Fonts::default(), &platform)?;
    session.open(std::path::Path::new(&deck))?;
    let shared = Arc::new(Mutex::new(session));
    let sessions: Arc<dyn Sessions> = shared.clone();

    // Les outils et l'agent vivent dans ce runtime tokio, construit à la main
    // (comme `oku-slides serve`) pour pouvoir arrêter la pile avant lui.
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let stack = Stack::start(sessions, &platform, None, runtime.handle().clone());

    let (tx, mut rx) = mpsc::unbounded_channel::<(&'static str, String)>();
    stack.forward(move |name, json| {
        let _ = tx.send((name, json));
    });
    stack
        .handle(AgentCommand::Prompt {
            tab: TAB.to_owned(),
            scope: AgentScope::Slides,
            text: prompt,
            ui: AgentUi::default(),
            answer: None,
        })
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    let mut worked = false;
    let mut started = false;
    runtime.block_on(async {
        while let Some((channel, json)) = rx.recv().await {
            let envelope: Value = serde_json::from_str(&json)?;
            let event = &envelope["event"];
            let kind = event["kind"].as_str().unwrap_or_default();
            match channel {
                "agent" => {
                    println!("{event}");
                    if kind == "status" && event["state"] == "working" {
                        worked = true;
                    } else if kind == "status" && event["state"] == "starting" {
                        started = true;
                    } else if kind == "status" && event["state"] == "idle" && (worked || started) {
                        break;
                    }
                }
                "mcp" if kind == "surface" && event["surface"]["question"].as_bool() == Some(true) => {
                    let surface = &event["surface"];
                    let id = surface["id"].as_str().unwrap_or_default().to_owned();
                    println!("surface {id} ({:?})", surface["place"]);
                    let (name, context) = first_answer(surface);
                    stack
                        .handle(AgentCommand::SurfaceAction { tab: TAB.to_owned(), surface: id, name, context })
                        .map_err(|e| color_eyre::eyre::eyre!(e))?;
                }
                "mcp" if kind == "surface_patch" || kind == "surface" => println!("{event}"),
                _ => {}
            }
        }
        Ok::<(), color_eyre::eyre::Error>(())
    })?;

    // La pile de l'agent (et sa copie de la plateforme) s'arrêtent avant le
    // runtime tokio qui les porte.
    drop(stack);
    drop(platform);
    drop(runtime);

    let out = format!("{deck}.agent.json");
    let json = shared.lock().map_err(|_| color_eyre::eyre::eyre!("session"))?.deck().to_json()?;
    std::fs::write(&out, json)?;
    println!("deck : {out}");
    Ok(())
}

/// La première action proposée par la surface (bouton principal, sinon première option).
fn first_answer(surface: &Value) -> (String, Map<String, Value>) {
    let text = surface.to_string();
    let mut context = Map::new();
    if text.contains("\"confirm\"") {
        context.insert("confirmed".into(), Value::Bool(true));
        return ("confirm".into(), context);
    }
    if text.contains("\"permission\"") {
        // Le bouton principal (« autoriser une fois ») : l'ordre des options n'est plus celui de l'agent.
        let index = primary_index(surface).unwrap_or(0);
        context.insert("index".into(), Value::from(index));
        return ("permission".into(), context);
    }
    if text.contains("\"rewrite\"") {
        context.insert("kept".into(), Value::from(0));
        let first = surface.pointer("/messages/2/updateDataModel/value/draft").cloned().unwrap_or(Value::Null);
        // Un élément (ancre pas un champ `@…`) exige `paragraphs` : la vraie
        // interface y met le markdown reparsé (`withRewriteParagraphs`,
        // `ui/src/agent/rewrite.ts`) ; ici, sans parseur, un paragraphe simple
        // par ligne suffit puisque les propositions de cet exemple sont en
        // texte brut.
        let element = surface.pointer("/anchor/element").and_then(Value::as_str).unwrap_or_default();
        if !element.starts_with('@')
            && let Some(text) = first.as_str()
        {
            let paragraphs: Vec<_> = text.lines().map(oku_slides_core::Paragraph::plain).collect();
            context
                .insert("paragraphs".into(), serde_json::to_value(paragraphs).unwrap_or_else(|_| Value::Array(vec![])));
        }
        context.insert("text".into(), first);
        return ("rewrite".into(), context);
    }
    if let Some(id) =
        surface.pointer("/messages/1/updateComponents/components").and_then(Value::as_array).and_then(|cs| {
            cs.iter().find_map(|c| c.get("options").and_then(|o| o.get(0)).and_then(|o| o.get("id")).cloned())
        })
    {
        context.insert("choice".into(), id);
        return ("choose".into(), context);
    }
    if text.contains("\"variant\"") {
        context.insert("index".into(), Value::from(0));
        return ("variant".into(), context);
    }
    ("dismiss".into(), context)
}

/// L'index d'action du bouton principal d'une surface, s'il y en a un.
fn primary_index(surface: &Value) -> Option<u64> {
    surface["messages"].as_array()?.iter().find_map(|m| m.get("updateComponents"))?["components"]
        .as_array()?
        .iter()
        .find(|c| c["primary"] == true)?["action"]["event"]["context"]["index"]
        .as_u64()
}
