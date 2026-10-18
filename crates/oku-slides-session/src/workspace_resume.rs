//! Reprise de l'espace de travail au lancement, et fichiers reçus ensuite.
//!
//! Relit `workspace.json` et chaque onglet, écarte les orphelins et les
//! entrées non fiables, puis réécrit l'état à chaque changement.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_platform::storage::resolve;

use crate::Localize;
use crate::error::Result;
use crate::resume::{Resume, ResumeError};
use crate::views::TabInfo;
use crate::{Fonts, Platform};

use serde::{Deserialize, Serialize};

use super::import_pick::is_pptx;
use super::{
    Dormant, LAYOUT_VERSION, Layout, MAX_NUMBER, Opening, TABS_DIR, Tab, WindowLayout, Workspace, WorkspaceRequest,
    is_tab_id, is_window_label, number,
};

/// `workspace.json` tel qu'écrit : la structure, et les infos de chaque onglet.
#[derive(Serialize)]
struct LayoutFile<'a> {
    #[serde(flatten)]
    layout: &'a Layout,
    /// La barre du lancement suivant, sans session relue.
    info: BTreeMap<&'a str, &'a TabInfo>,
}

/// `workspace.json` tel que relu.
///
/// Les infos restent brutes jusqu'à leur onglet : une entrée malformée ne
/// coûte que la sienne (l'onglet est alors relu au lancement), jamais tout
/// l'espace de travail.
#[derive(Deserialize)]
struct SavedLayout {
    #[serde(flatten)]
    layout: Layout,
    /// Brut jusqu'au bout : même autre chose qu'un objet ne coûte que les infos.
    #[serde(default)]
    info: serde_json::Value,
}

impl SavedLayout {
    /// Les infos enregistrées de l'onglet `id`, si elles se lisent.
    fn take_info(&mut self, id: &str) -> Option<TabInfo> {
        let raw = self.info.as_object_mut()?.remove(id)?;
        serde_json::from_value(raw)
            .inspect_err(|e| tracing::warn!(tab = id, error = %e, "infos d'onglet illisibles : onglet relu"))
            .ok()
    }
}

