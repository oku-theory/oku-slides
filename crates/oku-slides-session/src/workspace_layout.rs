//! La disposition des onglets dans les fenêtres : trouver, activer, insérer,
//! retirer et déplacer un onglet ; la fenêtre active suit.
//! Aussi la structure écrite dans `workspace.json` ([`Layout`]) et [`Rect`].

use serde::{Deserialize, Serialize};
use snafu::OptionExt;

use super::{Result, Session, Tab, UnknownTabSnafu, UnknownWindowSnafu, Workspace};

/// Version de `workspace.json`.
pub(super) const LAYOUT_VERSION: u32 = 2;

/// Un rectangle à l'écran, dans les unités de l'hôte (celles du point comparé).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Bord gauche.
    pub x: f64,
    /// Bord haut.
    pub y: f64,
    /// Largeur.
    pub w: f64,
    /// Hauteur.
    pub h: f64,
}

impl Rect {
    /// Le point est dedans (bords gauche et haut compris).
    #[must_use]
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Fenêtres et onglets, tels qu'écrits dans `workspace.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct Layout {
    pub(super) version: u32,
    /// Prochains numéros : jamais réutilisés.
    pub(super) next_tab: u64,
    pub(super) next_window: u64,
    pub(super) focused: String,
    pub(super) windows: Vec<WindowLayout>,
    /// Onglets cités mais non restaurés, dont le dossier existe encore : ils
    /// restent cités, leur dossier n'est jamais supprimé.
    #[serde(default)]
    pub(super) kept: Vec<String>,
    /// L'agent est dans sa propre fenêtre (`agent`), pas dans la colonne de droite.
    #[serde(default)]
    pub(super) agent_detached: bool,
}

/// Where a new tab goes in its window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TabPlace {
    /// In place of this pristine tab, else at the end.
    Replace(String),
    /// Right after this tab, else at the end.
    After(String),
    /// At the end of the window.
    End,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct WindowLayout {
    pub(super) label: String,
    pub(super) tabs: Vec<String>,
    pub(super) active: String,
}

impl Layout {
    pub(super) fn new() -> Self {
        Self {
            version: LAYOUT_VERSION,
            next_tab: 1,
            next_window: 1,
            focused: String::new(),
            windows: Vec::new(),
            kept: Vec::new(),
            agent_detached: false,
        }
    }
}

impl Workspace {
    /// A layout change: an owner ignores the links copied before it.
    pub(super) fn bump_layout(&mut self) {
        self.layout_generation = self.layout_generation.saturating_add(1);
    }

    pub(super) fn window_index(&self, label: &str) -> Result<usize> {
        Ok(self.layout.windows.iter().position(|w| w.label == label).context(UnknownWindowSnafu { label })?)
    }

    pub(super) fn window_of(&self, tab: &str) -> Result<usize> {
        Ok(self
            .layout
            .windows
            .iter()
            .position(|w| w.tabs.iter().any(|t| t == tab))
            .context(UnknownTabSnafu { id: tab })?)
    }

    /// `label` devient la fenêtre la plus récemment active.
    pub(super) fn touch(&mut self, label: &str) {
        self.recent.retain(|l| l != label);
        self.recent.push(label.to_owned());
        label.clone_into(&mut self.layout.focused);
    }

    pub(super) fn forget_window(&mut self, label: &str) {
        self.recent.retain(|l| l != label);
        if self.layout.focused == label {
            self.layout.focused = self.recent.last().cloned().unwrap_or_default();
        }
    }

    pub(super) fn add_window(&mut self) -> String {
        let label = format!("w-{}", self.layout.next_window);
        self.layout.next_window = self.layout.next_window.saturating_add(1);
        self.layout.windows.push(WindowLayout { label: label.clone(), tabs: Vec::new(), active: String::new() });
        self.touch(&label);
        label
    }

    /// Inserts an active tab into a window, at `place`.
    ///
    /// In `window` (in place of a `Replace` tab or after an `After` one when it
    /// is there, else at the end), or in a new window if `window` is absent or unknown.
    pub(super) fn insert_tab(&mut self, window: Option<&str>, id: String, session: Session, place: &TabPlace) {
        // La fenêtre d'abord : un onglet n'existe jamais sans fenêtre.
        let label = match window.filter(|l| self.window_index(l).is_ok()) {
            Some(label) => label.to_owned(),
            None => self.add_window(),
        };
        self.tabs.insert(id.clone(), Tab::new(session));
        self.bump_layout();
        let win = self.layout.windows.iter_mut().find(|w| w.label == label);
        debug_assert!(win.is_some(), "la fenêtre vient d'être trouvée ou créée : elle existe toujours ici");
        if let Some(win) = win {
            let at = match place {
                TabPlace::Replace(r) => win.tabs.iter().position(|t| t == r),
                TabPlace::After(a) => win.tabs.iter().position(|t| t == a).map(|i| i + 1),
                TabPlace::End => None,
            };
            let at = at.unwrap_or(win.tabs.len());
            win.tabs.insert(at, id.clone());
            win.active = id;
        }
        self.touch(&label);
    }

