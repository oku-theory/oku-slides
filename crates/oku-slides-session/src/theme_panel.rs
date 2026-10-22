//! Surface `host-theme` : chaque jeton du brouillon du thème, par section,
//! avec son état et ses actions (`host_theme`, `host_token`). L'interface la
//! rend au dock de l'agent, en mode pack.
//!
//! Chaque action est l'évènement `token` : `{token, action, value?, name?}`,
//! avec `token` le chemin du jeton (un tableau de segments) et `action` un
//! [`TokenAction`]. Jamais de clé `path` dans le contexte : @a2ui y verrait
//! une liaison de données et l'action ne partirait pas. Un
//! champ envoie `set` et sa valeur (`value`), ou le changement d'un tableau
//! sous `change` au lieu de `value` ; les boutons envoient `reset`, `remove`
//! ou `clear` sans valeur. La ligne « nouvelle couleur » envoie `set` avec
//! `token: ["color"]` et le nom saisi sous `name`, à ajouter au chemin. Le
//! champ de filtre envoie l'évènement `filter {filter}`.

use oku_slides_a2ui::{Action, Component, ComponentKind, Dynamic, TextVariant, Tone};
use oku_slides_core::Op;
use oku_slides_i18n::Localizer;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use snafu::ensure;

use crate::agent::Surface;
use crate::agent::forms::{FieldEvent, Place, field_components, lit, surface};
use crate::error::{InvalidTokenValueSnafu, NotEditingPackSnafu};
use crate::theme_controls::{Shown, token_fields, token_value};
use crate::theme_schema::{Section, TokenSpec, default_tokens, lookup, merge, schema, spec};
use crate::{Mode, Result, Session};

/// Surface des jetons du brouillon du thème.
pub(crate) const HOST_THEME: &str = "host-theme";

/// Évènement des actions sur un jeton.
const TOKEN_EVENT: &str = "token";

/// Sections, dans l'ordre d'affichage, et leur nom (clés Fluent `theme-section-*`).
const SECTIONS: [(Section, &str); 10] = [
    (Section::Colors, "colors"),
    (Section::Typography, "typography"),
    (Section::Scripts, "scripts"),
    (Section::Spacing, "spacing"),
    (Section::Shapes, "shapes"),
    (Section::Page, "page"),
    (Section::Charts, "charts"),
    (Section::Components, "components"),
    (Section::AppLight, "app-light"),
    (Section::AppDark, "app-dark"),
];

/// Où en est un jeton du brouillon par rapport à sa base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenState {
    /// Absent du brouillon : la valeur par défaut s'applique.
    Default,
    /// Dans le brouillon, comme dans la base.
    Defined,
    /// Dans le brouillon, autrement que dans la base (ou absent d'elle).
    Modified,
}

/// Une action sur un jeton du brouillon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokenAction {
    /// Pose la valeur envoyée.
    Set,
    /// Reprend la valeur de la base, ou retire le jeton si la base ne l'a pas.
    Reset,
    /// Retire le jeton du brouillon (refusé s'il est nommé ailleurs).
    Remove,
    /// Vide le jeton (`null`), s'il le permet.
    Clear,
}

/// L'état du jeton `path` : dans le brouillon `draft` ou non, comme dans `base` ou non.
pub(crate) fn token_state(path: &[String], draft: &Map<String, Value>, base: &Map<String, Value>) -> TokenState {
    match (lookup(draft, path), lookup(base, path)) {
        (None, _) => TokenState::Default,
        (Some(d), Some(b)) if d == b => TokenState::Defined,
        _ => TokenState::Modified,
    }
}

/// Le libellé d'un jeton : `theme-token-<chemin à tirets>`, ou le nom d'une couleur libre.
fn token_label(path: &[String], l: &Localizer) -> String {
    match schema().iter().any(|s| s.path == path) {
        true => l.tr(&format!("theme-token-{}", path.join("-"))),
        false => path.last().cloned().unwrap_or_default(),
    }
}

/// Une ligne de la surface : un jeton et ce qu'on en montre.
struct Row {
    path: Vec<String>,
    spec: &'static TokenSpec,
    label: String,
    /// Valeur effective : du brouillon, sinon des défauts.
    value: Option<Value>,
    state: TokenState,
    /// Le brouillon diffère de la base à ce chemin (↺ possible).
    differs: bool,
}

impl Row {
    fn key(&self) -> String {
        self.path.join(".")
    }

    fn matches(&self, filter: &str) -> bool {
        filter.is_empty() || self.label.to_lowercase().contains(filter) || self.key().to_lowercase().contains(filter)
    }
}

