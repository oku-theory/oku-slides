//! Les onglets démarrés et fermés de la pile, sous un seul verrou.

use std::collections::{HashMap, HashSet};

use crate::tab_entry::{TabEntry, TabLink};

/// Les onglets démarrés et fermés, sous un seul verrou : un onglet ne
/// démarre jamais après sa fermeture, même si les deux se croisent.
#[derive(Debug, Default)]
pub(crate) struct Table {
    /// Les onglets démarrés.
    pub tabs: HashMap<String, TabEntry>,
    /// Onglets fermés : leurs identifiants ne reviennent jamais, une
    /// commande tardive pour l'un d'eux est ignorée (sinon elle relancerait
    /// un agent que plus rien n'arrêterait).
    pub closed: HashSet<String>,
    /// Fin de l'app (`Stack::shutdown`) : plus aucun onglet ne démarre.
    pub shut: bool,
}

impl Table {
    /// L'onglet `tab` ne doit plus démarrer : fermé, ou l'app finie.
    pub(crate) fn refuses(&self, tab: &str) -> bool {
        self.shut || self.closed.contains(tab)
    }

    /// Les onglets démarrés et ce que la pile en utilise, copiés : le verrou n'est pas gardé.
    pub(crate) fn links(&self) -> Vec<(String, TabLink)> {
        self.tabs.iter().map(|(tab, e)| (tab.clone(), e.link.clone())).collect()
    }
}
