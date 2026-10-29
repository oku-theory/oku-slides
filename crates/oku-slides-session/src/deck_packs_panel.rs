//! The A2UI surface of the deck's packs panel, and its events.
//!
//! Built from a view without text ([`DeckPacksView`]) and the host's
//! localizer at each use. Three sections in order: layouts, widgets, theme.
//! Each pinned pack is a card: name, short version, namespace, relation to
//! my version, then its actions, or its removal refusal in place of the
//! button. Each event names its action (`add_pack`, `remove_pack`,
//! `use_mine`, `make_current`, `theme`, `theme_update`, `open_pack`, `open_theme`); its
//! context carries the kind and namespace, which the session checks again.
//! `open_pack` and `open_theme` are no session actions: the interface opens the tab.

use oku_slides_a2ui::{Action, ButtonIcon, ChoiceOption, Component, ComponentKind, Dynamic, TextVariant, Tone};
use oku_slides_core::{Namespace, PackKind};
use oku_slides_i18n::{Localize, Localizer};
use serde_json::{Map, Value, json};

use crate::agent::Surface;
use crate::agent::forms::{Place, lit, surface};
use crate::deck_packs::{DeckPacksView, Drift, PinnedPackView, ThemeSectionView};
use crate::theme_picker::picked_theme;
use crate::{Result, Session};

/// Surface du panneau des packs du deck.
pub const HOST_PACKS: &str = "host-packs";

/// Chiffres hexadécimaux montrés d'une version.
const SHORT_VERSION: usize = 7;

/// Une action du panneau des packs.
#[derive(Debug, Clone, PartialEq)]
pub enum PacksEvent {
    /// Ajouter la version courante d'un pack de la bibliothèque.
    AddPack {
        /// Kind of pack: elements or layouts.
        kind: PackKind,
        /// Namespace of the pack in the library.
        namespace: Namespace,
    },
    /// Retirer un pack du deck.
    RemovePack {
        /// Kind of pack: elements or layouts.
        kind: PackKind,
        /// Namespace of the pack the deck pins.
        namespace: Namespace,
    },
    /// Passer le deck à ma version.
    UseMine {
        /// Kind of pack: elements or layouts.
        kind: PackKind,
        /// Namespace of the pack the deck pins.
        namespace: Namespace,
    },
    /// Faire de la version du deck ma version courante.
    MakeCurrent {
        /// Kind of pack: elements or layouts.
        kind: PackKind,
        /// Namespace of the pack the deck pins.
        namespace: Namespace,
    },
    /// Choisir le thème du deck.
    Theme {
        /// Namespace of the chosen theme.
        theme: Namespace,
    },
    /// Passer le deck à ma version de son thème.
    ThemeUpdate,
}

/// Le nom d'une sorte dans les ids et les contextes.
fn kind_name(kind: PackKind) -> &'static str {
    kind.files_dir()
}

/// La sorte nommée `name` dans un contexte ; `None` pour un autre nom.
fn kind_of(name: &str) -> Option<PackKind> {
    PackKind::ALL.into_iter().find(|k| kind_name(*k) == name)
}

/// L'action du panneau que décrivent un évènement et son contexte ; `None` si mal formée.
#[must_use]
pub fn packs_event(name: &str, context: &Map<String, Value>) -> Option<PacksEvent> {
    let pack = || -> Option<(PackKind, Namespace)> {
        let kind = context.get("kind").and_then(Value::as_str).and_then(kind_of)?;
        let namespace = context.get("namespace").and_then(Value::as_str).and_then(Namespace::parse)?;
        Some((kind, namespace))
    };
    match name {
        "add_pack" => pack().map(|(kind, namespace)| PacksEvent::AddPack { kind, namespace }),
        "remove_pack" => pack().map(|(kind, namespace)| PacksEvent::RemovePack { kind, namespace }),
        "use_mine" => pack().map(|(kind, namespace)| PacksEvent::UseMine { kind, namespace }),
        "make_current" => pack().map(|(kind, namespace)| PacksEvent::MakeCurrent { kind, namespace }),
        "theme" => picked_theme(context).map(|theme| PacksEvent::Theme { theme }),
        "theme_update" => Some(PacksEvent::ThemeUpdate),
        _ => None,
    }
}

