//! The `host-theme` surface: each token of a theme tab's draft, by section,
//! with its state and its actions (`host_theme`, `host_token`). The interface
//! renders it in the right column of a theme tab.
//!
//! The tokens are those of the specimen deck's composed schema: the global
//! sections, then one per pinned pack (named like it, in namespace order),
//! then the interface variants. A token of a built-in pack is named through
//! Fluent (`theme-token-component-<ns>-…`), one of another pack by its entry's
//! `label`, else by its dotted path. At the top: the themes the draft
//! extends, one row each with its up, down and remove buttons, then the list
//! of themes to add (event [`THEME_EXTENDS_EVENT`]: `{edit, theme}`, an
//! `ExtendsEdit`).
//!
//! Each action is the `token` event: `{token, action, value?, name?}`, with
//! `token` the token's path (an array of segments) and `action` a
//! [`TokenAction`]. Never a `path` key in the context: @a2ui would see a data
//! binding there and the action would not leave. A field sends `set` and its
//! value (`value`), or an array's change under `change` instead of `value`;
//! the buttons send `reset`, `remove` or `clear` without a value. The "new
//! color" row sends `set` with `token: ["color"]` and the typed name under
//! `name`, to append to the path. The filter field sends the `filter {filter}` event.

use oku_slides_a2ui::{Action, ButtonIcon, ChoiceOption, Component, ComponentKind, Dynamic, TextVariant, Tone};
use oku_slides_core::Op;
use oku_slides_i18n::Localizer;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::agent::Surface;
use crate::agent::forms::{FieldEvent, Place, field_components, lit, surface};
use crate::error::InvalidTokenValueSnafu;
use crate::theme_composed::is_builtin_namespace;
use crate::theme_controls::{Shown, token_fields, token_value};
use crate::theme_edit::ThemeParentView;
use crate::theme_schema::{Section, TokenSpec, default_tokens, lookup, merge, spec};
use crate::{Mode, Result, Session};

/// Surface des jetons du brouillon du thème.
pub(crate) const HOST_THEME: &str = "host-theme";

/// Évènement des actions sur un jeton.
const TOKEN_EVENT: &str = "token";

/// Évènement des changements de la liste `extends` du brouillon : `{edit, theme}`.
pub const THEME_EXTENDS_EVENT: &str = "extends";

/// Les boutons d'une ligne de parent : changement envoyé, clé Fluent, pictogramme.
const EXTENDS_BUTTONS: [(&str, &str, ButtonIcon); 3] = [
    ("up", "theme-extends-up", ButtonIcon::Up),
    ("down", "theme-extends-down", ButtonIcon::Down),
    ("remove", "theme-extends-remove", ButtonIcon::Remove),
];

/// Sections globales d'avant les packs, dans l'ordre, et leur nom (clés Fluent `theme-section-*`).
const SECTIONS_BEFORE: [(Section, &str); 9] = [
    (Section::Colors, "colors"),
    (Section::Typography, "typography"),
    (Section::Headings, "headings"),
    (Section::Scripts, "scripts"),
    (Section::Spacing, "spacing"),
    (Section::Shapes, "shapes"),
    (Section::Page, "page"),
    (Section::Charts, "charts"),
    (Section::Frame, "frame"),
];

/// Sections globales d'après les packs.
const SECTIONS_AFTER: [(Section, &str); 2] = [(Section::AppLight, "app-light"), (Section::AppDark, "app-dark")];

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

/// Le libellé d'un jeton de `schema`.
///
/// `theme-token-<chemin à tirets>` pour une entrée globale ou d'un pack
/// intégré ; le `label` de l'entrée d'un autre pack, sinon son chemin pointé
/// dans le pack ; le nom d'une couleur libre.
pub(crate) fn token_label(schema: &[TokenSpec], path: &[String], l: &Localizer) -> String {
    match schema.iter().find(|s| s.path == path) {
        Some(TokenSpec { section: Section::Pack(ns), label, .. }) if !is_builtin_namespace(ns) => {
            label.clone().unwrap_or_else(|| path.get(2..).unwrap_or_default().join("."))
        }
        Some(_) => l.tr(&format!("theme-token-{}", path.join("-"))),
        None => path.last().cloned().unwrap_or_default(),
    }
}

/// Une ligne de la surface : un jeton et ce qu'on en montre.
struct Row<'s> {
    path: Vec<String>,
    spec: &'s TokenSpec,
    label: String,
    /// Valeur effective : du brouillon, sinon des défauts.
    value: Option<Value>,
    state: TokenState,
    /// Le brouillon diffère de la base à ce chemin (↺ possible).
    differs: bool,
}