impl Workspace {
    /// Reprend l'espace de travail laissé dans `dir` (`<data>/resume`).
    ///
    /// Ses fenêtres, ses onglets et leurs sessions reviennent ; ensuite tout
    /// est écrit à chaque changement. `workspace.json` illisible : tout le
    /// dossier est mis de côté, jamais écrasé. Un onglet illisible est mis de
    /// côté seul et retiré ; les autres restent.
    pub fn resume(fonts: Fonts, platform: &Platform, dir: PathBuf) -> Result<Self> {
        let mut ws = Self::empty(fonts, platform, Some(dir.clone()));
        // Avant toute relecture : ce que ce lancement met de côté reste
        // intact jusqu'au suivant, quels que soient le réglage et l'horloge.
        ws.prune_failed(&dir);
        let cited = match ws.restore(&dir) {
            Ok(cited) => cited,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "reprise de l'espace de travail impossible");
                ws.tabs.clear();
                ws.layout = Layout::new();
                ws.recent.clear();
                match Resume::new(platform.storage.clone(), dir.clone()).quarantine() {
                    Ok(moved) => tracing::warn!(path = %moved.display(), "reprise en échec mise de côté"),
                    Err(e) => {
                        tracing::warn!(
                            error = %oku_slides_platform::errors::one_line(&e),
                            "reprise en échec non déplacée : rien n'est écrit"
                        );
                        ws.dir = None;
                    }
                }
                HashSet::new()
            }
        };
        // Un orphelin qu'on n'a pas pu supprimer garde son numéro : jamais réutilisé.
        let listed = ws.remove_orphans(&cited);
        ws.layout.next_tab = ws.layout.next_tab.max(listed.saturating_add(1)).min(MAX_NUMBER);
        if ws.layout.windows.is_empty() {
            let pending = ws.pending(None, Opening::Blank, None);
            let session = pending.build()?;
            ws.insert_tab(None, pending.id, session, None);
        }
        ws.save_layout();
        Ok(ws)
    }

    /// Relit `workspace.json` et chaque onglet.
    ///
    /// Renvoie les identifiants cités (restaurés ou non), dont les dossiers
    /// ne sont jamais supprimés.
    ///
    /// Le fichier n'est pas digne de confiance : identifiants et labels
    /// validés, doublons refusés, compteurs ramenés entre le plus grand
    /// identifiant relu + 1 et `MAX_NUMBER`.
    fn restore(&mut self, dir: &Path) -> std::result::Result<HashSet<String>, ResumeError> {
        let resume = Resume::new(self.platform.storage.clone(), dir.to_owned());
        let Some(mut file) = resume.read_json::<SavedLayout>("workspace.json", LAYOUT_VERSION)? else {
            return Ok(HashSet::new());
        };
        let windows = std::mem::take(&mut file.layout.windows);
        let mut cited = HashSet::new();
        let mut restored = HashSet::new();
        let mut labels = HashSet::new();
        let mut max_window = 0;
        for w in windows {
            // Une fenêtre refusée cite quand même ses onglets : leurs dossiers restent.
            let accepted = is_window_label(&w.label) && labels.insert(w.label.clone());
            if accepted {
                max_window = max_window.max(number(&w.label));
            } else {
                tracing::warn!(label = %w.label, "fenêtre de reprise refusée");
            }
            let mut tabs = Vec::new();
            for id in w.tabs {
                if !is_tab_id(&id) || !cited.insert(id.clone()) {
                    tracing::warn!(tab = %id, "onglet de reprise refusé");
                    continue;
                }
                if accepted && self.restore_tab(dir, &id, file.take_info(&id)) {
                    restored.insert(id.clone());
                    tabs.push(id);
                }
            }
            if tabs.is_empty() {
                continue;
            }
            let active = if tabs.contains(&w.active) { w.active } else { tabs[0].clone() };
            self.recent.push(w.label.clone());
            self.layout.windows.push(WindowLayout { label: w.label, tabs, active });
        }
        let saved = file.layout;
        self.layout.agent_detached = saved.agent_detached;
        // Les onglets gardés la dernière fois restent cités (après les
        // fenêtres : un identifiant des deux côtés reste restauré).
        cited.extend(saved.kept.iter().filter(|id| is_tab_id(id)).cloned());
        // Cités, non restaurés, dossier encore là : gardés d'un lancement à l'autre.
        let storage = &self.platform.storage;
        let mut kept: Vec<String> = cited
            .iter()
            .filter(|id| !restored.contains(*id))
            .filter(|id| storage.exists(&dir.join(TABS_DIR).join(id.as_str())).unwrap_or(true))
            .cloned()
            .collect();
        kept.sort();
        self.layout.kept = kept;
        let max_tab = cited.iter().map(|id| number(id)).max().unwrap_or(0);
        // Jamais plus de 18 chiffres, même depuis un identifiant maximal relu.
        let counter = |saved: u64, max: u64| {
            let next = if saved > MAX_NUMBER { max + 1 } else { saved.max(max + 1) };
            next.min(MAX_NUMBER)
        };
        self.layout.next_tab = counter(saved.next_tab, max_tab);
        self.layout.next_window = counter(saved.next_window, max_window);
        if self.recent.contains(&saved.focused) {
            self.touch(&saved.focused);
        } else {
            self.layout.focused = self.recent.last().cloned().unwrap_or_default();
        }
        Ok(cited)
    }

    /// Reprend un onglet ; `false` : onglet retiré (sans dossier, ou illisible).
    ///
    /// Avec ses infos enregistrées, l'onglet dort : sa session ne sera relue
    /// qu'à sa première requête, et un état illisible découvert alors ne le
    /// retire plus (voir `workspace_slot.rs`). Sans infos lisibles (fichier
    /// d'avant t-158, ou altéré), l'onglet est relu tout de suite et retiré
    /// s'il est illisible. Son dossier, s'il existe, reste en place.
    fn restore_tab(&mut self, dir: &Path, id: &str, info: Option<TabInfo>) -> bool {
        let tab_dir = dir.join(TABS_DIR).join(id);
        match self.platform.storage.exists(&tab_dir.join("state.json")) {
            Ok(true) => {}
            Ok(false) => {
                tracing::warn!(tab = id, "onglet sans dossier de reprise : retiré");
                return false;
            }
            Err(e) => {
                tracing::warn!(tab = id, error = %oku_slides_platform::errors::one_line(&e), "onglet illisible : retiré, gardé sur le disque");
                return false;
            }
        }
        if let Some(info) = info {
            let dormant = Dormant { dir: tab_dir, fonts: self.fonts.clone(), platform: self.platform.clone() };
            self.tabs.insert(id.to_owned(), Tab::dormant(info, dormant));
            return true;
        }
        let mut session = match super::tab_session(&self.fonts, &self.platform) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(tab = id, error = %e.one_line(), "session de l'onglet");
                return false;
            }
        };
        if !session.resume(tab_dir) {
            return false;
        }
        self.tabs.insert(id.to_owned(), Tab::new(session));
        true
    }

    /// Ne garde que les plus récentes reprises mises de côté (réglage
    /// `resume.keep_failed`) : `resume.failed-*` à côté de `dir`, et
    /// `t-<n>.failed-*` dans `dir/tabs`, tous onglets confondus. Appelée
    /// avant la relecture : jamais sur ce que ce lancement met de côté.
    fn prune_failed(&self, dir: &Path) {
        let (storage, keep) = (&self.platform.storage, self.platform.settings.resume.keep_failed);
        let root = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if let Some(parent) = dir.parent() {
            Resume::prune_failed(storage, parent, |b| b == root, keep);
        }
        Resume::prune_failed(storage, &dir.join(TABS_DIR), is_tab_id, keep);
    }

    /// Supprime les dossiers `t-<n>` que `workspace.json` ne citait pas.
    ///
    /// Ce sont les restes d'une fermeture interrompue. Les dossiers cités,
    /// mis de côté ou étrangers restent. Renvoie le plus grand numéro de
    /// dossier `t-<n>` vu (0 sans dossier).
    fn remove_orphans(&self, cited: &HashSet<String>) -> u64 {
        let Some(dir) = &self.dir else { return 0 };
        let storage = &self.platform.storage;
        let listed = match storage.list_dirs(&dir.join(TABS_DIR)) {
            Ok(listed) => listed,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "dossiers d'onglets");
                return 0;
            }
        };
        let mut highest = 0;
        for path in listed {
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if !is_tab_id(&name) {
                continue;
            }
            highest = highest.max(number(&name));
            if !cited.contains(&name)
                && !self.tabs.contains_key(&name)
                && let Err(e) = storage.delete_dir(&path)
            {
                tracing::warn!(path = %path.display(), error = %oku_slides_platform::errors::one_line(&e), "orphelin");
            }
        }
        highest
    }

    /// Ouvre des fichiers reçus dans la fenêtre active.
    ///
    /// Sources : argument, seconde instance, macOS. Un fichier illisible le
    /// dit dans l'onglet actif (avis montré une fois). Un `.pptx` ne s'ouvre
    /// pas : il devient le choix d'import de la fenêtre active
    /// ([`super::ImportPick`]), dont la page ouvre le dialogue d'import. Fin
    /// déjà engagée (`settle`, `closed`) : s'arrête net, sous le même verrou
    /// qu'elle, jamais après coup.
    pub fn open_files(ws: &Mutex<Self>, files: &[PathBuf]) {
        for path in files {
            let resolved = resolve(path);
            if is_pptx(&resolved) {
                let mut w = Self::lock(ws);
                if w.is_closed() {
                    break;
                }
                w.pick_import(resolved);
                continue;
            }
            let (window, closed, already) = {
                let w = Self::lock(ws);
                (w.focused().to_owned(), w.is_closed(), w.tab_with_path(&resolved).is_some())
            };
            if closed {
                break;
            }
            let open = WorkspaceRequest::OpenTab { window: window.clone(), path: path.clone() };
            let e = match Self::apply(ws, open) {
                // Un nouvel onglet : son deck est peut-être déjà ouvert ailleurs
                // (l'hôte ne peut rien demander, le fichier s'ouvre à part).
                Ok(_) if !already => {
                    Self::note_same_deck(ws, &resolved);
                    continue;
                }
                Ok(_) => continue,
                Err(e) => e,
            };
            tracing::warn!(path = %path.display(), error = %e.one_line(), "ouverture d'un fichier reçu");
            let (text, session) = {
                let w = Self::lock(ws);
                let l = &w.platform.localizer;
                let text = l.tr_with(
                    "notice-open-failed",
                    &[("path", path.display().to_string().into()), ("reason", e.localize(l).into())],
                );
                // Fenêtre disparue pendant l'ouverture (comme dans `apply`) :
                // la fenêtre active maintenant fait foi.
                let active = w
                    .layout
                    .windows
                    .iter()
                    .find(|x| x.label == window)
                    .or_else(|| w.layout.windows.iter().find(|x| x.label == w.layout.focused))
                    .map(|x| x.active.clone());
                if active.is_none() {
                    tracing::warn!(path = %path.display(), "aucune fenêtre pour montrer l'avis d'ouverture");
                }
                (text, active.and_then(|id| Some((w.session(&id).ok()?, id))))
            };
            let Some((session, id)) = session else { continue };
            // L'onglet actif, peut-être endormi : relu ici, il le serait de toute façon.
            Self::post_notice(ws, &id, &session, text);
        }
    }

    /// Écrit `workspace.json`, avec les infos de chaque onglet ; un échec est journalisé.
    pub(super) fn save_layout(&self) {
        let Some(dir) = &self.dir else { return };
        let file =
            LayoutFile { layout: &self.layout, info: self.tabs.iter().map(|(id, t)| (id.as_str(), &t.info)).collect() };
        let resume = Resume::new(self.platform.storage.clone(), dir.clone());
        if let Err(e) = resume.write_json("workspace.json", &file) {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "espace de travail non enregistré");
        }
    }
}