/// L'évènement `name` sur le pack `p`.
fn pack_action(name: &str, p: &PinnedPackView) -> Action {
    Action::event(name, [("kind", lit(json!(kind_name(p.kind)))), ("namespace", lit(json!(p.namespace.as_str())))])
}

/// The `open_pack` event for pack `p`.
fn open_action(p: &PinnedPackView) -> Action {
    Action::event("open_pack", [("pack", lit(json!(p.id.to_string())))])
}

/// Les composants d'une surface en construction.
#[derive(Default)]
struct Builder {
    components: Vec<Component>,
}

impl Builder {
    /// Ajoute `c` ; son id.
    fn push(&mut self, c: Component) -> String {
        let id = c.id.clone();
        self.components.push(c);
        id
    }

    /// La carte du pack `p` ; son id.
    ///
    /// Ids `{sorte}/{espace}` et `{sorte}/{espace}/{partie}` : un espace de noms n'a jamais
    /// de `/`, et les ids fixes du panneau n'en ont pas, donc aucun id ne se répète.
    fn card(&mut self, p: &PinnedPackView, l: &Localizer) -> String {
        let id = format!("{}/{}", kind_name(p.kind), p.namespace);
        let sub = |part: &str| format!("{id}/{part}");
        let short: String = p.version.chars().take(SHORT_VERSION).collect();
        let name = self.push(Component::text(sub("name"), p.name.clone(), Some(TextVariant::Body)));
        let badge = ComponentKind::Badge { text: Dynamic::Literal(short), tone: None::<Tone> };
        let version = self.push(Component::new(sub("version"), badge));
        let head = self.push(Component::row(sub("head"), &[&name, &version]));
        let ns = self.push(Component::text(sub("ns"), p.namespace.to_string(), Some(TextVariant::Caption)));
        let mut body = vec![head, ns];
        // Un pack du deck seul n'a pas de relation (`pin_relation`) : ni ligne, ni boutons de version.
        let drift = p.drift;
        if let Some(r) = drift {
            let key = match r {
                Drift::Older => "packs-relation-older",
                Drift::Newer => "packs-relation-newer",
                Drift::Parallel => "packs-relation-parallel",
            };
            body.push(self.push(Component::text(sub("relation"), l.tr(key), Some(TextVariant::Note))));
        }
        let open = Component::button(sub("open"), l.tr("packs-pack-open"), false, open_action(p));
        let mut actions = vec![self.push(open)];
        if drift.is_some() {
            let action = pack_action("use_mine", p);
            actions.push(self.push(Component::button(sub("use-mine"), l.tr("packs-use-mine"), true, action)));
            if p.library_has_pinned && !p.builtin {
                let action = pack_action("make_current", p);
                let label = l.tr("packs-make-current");
                actions.push(self.push(Component::button(sub("make-current"), label, false, action)));
            }
        }
        match &p.remove_refusal {
            None => {
                let remove =
                    Component::button(sub("remove"), l.tr("packs-remove"), false, pack_action("remove_pack", p));
                actions.push(self.push(remove.danger(true)));
            }
            Some(refusal) => {
                let text = refusal.localize(l);
                body.push(self.push(Component::text(sub("refusal"), text, Some(TextVariant::Caption))));
            }
        }
        if !actions.is_empty() {
            let refs: Vec<&str> = actions.iter().map(String::as_str).collect();
            body.push(self.push(Component::row(sub("actions"), &refs)));
        }
        let refs: Vec<&str> = body.iter().map(String::as_str).collect();
        let column = self.push(Component::column(sub("body"), &refs));
        self.push(Component::card(&id, &column))
    }

    /// La section des packs de sorte `kind` : titre, cartes, puis l'ajout depuis la bibliothèque.
    fn section(&mut self, kind: PackKind, view: &DeckPacksView, l: &Localizer) -> String {
        let id = kind_name(kind);
        let (packs, key) = match kind {
            PackKind::Layouts => (&view.layouts, "packs-section-layouts"),
            PackKind::Widgets => (&view.widgets, "packs-section-widgets"),
        };
        let mut children = vec![self.push(Component::text(format!("{id}-title"), l.tr(key), Some(TextVariant::Body)))];
        children.extend(packs.iter().map(|p| self.card(p, l)));
        if let Some(addable) = view.addable.get(&kind).filter(|a| !a.is_empty()) {
            let options = addable
                .iter()
                .map(|(ns, name)| ChoiceOption { id: ns.to_string(), label: name.clone(), hint: None })
                .collect();
            let path = format!("/add/{id}");
            let action =
                Action::event("add_pack", [("kind", lit(json!(id))), ("namespace", Dynamic::path(path.as_str()))]);
            let select = ComponentKind::Select {
                label: Dynamic::Literal(l.tr("packs-add")),
                value: Dynamic::path(path.as_str()),
                options,
                action: Some(action),
            };
            children.push(self.push(Component::new(format!("{id}-add"), select)));
        }
        let refs: Vec<&str> = children.iter().map(String::as_str).collect();
        self.push(Component::column(id, &refs))
    }

