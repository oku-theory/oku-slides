//! Resume of the workspace at launch, and of the files received afterwards.
//!
//! Rereads `workspace.json` and each tab, drops the orphans and the untrusted
//! entries, then rewrites the state at each change.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oku_slides_platform::storage::resolve;
use serde_json::{Value, json};

use crate::Localize;
use crate::agent::bounds::label;
use crate::error::Result;
use crate::files::EXTENSION;
use crate::help::help_locale;
use crate::resume::{Resume, ResumeError};
use crate::tab_document::TabDocument;
use crate::views::TabInfo;
use crate::{Fonts, Platform};

use serde::{Deserialize, Serialize};

use super::import_pick::is_pptx;
use super::{
    Dormant, LAYOUT_VERSION, Layout, MAX_NUMBER, Opening, Slot, TABS_DIR, Tab, TabPlace, WindowLayout, Workspace,
    WorkspaceRequest, is_tab_id, is_window_label, number,
};

/// A window's layout with its accepted tabs and their saved info, before restoring.
type Entry = (WindowLayout, Vec<(String, Option<TabInfo>)>);

/// A tab's saved entry: the [`TabInfo`] plus the legacy keys of the previous release.
///
/// The previous release reads `path`, `deck_uid` and `help` and ignores `document`, so
/// it still reads what this one writes (a pack tab looks like a deck tab with no path).
#[derive(Serialize)]
struct SavedTabInfo<'a> {
    #[serde(flatten)]
    info: &'a TabInfo,
    /// The deck's file; `None` for any other document.
    path: Option<&'a Path>,
    /// The deck's identity; `None` for any other document.
    deck_uid: Option<&'a str>,
    /// The help's locale; `None` for any other document.
    help: Option<&'a str>,
}

impl<'a> SavedTabInfo<'a> {
    /// `info` with the legacy keys derived from its document.
    fn new(info: &'a TabInfo) -> Self {
        let document = &info.document;
        Self { info, path: document.path(), deck_uid: document.deck_uid(), help: document.help_locale() }
    }
}

/// `workspace.json` as written: the structure, and the info of each tab.
#[derive(Serialize)]
struct LayoutFile<'a> {
    #[serde(flatten)]
    layout: &'a Layout,
    /// The next launch's bar, with no session reread.
    info: BTreeMap<&'a str, SavedTabInfo<'a>>,
}

/// `workspace.json` as reread.
///
/// The info stays raw until its tab: a malformed entry costs only itself (the
/// tab is then reread at launch), never the whole workspace.
#[derive(Deserialize)]
struct SavedLayout {
    #[serde(flatten)]
    layout: Layout,
    /// Raw to the end: even something other than an object costs only the info.
    #[serde(default)]
    info: Value,
}

impl SavedLayout {
    /// The saved info of tab `id`, if it reads.
    fn take_info(&mut self, id: &str) -> Option<TabInfo> {
        let raw = self.info.as_object_mut()?.remove(id)?;
        read_info(id, raw)
    }
}

/// A saved entry as a [`TabInfo`], converted from the legacy keys and validated.
///
/// An entry with a `document` ignores the legacy keys; one without builds its
/// document from them. The file is untrusted: what fails validation is dropped
/// (a path, an identity) or costs the entry (an unknown help locale).
fn read_info(id: &str, mut raw: Value) -> Option<TabInfo> {
    let Some(object) = raw.as_object_mut() else {
        tracing::warn!(tab = id, "infos d'onglet illisibles : onglet relu");
        return None;
    };
    let (path, deck_uid, help) = (object.remove("path"), object.remove("deck_uid"), object.remove("help"));
    if !object.contains_key("document") {
        let document = match help {
            Some(Value::String(locale)) => json!({"kind": "help", "locale": locale}),
            None | Some(Value::Null) => json!({
                "kind": "deck",
                "path": path.unwrap_or(Value::Null),
                "deck_uid": deck_uid.unwrap_or(Value::Null),
            }),
            Some(_) => {
                tracing::warn!(tab = id, "infos d'onglet illisibles : onglet relu");
                return None;
            }
        };
        object.insert("document".to_owned(), document);
    }
    let mut info: TabInfo = serde_json::from_value(raw)
        .inspect_err(|e| tracing::warn!(tab = id, error = %e, "infos d'onglet illisibles : onglet relu"))
        .ok()?;
    info.document = match info.document {
        TabDocument::Help { locale } if help_locale(&locale) != locale => {
            tracing::warn!(tab = id, locale = %label(&locale), "langue d'aide inconnue : onglet relu");
            return None;
        }
        TabDocument::Deck { path, deck_uid } => TabDocument::Deck {
            path: path.filter(|p| valid_deck_path(id, p)),
            deck_uid: deck_uid.filter(|uid| valid_deck_uid(id, uid)),
        },
        // Kept even with an invalid owner (not a tab id, its own id): the
        // pre-pass of `restore` sets it aside, never reread as a pack tab.
        other => other,
    };
    info.title = label(&info.title);
    // Only a pack or theme tab names a draft it created: anything else would delete a pack at close.
    info.created =
        info.created.filter(|_| matches!(info.document, TabDocument::Pack { .. } | TabDocument::Theme { .. }));
    Some(info)
}