/// L'action `action` du jeton `path`, plus les entrées `extra`.
fn token_action(path: &[String], action: &str, extra: Vec<(&'static str, Dynamic<Value>)>) -> Action {
    let mut context = vec![("token", lit(json!(path))), ("action", lit(json!(action)))];
    context.extend(extra);
    Action::event(TOKEN_EVENT, context)
}

/// Les composants d'une surface en construction et son modèle de données.
#[derive(Default)]
struct Builder {
    components: Vec<Component>,
    data: Map<String, Value>,
}

impl Builder {
    /// Ajoute les composants d'une ligne ; renvoie l'id de la ligne.
    fn row(&mut self, row: &Row, palette: &[String], tokens: &Map<String, Value>, l: &Localizer) -> String {
        let key = row.key();
        let mut children = Vec::new();
        let shown = Shown { key: &key, path: &row.path, spec: row.spec, label: &row.label, value: row.value.as_ref() };
        for f in token_fields(&shown, tokens, l) {
            let event = FieldEvent {
                name: TOKEN_EVENT,
                context: vec![("token", lit(json!(f.path))), ("action", lit(json!("set")))],
            };
            let (component, entries) = field_components(&f.key, &f.field, &event, palette);
            self.components.push(component);
            self.data.extend(entries);
            children.push(f.key);
        }
        let (state, tone) = match row.state {
            TokenState::Default => ("default", Some(Tone::Neutral)),
            TokenState::Defined => ("defined", None),
            TokenState::Modified => ("modified", Some(Tone::Accent)),
        };
        let badge = format!("{key}:state");
        let text = Dynamic::Literal(l.tr(&format!("theme-state-{state}")));
        self.components.push(Component::new(&badge, ComponentKind::Badge { text, tone }));
        children.push(badge);
        let present = row.state != TokenState::Default;
        let nonnull = row.value.as_ref().is_some_and(|v| !v.is_null());
        for (action, shown) in [("reset", row.differs), ("remove", present), ("clear", row.spec.nullable && nonnull)] {
            if shown {
                let id = format!("{key}:{action}");
                let label = l.tr(&format!("theme-action-{action}"));
                self.components.push(Component::button(&id, label, false, token_action(&row.path, action, vec![])));
                children.push(id);
            }
        }
        let id = format!("{key}:row");
        let refs: Vec<&str> = children.iter().map(String::as_str).collect();
        self.components.push(Component::row(&id, &refs));
        id
    }

    /// La ligne « nouvelle couleur » : nom, couleur, bouton d'ajout.
    fn new_color(&mut self, palette: &[String], l: &Localizer) -> String {
        let field = |label: &str| Dynamic::Literal(l.tr(label));
        self.components.push(Component::new(
            "new-color:name",
            ComponentKind::TextField {
                label: field("theme-new-color-name"),
                value: Dynamic::path("/new-color-name"),
                action: None,
            },
        ));
        self.components.push(Component::new(
            "new-color:value",
            ComponentKind::ColorField {
                label: field("theme-new-color-value"),
                value: Dynamic::path("/new-color-value"),
                palette: palette.to_vec(),
                action: None,
            },
        ));
        let extra = vec![("name", Dynamic::path("/new-color-name")), ("value", Dynamic::path("/new-color-value"))];
        let add = token_action(&["color".to_owned()], "set", extra);
        self.components.push(Component::button("new-color:add", l.tr("theme-action-add-color"), false, add));
        self.components.push(Component::row("new-color", &["new-color:name", "new-color:value", "new-color:add"]));
        self.data.insert("new-color-name".into(), json!(""));
        self.data.insert("new-color-value".into(), json!(""));
        "new-color".to_owned()
    }
}

impl Session {
    /// Les lignes du brouillon : le schéma, puis ses couleurs libres.
    fn theme_rows(&self) -> Vec<Row> {
        let (draft, base, l) = (self.edited_tokens(), self.base_tokens(), &self.localizer);
        let row = |path: Vec<String>, spec: &'static TokenSpec| {
            let value = lookup(&draft, &path).or_else(|| lookup(default_tokens(), &path)).cloned();
            Row {
                label: token_label(&path, l),
                state: token_state(&path, &draft, &base),
                differs: lookup(&draft, &path) != lookup(&base, &path),
                value,
                spec,
                path,
            }
        };
        let mut rows: Vec<Row> = schema().iter().map(|s| row(s.path.clone(), s)).collect();
        let free = draft.get("color").and_then(Value::as_object).into_iter().flatten();
        for name in free.map(|(name, _)| name) {
            let path = vec!["color".to_owned(), name.clone()];
            if let Some(s) = spec(&path).filter(|_| !schema().iter().any(|s| s.path == path)) {
                rows.push(row(path, s));
            }
        }
        rows
    }

    /// La surface des jetons du brouillon (mode pack) ; `filter` garde les
    /// jetons dont le libellé traduit ou le chemin le contient (casse ignorée).
    pub(crate) fn host_theme(&self, filter: &str) -> Result<Option<Surface>> {
        let Mode::Pack { theme, .. } = &self.mode else { return Ok(None) };
        let l = &self.localizer;
        let typed = filter;
        // Normalisé pour la recherche seulement : le champ garde la saisie telle quelle.
        let filter = filter.trim().to_lowercase();
        let palette: Vec<String> = self.appearance.slide.values().cloned().collect();
        let mut tokens = default_tokens().clone();
        merge(&mut tokens, &self.edited_tokens());
        let rows: Vec<Row> = self.theme_rows().into_iter().filter(|r| r.matches(&filter)).collect();
        let mut b = Builder::default();
        let mut body = vec!["title".to_owned(), "filter".to_owned()];
        b.components.push(Component::text("title", theme.name.clone(), Some(TextVariant::H4)));
        b.components.push(Component::new(
            "filter",
            ComponentKind::TextField {
                label: Dynamic::Literal(l.tr("theme-filter")),
                value: Dynamic::path("/filter"),
                action: Some(Action::event("filter", [("filter", Dynamic::path("/filter"))])),
            },
        ));
        b.data.insert("filter".into(), json!(typed));
        for (section, name) in SECTIONS {
            let mut ids: Vec<String> = Vec::new();
            for r in rows.iter().filter(|r| r.spec.section == section) {
                ids.push(b.row(r, &palette, &tokens, l));
            }
            if section == Section::Colors && filter.is_empty() {
                ids.push(b.new_color(&palette, l));
            }
            if ids.is_empty() {
                continue;
            }
            let (id, rows_id) = (format!("s-{name}"), format!("s-{name}-rows"));
            let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
            b.components.push(Component::column(&rows_id, &refs));
            let open = !filter.is_empty() || section == Section::Colors;
            let summary = Dynamic::Literal(l.tr(&format!("theme-section-{name}")));
            b.components
                .push(Component::new(&id, ComponentKind::Disclosure { summary, child: rows_id, open: Some(open) }));
            body.push(id);
        }
        let refs: Vec<&str> = body.iter().map(String::as_str).collect();
        b.components.insert(0, Component::column("body", &refs));
        b.components.insert(0, Component::card("root", "body"));
        Ok(Some(surface(HOST_THEME, Place::Thread, None, b.components, Value::Object(b.data))))
    }

    /// Agit sur le jeton `path` du brouillon : une étape d'annulation, vérifiée
    /// par le schéma ; sans effet si le jeton a déjà la valeur visée.
    pub(crate) fn host_token(&mut self, path: Vec<String>, action: TokenAction, value: Option<Value>) -> Result<()> {
        ensure!(self.in_pack_mode(), NotEditingPackSnafu);
        let draft = self.edited_tokens();
        let (path, target) = match action {
            TokenAction::Set => {
                let current = |p: &[String]| lookup(&draft, p).or_else(|| lookup(default_tokens(), p)).cloned();
                let (path, v) = token_value(path, value.unwrap_or(Value::Null), current)?;
                (path, Some(v))
            }
            TokenAction::Reset => {
                let base = lookup(&self.base_tokens(), &path).cloned();
                (path, base)
            }
            TokenAction::Remove => (path, None),
            TokenAction::Clear => (path, Some(Value::Null)),
        };
        if spec(&path).is_none() {
            let reason = if matches!(path.as_slice(), [g, _] if g == "color") { "bad-name" } else { "unknown-token" };
            return InvalidTokenValueSnafu { path: path.join("."), reason }.fail()?;
        }
        if lookup(&draft, &path) == target.as_ref() {
            return Ok(());
        }
        let token = token_label(&path, &self.localizer);
        let label = self.localizer.tr_with("session-undo-theme-token", &[("token", token.into())]);
        let op = match target {
            Some(value) => Op::SetToken { path, value },
            None => Op::UnsetToken { path },
        };
        self.apply(&label, vec![op], None)
    }
}

#[cfg(test)]
#[path = "theme_panel_tests.rs"]
mod theme_panel_tests;