    /// The theme section: name, parents, overrides, choice, opening, update.
    fn theme(&mut self, t: &ThemeSectionView, l: &Localizer) -> String {
        let title = l.tr("packs-section-theme");
        let mut children = vec![
            self.push(Component::text("theme-title", title, Some(TextVariant::Body))),
            self.push(Component::text("theme-name", t.name.clone(), Some(TextVariant::Body))),
        ];
        if !t.extends.is_empty() {
            children.push(self.push(Component::text(
                "theme-extends",
                t.extends.join(", "),
                Some(TextVariant::Caption),
            )));
        }
        let overrides = l.tr_with("packs-theme-overrides", &[("count", t.overrides.into())]);
        children.push(self.push(Component::text("theme-overrides", overrides, Some(TextVariant::Caption))));
        let options = t
            .choices
            .iter()
            .map(|(ns, name)| ChoiceOption { id: ns.to_string(), label: name.clone(), hint: None })
            .collect();
        let pick = ComponentKind::Select {
            label: Dynamic::Literal(l.tr("packs-theme-pick")),
            value: Dynamic::path("/theme"),
            options,
            action: Some(Action::event("theme", [("theme", Dynamic::path("/theme"))])),
        };
        children.push(self.push(Component::new("theme-pick", pick)));
        if let Some(id) = t.id {
            let action = Action::event("open_theme", [("theme", lit(json!(id.to_string())))]);
            children.push(self.push(Component::button("theme-open", l.tr("packs-theme-open"), false, action)));
        }
        if t.outdated {
            let update = Component::button("theme-update", l.tr("packs-use-mine"), true, Action::named("theme_update"));
            children.push(self.push(update));
        }
        let refs: Vec<&str> = children.iter().map(String::as_str).collect();
        self.push(Component::column("theme", &refs))
    }
}

/// La surface du panneau, traduite par `l` (celui de l'hôte, à chaque usage).
///
/// Son modèle porte le thème actif (`/theme`), valeur du choix du thème.
pub fn deck_packs_panel(view: &DeckPacksView, l: &Localizer) -> Surface {
    let mut b = Builder::default();
    b.push(Component::text("title", l.tr("packs-title"), Some(TextVariant::H4)));
    b.section(PackKind::Layouts, view, l);
    b.section(PackKind::Widgets, view, l);
    b.theme(&view.theme, l);
    let close = Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss"));
    b.push(close.icon(ButtonIcon::Close));
    let root = Component::column("root", &["title", "layouts", "widgets", "theme", "close"]);
    let components = std::iter::once(root).chain(b.components).collect();
    surface(HOST_PACKS, Place::Thread, None, components, json!({ "theme": view.theme.active.as_str() }))
}

impl Session {
    /// Applies an action of the panel.
    ///
    /// # Errors
    ///
    /// Fails like the applied action (add, remove, version, theme): see each one.
    pub fn packs_action(&mut self, event: PacksEvent) -> Result<()> {
        match event {
            PacksEvent::AddPack { kind, namespace } => self.add_library_pack(kind, &namespace),
            PacksEvent::RemovePack { kind, namespace } => self.remove_deck_pack(kind, &namespace),
            PacksEvent::UseMine { kind, namespace } => self.pack_use_mine(kind, &namespace),
            PacksEvent::MakeCurrent { kind, namespace } => self.pack_make_current(kind, &namespace),
            PacksEvent::Theme { theme } => self.pick_theme(&theme),
            PacksEvent::ThemeUpdate => self.update_deck_theme(),
        }
    }
}

#[cfg(test)]
#[path = "deck_packs_panel_tests.rs"]
mod deck_packs_panel_tests;