impl Row<'_> {
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
    fn row(&mut self, row: &Row<'_>, palette: &[String], tokens: &Map<String, Value>, l: &Localizer) -> String {
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

    /// Les thèmes qu'étend le brouillon, `parents`, puis la liste `choices` des
    /// thèmes à ajouter (absente si vide) ; renvoie l'id de la colonne.
    fn extends(&mut self, parents: &[ThemeParentView], choices: Vec<ChoiceOption>, l: &Localizer) -> String {
        self.components.push(Component::text("extends-title", l.tr("theme-extends-title"), Some(TextVariant::Body)));
        let mut ids = vec!["extends-title".to_owned()];
        for (i, parent) in parents.iter().enumerate() {
            let id = format!("extends:{}", parent.namespace);
            let name = format!("{id}:name");
            self.components.push(Component::text(&name, parent.name.clone(), None));
            let mut children = vec![name];
            let shown = [i > 0, i + 1 < parents.len(), true];
            for ((edit, label, icon), shown) in EXTENDS_BUTTONS.into_iter().zip(shown) {
                if !shown {
                    continue;
                }
                let button = format!("{id}:{edit}");
                let context = [("edit", lit(json!(edit))), ("theme", lit(json!(parent.namespace)))];
                let action = Action::event(THEME_EXTENDS_EVENT, context);
                self.components.push(Component::button(&button, l.tr(label), false, action).icon(icon));
                children.push(button);
            }
            let refs: Vec<&str> = children.iter().map(String::as_str).collect();
            self.components.push(Component::row(&id, &refs));
            ids.push(id);
        }
        if !choices.is_empty() {
            let context = [("edit", lit(json!("add"))), ("theme", Dynamic::path("/extends_add"))];
            self.components.push(Component::new(
                "extends-add",
                ComponentKind::Select {
                    label: Dynamic::Literal(l.tr("theme-extends-add")),
                    value: Dynamic::path("/extends_add"),
                    options: choices,
                    action: Some(Action::event(THEME_EXTENDS_EVENT, context)),
                },
            ));
            self.data.insert("extends_add".into(), json!(""));
            ids.push("extends-add".to_owned());
        }
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        self.components.push(Component::column("extends", &refs));
        "extends".to_owned()
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

/// Une section de la surface : son id, son libellé, et ce qu'elle tient.
struct SectionView {
    section: Section,
    id: String,
    summary: String,
}

/// Les sections, dans l'ordre : globales, une par pack de `packs`, variantes d'interface.
fn sections(packs: &[(oku_slides_core::Namespace, String)], l: &Localizer) -> Vec<SectionView> {
    let global = |(section, name): &(Section, &str)| SectionView {
        section: section.clone(),
        id: format!("s-{name}"),
        summary: l.tr(&format!("theme-section-{name}")),
    };
    let packs = packs.iter().map(|(ns, name)| SectionView {
        section: Section::Pack(ns.clone()),
        id: format!("s-pack:{}", ns.as_str()),
        summary: name.clone(),
    });
    SECTIONS_BEFORE.iter().map(global).chain(packs).chain(SECTIONS_AFTER.iter().map(global)).collect()
}

impl Session {
    /// Les jetons sous le brouillon : défauts, puis calque du deck de travail.
    fn draft_lower(&self) -> Result<Map<String, Value>> {
        let mut lower = default_tokens().clone();
        merge(&mut lower, &self.theme_layer(self.doc.sources.deck())?);
        Ok(lower)
    }

    /// Les lignes du brouillon : le schéma composé `schema`, puis les couleurs
    /// libres du brouillon et de `lower` (les jetons sous lui).
    fn theme_rows<'s>(&self, schema: &'s [TokenSpec], lower: &Map<String, Value>) -> Vec<Row<'s>> {
        let (draft, base, l) = (self.edited_tokens(), self.base_tokens(), &self.localizer);
        let row = |path: Vec<String>, spec: &'s TokenSpec| {
            let value = lookup(&draft, &path).or_else(|| lookup(lower, &path)).cloned();
            Row {
                label: token_label(schema, &path, l),
                state: token_state(&path, &draft, &base),
                differs: lookup(&draft, &path) != lookup(&base, &path),
                value,
                spec,
                path,
            }
        };
        let mut rows: Vec<Row<'_>> = schema.iter().map(|s| row(s.path.clone(), s)).collect();
        // Les couleurs libres du brouillon et celles qu'il hérite des thèmes dessous.
        let mut effective = lower.clone();
        merge(&mut effective, &draft);
        let free = effective.get("color").and_then(Value::as_object).into_iter().flatten();
        for name in free.map(|(name, _)| name) {
            let path = vec!["color".to_owned(), name.clone()];
            if let Some(s) = spec(schema, &path).filter(|_| !schema.iter().any(|s| s.path == path)) {
                rows.push(row(path, s));
            }
        }
        rows
    }

    /// The header: the draft's parents and the themes to add.
    fn theme_header(&self, b: &mut Builder, body: &mut Vec<String>) {
        let Mode::Theme { edit, .. } = &self.mode else { return };
        let theme = &edit.draft;
        let l = &self.localizer;
        // À ajouter : les thèmes de la bibliothèque hors de la liste, sauf le sien (un enfant n'en a pas).
        let choices = self
            .theme_list
            .iter()
            .filter(|t| !theme.extends.contains(&t.namespace) && (theme.child() || t.namespace != theme.namespace))
            .map(|t| ChoiceOption { id: t.namespace.to_string(), label: t.name.clone(), hint: None })
            .collect();
        body.push(b.extends(&self.draft_parents(theme), choices, l));
    }

    /// The surface of the draft's tokens (a theme tab); `filter` keeps the
    /// tokens whose translated label or path holds it (case ignored).
    pub(crate) fn host_theme(&self, filter: &str) -> Result<Option<Surface>> {
        let Mode::Theme { edit, .. } = &self.mode else { return Ok(None) };
        let theme = &edit.draft;
        let l = &self.localizer;
        let typed = filter;
        // Normalisé pour la recherche seulement : le champ garde la saisie telle quelle.
        let filter = filter.trim().to_lowercase();
        let palette: Vec<String> = self.appearance.slide.values().cloned().collect();
        let deck = self.doc.sources.deck();
        let schema = self.token_schema(deck);
        let packs: Vec<_> = self.pack_tokens(deck).into_iter().map(|(ns, name, _)| (ns, name)).collect();
        let lower = self.draft_lower()?;
        let mut tokens = lower.clone();
        merge(&mut tokens, &self.edited_tokens());
        let rows: Vec<Row<'_>> = self.theme_rows(&schema, &lower).into_iter().filter(|r| r.matches(&filter)).collect();
        let mut b = Builder::default();
        let mut body = vec!["title".to_owned()];
        b.components.push(Component::text("title", theme.name.clone(), Some(TextVariant::H4)));
        self.theme_header(&mut b, &mut body);
        body.push("filter".to_owned());
        b.components.push(Component::new(
            "filter",
            ComponentKind::TextField {
                label: Dynamic::Literal(l.tr("theme-filter")),
                value: Dynamic::path("/filter"),
                action: Some(Action::event("filter", [("filter", Dynamic::path("/filter"))])),
            },
        ));
        b.data.insert("filter".into(), json!(typed));
        for view in sections(&packs, l) {
            let mut ids: Vec<String> = Vec::new();
            for r in rows.iter().filter(|r| r.spec.section == view.section) {
                ids.push(b.row(r, &palette, &tokens, l));
            }
            if view.section == Section::Colors && filter.is_empty() {
                ids.push(b.new_color(&palette, l));
            }
            if ids.is_empty() {
                continue;
            }
            let rows_id = format!("{}-rows", view.id);
            let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
            b.components.push(Component::column(&rows_id, &refs));
            let open = !filter.is_empty() || view.section == Section::Colors;
            let summary = Dynamic::Literal(view.summary);
            b.components.push(Component::new(
                &view.id,
                ComponentKind::Disclosure { summary, child: rows_id, open: Some(open) },
            ));
            body.push(view.id);
        }
        let refs: Vec<&str> = body.iter().map(String::as_str).collect();
        b.components.insert(0, Component::column("body", &refs));
        b.components.insert(0, Component::card("root", "body"));
        Ok(Some(surface(HOST_THEME, Place::Thread, None, b.components, Value::Object(b.data))))
    }

    /// Acts on the draft's token `path`: one undo step, checked by the schema;
    /// no effect if the token already has the target value.
    pub(crate) fn host_token(&mut self, path: Vec<String>, action: TokenAction, value: Option<Value>) -> Result<()> {
        self.ensure_theme_tab()?;
        let draft = self.edited_tokens();
        let schema = self.token_schema(self.doc.sources.deck());
        let (path, target) = match action {
            TokenAction::Set => {
                let lower = self.draft_lower()?;
                let current = |p: &[String]| lookup(&draft, p).or_else(|| lookup(&lower, p)).cloned();
                let (path, v) = token_value(&schema, path, value.unwrap_or(Value::Null), current)?;
                (path, Some(v))
            }
            TokenAction::Reset => {
                let base = lookup(&self.base_tokens(), &path).cloned();
                (path, base)
            }
            TokenAction::Remove => (path, None),
            TokenAction::Clear => (path, Some(Value::Null)),
        };
        if spec(&schema, &path).is_none() {
            let reason = if matches!(path.as_slice(), [g, _] if g == "color") { "bad-name" } else { "unknown-token" };
            return InvalidTokenValueSnafu { path: path.join("."), reason }.fail()?;
        }
        if lookup(&draft, &path) == target.as_ref() {
            return Ok(());
        }
        let token = token_label(&schema, &path, &self.localizer);
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