    pub(super) fn activate(&mut self, tab: &str) -> Result<()> {
        let w = self.window_of(tab)?;
        tab.clone_into(&mut self.layout.windows[w].active);
        let label = self.layout.windows[w].label.clone();
        self.touch(&label);
        Ok(())
    }

    /// Retire l'onglet de sa fenêtre.
    ///
    /// Son voisin de droite devient actif, sinon celui de gauche ; une
    /// fenêtre vide est retirée.
    pub(super) fn detach(&mut self, tab: &str) -> Result<()> {
        let w = self.window_of(tab)?;
        self.bump_layout();
        let win = &mut self.layout.windows[w];
        if let Some(i) = win.tabs.iter().position(|t| t == tab) {
            win.tabs.remove(i);
            if win.active == tab {
                win.active = win.tabs.get(i).or_else(|| win.tabs.last()).cloned().unwrap_or_default();
            }
        }
        if win.tabs.is_empty() {
            let label = win.label.clone();
            self.layout.windows.remove(w);
            self.forget_window(&label);
        }
        Ok(())
    }

    pub(super) fn reorder(&mut self, tab: &str, index: usize) -> Result<()> {
        let w = self.window_of(tab)?;
        self.bump_layout();
        let tabs = &mut self.layout.windows[w].tabs;
        let from = tabs.iter().position(|t| t == tab).context(UnknownTabSnafu { id: tab })?;
        let id = tabs.remove(from);
        tabs.insert(index.min(tabs.len()), id);
        Ok(())
    }

    /// Déplace un onglet vers `window`, ou une nouvelle fenêtre ; il y devient actif.
    pub(super) fn move_tab(&mut self, tab: &str, window: Option<&str>, index: Option<usize>) -> Result<()> {
        let from = self.window_of(tab)?;
        self.bump_layout();
        if window == Some(self.layout.windows[from].label.as_str()) {
            // Même fenêtre : un index réordonne ; sans index, rien ne change.
            if let Some(index) = index {
                self.reorder(tab, index)?;
            }
            return self.activate(tab);
        }
        let dest = match window {
            Some(label) => {
                self.window_index(label)?;
                label.to_owned()
            }
            None => self.add_window(),
        };
        self.detach(tab)?;
        let d = self.window_index(&dest)?;
        let win = &mut self.layout.windows[d];
        let at = index.unwrap_or(win.tabs.len()).min(win.tabs.len());
        win.tabs.insert(at, tab.to_owned());
        tab.clone_into(&mut win.active);
        self.touch(&dest);
        Ok(())
    }

    /// La fenêtre sous le point `(x, y)` : la plus récemment active qui le contient.
    pub fn window_at(&self, x: f64, y: f64, rects: &[(String, Rect)]) -> Option<String> {
        self.recent.iter().rev().find(|label| rects.iter().any(|(l, r)| l == *label && r.contains(x, y))).cloned()
    }

    /// `tab` est-il l'onglet affiché de sa fenêtre ?
    pub fn tab_shown(&self, tab: &str) -> bool {
        self.layout.windows.iter().any(|w| w.active == tab)
    }

    /// Label de la fenêtre qui porte l'onglet `tab`.
    pub fn tab_window(&self, tab: &str) -> Option<&str> {
        self.layout.windows.iter().find(|w| w.tabs.iter().any(|t| t == tab)).map(|w| w.label.as_str())
    }

    /// `label` est-elle la seule fenêtre de l'espace de travail ?
    ///
    /// Prédicat unique, utilisé par [`Workspace::apply`] (`CloseWindow`, sous
    /// le verrou : la décision est atomique avec la suppression) et par
    /// l'hôte (`close_requested`, en aperçu hors verrou, avant de prévenir la
    /// page) : les deux points d'appel voient la même règle.
    pub fn is_only_window(&self, label: &str) -> bool {
        self.layout.windows.len() <= 1 && self.layout.windows.first().is_some_and(|w| w.label == label)
    }
}
