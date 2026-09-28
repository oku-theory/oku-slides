//! La disposition des onglets dans les fenêtres : trouver, activer, insérer,
//! retirer et déplacer un onglet ; la fenêtre active suit.

use snafu::OptionExt;

use super::{Result, Session, Tab, UnknownTabSnafu, UnknownWindowSnafu, WindowLayout, Workspace};

impl Workspace {
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
        self.layout.focused = label.to_owned();
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

    /// Insère un onglet actif dans une fenêtre.
    ///
    /// Dans `window` (à la place de `beside` s'il y est, sinon à la fin),
    /// ou dans une nouvelle fenêtre si `window` est absente ou inconnue.
    pub(super) fn insert_tab(&mut self, window: Option<&str>, id: String, session: Session, beside: Option<&str>) {
        // La fenêtre d'abord : un onglet n'existe jamais sans fenêtre.
        let label = match window.filter(|l| self.window_index(l).is_ok()) {
            Some(label) => label.to_owned(),
            None => self.add_window(),
        };
        self.tabs.insert(id.clone(), Tab::new(session));
        let win = self.layout.windows.iter_mut().find(|w| w.label == label);
        debug_assert!(win.is_some(), "la fenêtre vient d'être trouvée ou créée : elle existe toujours ici");
        if let Some(win) = win {
            let at = beside.and_then(|b| win.tabs.iter().position(|t| t == b)).unwrap_or(win.tabs.len());
            win.tabs.insert(at, id.clone());
            win.active = id;
        }
        self.touch(&label);
    }

    pub(super) fn activate(&mut self, tab: &str) -> Result<()> {
        let w = self.window_of(tab)?;
        self.layout.windows[w].active = tab.to_owned();
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
        let tabs = &mut self.layout.windows[w].tabs;
        let from = tabs.iter().position(|t| t == tab).context(UnknownTabSnafu { id: tab })?;
        let id = tabs.remove(from);
        tabs.insert(index.min(tabs.len()), id);
        Ok(())
    }

    /// Déplace un onglet vers `window`, ou une nouvelle fenêtre ; il y devient actif.
    pub(super) fn move_tab(&mut self, tab: &str, window: Option<&str>, index: Option<usize>) -> Result<()> {
        let from = self.window_of(tab)?;
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
        win.active = tab.to_owned();
        self.touch(&dest);
        Ok(())
    }
}