/// Whether a saved deck file is a normalized absolute deck path (`read_doc`'s rule); a refusal is logged.
fn valid_deck_path(id: &str, path: &Path) -> bool {
    let named = path.file_name().is_some_and(|n| n.to_string_lossy().ends_with(EXTENSION));
    let valid = path.is_absolute() && named && resolve(path) == path;
    if !valid {
        tracing::warn!(tab = id, path = %label(&path.to_string_lossy()), "fichier d'onglet refusé");
    }
    valid
}

/// Whether a saved deck identity is at most 64 characters of `[0-9a-zA-Z-]`; a refusal is logged.
fn valid_deck_uid(id: &str, uid: &str) -> bool {
    let valid = uid.len() <= 64 && uid.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !valid {
        tracing::warn!(tab = id, "identité de deck d'onglet refusée");
    }
    valid
}

impl Workspace {
    /// Resumes the workspace left in `dir` (`<data>/resume`).
    ///
    /// Its windows, tabs and their sessions come back; afterwards everything
    /// is written on each change. Unreadable `workspace.json`: the whole
    /// folder is set aside, never overwritten. An unreadable tab is set aside
    /// alone and removed; the others stay.
    ///
    /// # Errors
    ///
    /// Fails if, with no window resumed, the blank replacement tab cannot be built.
    pub fn resume(fonts: Fonts, platform: &Platform, dir: &Path) -> Result<Self> {
        let mut ws = Self::empty(fonts, platform, Some(dir.to_path_buf()));
        // Avant toute relecture : ce que ce lancement met de côté reste
        // intact jusqu'au suivant, quels que soient le réglage et l'horloge.
        ws.prune_failed(dir);
        let cited = match ws.restore(dir) {
            Ok(cited) => cited,
            Err(e) => {
                tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "reprise de l'espace de travail impossible");
                ws.tabs.clear();
                ws.layout = Layout::new();
                ws.recent.clear();
                match Resume::new(platform.storage.clone(), dir.to_path_buf()).quarantine() {
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
            let pending = ws.pending(None, Opening::Blank, TabPlace::End);
            let session = pending.build()?;
            ws.insert_tab(None, pending.id, session, &TabPlace::End, true);
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
        let mut labels = HashSet::new();
        let mut max_window = 0;
        // Every entry first: a deck-pack tab is restored only once its owner is.
        let mut entries: Vec<Entry> = Vec::new();
        for w in windows {
            // Une fenêtre refusée cite quand même ses onglets : leurs dossiers restent.
            let accepted = is_window_label(&w.label) && labels.insert(w.label.clone());
            if accepted {
                max_window = max_window.max(number(&w.label));
            } else {
                tracing::warn!(label = %w.label, "fenêtre de reprise refusée");
            }
            let mut tabs = Vec::new();
            for id in &w.tabs {
                if !is_tab_id(id) || !cited.insert(id.clone()) {
                    tracing::warn!(tab = %id, "onglet de reprise refusé");
                    continue;
                }
                if accepted {
                    tabs.push((id.clone(), file.take_info(id)));
                }
            }
            entries.push((w, tabs));
        }
        let restored = self.restore_entries(dir, &mut entries);
        for (w, tabs) in entries {
            let tabs: Vec<String> = tabs.into_iter().map(|(id, _)| id).filter(|id| restored.contains(id)).collect();
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

    /// Restores `entries`' tabs: every other tab first, then each deck-pack tab whose owner is a restored deck.
    ///
    /// The pre-pass sets aside a deck-pack entry whose owner is not a
    /// restored tab of kind exactly `Deck`, or that repeats an earlier
    /// `(deck_tab, pack)`: not restored, its folder kept (`kept`), and the
    /// owner, or else the first restored tab, shows `notice-tab-resume-failed`
    /// once it wakes. Returns the restored ids.
    fn restore_entries(&mut self, dir: &Path, entries: &mut [Entry]) -> HashSet<String> {
        let mut restored = HashSet::new();
        let mut deck_packs = Vec::new();
        for (id, info) in entries.iter_mut().flat_map(|(_, tabs)| tabs.iter_mut()) {
            if let Some(TabDocument::DeckPack { deck_tab, pack }) = info.as_ref().map(|i| &i.document) {
                deck_packs.push((id.clone(), deck_tab.clone(), *pack, info.take()));
            } else if self.restore_tab(dir, id, info.take()) {
                restored.insert(id.clone());
            }
        }
        let mut seen = HashSet::new();
        let mut set_aside = Vec::new();
        for (id, deck_tab, pack, info) in deck_packs {
            let owner_is_deck = restored.contains(&deck_tab)
                && self.tabs.get(&deck_tab).is_some_and(|t| matches!(t.info.document, TabDocument::Deck { .. }));
            if !owner_is_deck || !seen.insert((deck_tab.clone(), pack)) {
                tracing::warn!(tab = %id, owner = %label(&deck_tab), "deck-pack tab set aside at resume: no owner deck, or a duplicate");
                set_aside.push(deck_tab);
                continue;
            }
            if self.restore_tab(dir, &id, info) {
                restored.insert(id);
            }
        }
        if let Some(owner) = set_aside.first() {
            let first = entries.iter().flat_map(|(_, tabs)| tabs).map(|(id, _)| id).find(|id| restored.contains(*id));
            let target = Some(owner).filter(|o| restored.contains(*o)).or(first).cloned();
            self.leave_notice(target.as_deref(), "notice-tab-resume-failed");
        }
        restored
    }

    /// Leaves `key`'s notice on dormant tab `tab`, shown once it wakes; a tab already awake is logged.
    fn leave_notice(&mut self, tab: Option<&str>, key: &str) {
        let text = self.platform.localizer.tr(key);
        let Some(t) = tab.and_then(|id| self.tabs.get(id)) else { return };
        // Nothing else holds a slot while the workspace resumes: never waits.
        if let Ok(Slot::Dormant(dormant)) = t.session.try_lock().as_deref_mut() {
            dormant.notice = Some(text);
        } else {
            tracing::warn!(tab, "resume notice not left: the tab is awake");
        }
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
            let (fonts, platform) = (self.fonts.clone(), self.platform.clone());
            let (title, document) = (info.title.clone(), info.document.clone());
            let dormant = Dormant { dir: tab_dir, title, document, notice: None, fonts, platform };
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

    /// Writes `workspace.json`, with each tab's info and its legacy keys; a failure is logged.
    pub(super) fn save_layout(&self) {
        let Some(dir) = &self.dir else { return };
        let file = LayoutFile {
            layout: &self.layout,
            info: self.tabs.iter().map(|(id, t)| (id.as_str(), SavedTabInfo::new(&t.info))).collect(),
        };
        let resume = Resume::new(self.platform.storage.clone(), dir.clone());
        if let Err(e) = resume.write_json("workspace.json", &file) {
            tracing::warn!(error = %oku_slides_platform::errors::one_line(&e), "espace de travail non enregistré");
        }
    }
}

#[cfg(test)]
#[path = "workspace_resume_tests.rs"]
mod workspace_resume_tests;
