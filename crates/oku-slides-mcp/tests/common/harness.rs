//! Harness variants over special hosts: closed, gated or hidden tabs, a file open elsewhere.

use super::*;

/// A `Sessions` that always refuses the tab: a real workspace, asked about
/// a tab it does not have (closed on the host side).
#[derive(Debug)]
pub struct ClosedTabs(Mutex<oku_slides_session::Workspace>);

impl Sessions for ClosedTabs {
    /// Always-closed tab: `f` is never called.
    fn with(&self, _tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        self.0.with("onglet-ferme", f)
    }

    /// Always-closed tab: refused (`UnknownTab`).
    fn open_pack(
        &self,
        _tab: &str,
        pack: uuid::Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.0.open_pack("onglet-ferme", pack, widget, gate)
    }

    /// Always-closed tab: refused (`UnknownTab`).
    fn open_theme(
        &self,
        _tab: &str,
        theme: uuid::Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.0.open_theme("onglet-ferme", theme, gate)
    }

    /// Always-closed tab: refused (`UnknownTab`).
    fn document_kind(&self, _tab: &str) -> oku_slides_session::Result<oku_slides_session::DocumentKind> {
        self.0.document_kind("onglet-ferme")
    }

    fn usage_owner(&self, _tab: &str) -> Option<String> {
        self.0.usage_owner("onglet-ferme")
    }
}

/// A `Sessions` where a single file, `open`, is open in another tab.
#[derive(Debug)]
pub struct OneFileElsewhere {
    session: Arc<Mutex<Session>>,
    open: std::path::PathBuf,
}

impl Sessions for OneFileElsewhere {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        self.session.with(tab, f)
    }

    fn open_elsewhere(&self, _tab: &str, path: &std::path::Path) -> bool {
        path == self.open
    }

    fn open_pack(
        &self,
        tab: &str,
        pack: uuid::Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.session.open_pack(tab, pack, widget, gate)
    }

    fn open_theme(
        &self,
        tab: &str,
        theme: uuid::Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.session.open_theme(tab, theme, gate)
    }

    fn document_kind(&self, tab: &str) -> oku_slides_session::Result<oku_slides_session::DocumentKind> {
        self.session.document_kind(tab)
    }

    fn usage_owner(&self, tab: &str) -> Option<String> {
        self.session.usage_owner(tab)
    }
}

/// A server where only `open` is open in another tab (see [`OneFileElsewhere`]).
pub async fn start_with_one_file_elsewhere(open: std::path::PathBuf) -> Harness {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), Arc::new(OneFileElsewhere { session: session.clone(), open }), session, "t-1").await
}

/// A `Sessions` whose tab is behind another in its window: it is not shown.
#[derive(Debug)]
pub struct HiddenTab(Arc<Mutex<Session>>);

impl Sessions for HiddenTab {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        self.0.with(tab, f)
    }

    fn shown(&self, _tab: &str) -> bool {
        false
    }

    fn open_pack(
        &self,
        tab: &str,
        pack: uuid::Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.0.open_pack(tab, pack, widget, gate)
    }

    fn open_theme(
        &self,
        tab: &str,
        theme: uuid::Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.0.open_theme(tab, theme, gate)
    }

    fn document_kind(&self, tab: &str) -> oku_slides_session::Result<oku_slides_session::DocumentKind> {
        self.0.document_kind(tab)
    }

    fn usage_owner(&self, tab: &str) -> Option<String> {
        self.0.usage_owner(tab)
    }
}

/// A server for a hidden tab (see [`HiddenTab`]).
pub async fn start_hidden() -> Harness {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    start_with((platform, data), Arc::new(HiddenTab(session.clone())), session, "t-1").await
}

/// A `Sessions` holding the next session access until the test releases it
/// (see [`Gate`]): a tool call then lasts while turns change.
#[derive(Debug)]
pub struct GatedTabs {
    session: Arc<Mutex<Session>>,
    hold: Mutex<Option<Hold>>,
}

#[derive(Debug)]
struct Hold {
    entered: tokio::sync::oneshot::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
}

/// The test side of a held access: `entered` waits for it, `release` lets it through.
#[derive(Debug)]
pub struct Gate {
    entered: tokio::sync::oneshot::Receiver<()>,
    release: std::sync::mpsc::Sender<()>,
}

impl Gate {
    /// Waits until the held access began (so the tool is already called).
    pub async fn entered(&mut self) {
        tokio::time::timeout(std::time::Duration::from_secs(10), &mut self.entered)
            .await
            .unwrap_or_else(|_| panic!("no session access"))
            .unwrap_or_else(|e| panic!("held access lost: {e}"));
    }

    /// Lets the held access through.
    pub fn release(self) {
        let _ = self.release.send(());
    }
}

impl GatedTabs {
    /// Holds the next session access.
    pub fn hold_next(&self) -> Gate {
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        *self.hold.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
            Some(Hold { entered: entered_tx, release: release_rx });
        Gate { entered, release }
    }
}

impl Sessions for GatedTabs {
    fn with(&self, tab: &str, f: &mut dyn FnMut(&mut Session)) -> oku_slides_session::Result<()> {
        let hold = self.hold.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
        if let Some(hold) = hold {
            let _ = hold.entered.send(());
            let _ = hold.release.recv();
        }
        self.session.with(tab, f)
    }

    /// Applies the hold, then opens in the inner session.
    fn open_pack(
        &self,
        tab: &str,
        pack: uuid::Uuid,
        widget: Option<&str>,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        let hold = self.hold.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
        if let Some(hold) = hold {
            let _ = hold.entered.send(());
            let _ = hold.release.recv();
        }
        self.session.open_pack(tab, pack, widget, gate)
    }

    fn open_theme(
        &self,
        tab: &str,
        theme: uuid::Uuid,
        gate: &mut dyn FnMut(TabGate) -> bool,
    ) -> oku_slides_session::Result<TabOpen> {
        self.session.open_theme(tab, theme, gate)
    }

    fn document_kind(&self, tab: &str) -> oku_slides_session::Result<oku_slides_session::DocumentKind> {
        self.session.document_kind(tab)
    }

    fn usage_owner(&self, tab: &str) -> Option<String> {
        self.session.usage_owner(tab)
    }
}

/// A server whose session accesses can be held (see [`GatedTabs`]).
pub async fn start_gated() -> (Harness, Arc<GatedTabs>) {
    let (platform, session, data) = prepared().await;
    let session = Arc::new(Mutex::new(session));
    let tabs = Arc::new(GatedTabs { session: session.clone(), hold: Mutex::default() });
    let h = start_with((platform, data), tabs.clone(), session, "t-1").await;
    (h, tabs)
}

/// A server whose tab is closed: every session tool fails.
pub async fn start_closed() -> Harness {
    let (platform, session, data) = prepared().await;
    let (platform, workspace) = tokio::task::spawn_blocking(move || {
        let workspace = oku_slides_session::Workspace::new(Fonts::default(), &platform, None)
            .unwrap_or_else(|e| panic!("espace de travail de test : {e}"));
        (platform, workspace)
    })
    .await
    .unwrap_or_else(|e| panic!("tâche bloquante de l'espace de travail : {e}"));
    let session = Arc::new(Mutex::new(session));
    let sessions: Arc<dyn Sessions> = Arc::new(ClosedTabs(Mutex::new(workspace)));
    start_with((platform, data), sessions, session, "t-1").await
}
