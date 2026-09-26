//! Panneau de droite : une surface A2UI décrite par la session.
//!
//! En édition des modèles, le panneau montre le nom et la description du
//! modèle, et sa grille. L'interface rend la surface et renvoie ses actions :
//! `rename_template` et `describe_template` (contexte `slide`, `value`),
//! `set_grid` (contexte `slide`, `columns`, `rows`).

use serde_json::{Value, json};
use slider_a2ui::{Action, Component, ComponentKind, Dynamic, ServerMessage, TextVariant};
use slider_core::Slide;
use slider_i18n::Localizer;
use slider_render::GridBox;

/// Identifiant de la surface du panneau.
pub const PANEL_SURFACE: &str = "panel";

/// Bornes des compteurs de la grille.
const GRID_MIN: i64 = 1;
const GRID_MAX: i64 = 24;

/// Messages qui décrivent le panneau pour `slide` ; rien hors des modèles.
pub(crate) fn panel(slide: &Slide, grid: Option<&GridBox>, editing: bool, l: &Localizer) -> Vec<ServerMessage> {
    if !editing {
        return Vec::new();
    }
    let mut cards = vec!["template".to_owned()];
    let mut components = template_card(slide, l);
    let mut data = json!({
        "template": {
            "name": slide.name.as_deref().unwrap_or_default(),
            "description": slide.description.as_deref().unwrap_or_default(),
        }
    });
    if let Some(grid) = grid {
        cards.push("grid".to_owned());
        components.extend(grid_card(slide, l));
        data["grid"] = json!({ "columns": grid.columns, "rows": grid.rows });
    }
    components.insert(0, Component::new("root", ComponentKind::Column { children: cards }));
    vec![
        ServerMessage::create_surface(PANEL_SURFACE),
        ServerMessage::update_components(PANEL_SURFACE, components),
        ServerMessage::set_data_model(PANEL_SURFACE, data),
    ]
}

/// Post-it du modèle : titre, nom, description.
fn template_card(slide: &Slide, l: &Localizer) -> Vec<Component> {
    let field = |id: &str, label: &str, path: &str, action: &str| {
        Component::new(
            id,
            ComponentKind::TextField {
                label: Dynamic::Literal(l.tr(label)),
                value: Dynamic::path(path),
                action: Some(Action::event(action, [slide_context(slide), ("value", Dynamic::path(path))])),
            },
        )
    };
    vec![
        Component::new("template", ComponentKind::Card { child: "template-body".into() }),
        column("template-body", &["template-title", "template-name", "template-description"]),
        title("template-title", l.tr("session-panel-template")),
        field("template-name", "session-panel-name", "/template/name", "rename_template"),
        field("template-description", "session-panel-description", "/template/description", "describe_template"),
    ]
}

/// Post-it de la grille : colonnes et lignes.
fn grid_card(slide: &Slide, l: &Localizer) -> Vec<Component> {
    let stepper = |id: &str, label: &str, path: &str| {
        let action = Action::event(
            "set_grid",
            [slide_context(slide), ("columns", Dynamic::path("/grid/columns")), ("rows", Dynamic::path("/grid/rows"))],
        );
        Component::new(
            id,
            ComponentKind::Stepper {
                label: Dynamic::Literal(l.tr(label)),
                value: Dynamic::path(path),
                min: Some(GRID_MIN),
                max: Some(GRID_MAX),
                action: Some(action),
            },
        )
    };
    vec![
        Component::new("grid", ComponentKind::Card { child: "grid-body".into() }),
        column("grid-body", &["grid-title", "grid-columns", "grid-rows"]),
        title("grid-title", l.tr("session-panel-grid")),
        stepper("grid-columns", "session-panel-columns", "/grid/columns"),
        stepper("grid-rows", "session-panel-rows", "/grid/rows"),
    ]
}

fn slide_context(slide: &Slide) -> (&'static str, Dynamic<Value>) {
    ("slide", Dynamic::Literal(Value::String(slide.id.clone())))
}

fn column(id: &str, children: &[&str]) -> Component {
    Component::new(id, ComponentKind::Column { children: children.iter().map(|c| (*c).to_owned()).collect() })
}

fn title(id: &str, text: String) -> Component {
    Component::new(id, ComponentKind::Text { text: Dynamic::Literal(text), variant: Some(TextVariant::H4) })
}
