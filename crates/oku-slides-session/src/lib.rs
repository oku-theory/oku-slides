//! Session d'édition et espace de travail : deck, historique, rendu et
//! fichier de chaque onglet.
//!
//! - [`Session`] et [`Request`] : un deck (une session par onglet).
//! - [`Workspace`], [`WorkspaceRequest`], [`WorkspaceView`], [`TabInfo`] :
//!   les fenêtres, les onglets, la reprise de l'ensemble.
//!
//! L'agent atteint la session d'un onglet par [`agent::TabSession`], fournie
//! par l'hôte via [`agent::Sessions`] (voir [`Workspace::with_session`]).
//!
//! L'app Tauri et le serveur de développement exposent la même API. Ordre
//! des verrous : l'espace de travail puis une session, jamais l'inverse ;
//! cinq fonctions s'en chargent, en séquence, `Workspace::apply`, `request`,
//! `open_files`, `settle` et `post_notice`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_chart::ChartLabels;
use oku_slides_core::{
    Deck, DeckParams, Element, Op, PackRef, Paragraph, ParamValue, Slide, SourceBundle, SourceLimits, WidgetRegistry,
};
pub use oku_slides_i18n::{Localize, Localizer};
pub use oku_slides_platform::Platform;
pub use oku_slides_platform::config::Settings;
pub use oku_slides_platform::storage::Storage;
pub use oku_slides_render::Fonts;
use oku_slides_render::{PackWidgets, RenderError, RenderErrorKind};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot, RenderWorker};

pub mod agent;
mod appearance;
mod charts;
mod deck_file;
mod deck_pack;
mod deck_widgets;
mod error;
mod fields;
mod files;
mod history;
mod inline;
#[cfg(test)]
pub(crate) mod lib_tests_support;
pub mod library;
mod merge;
pub mod pack;
mod pack_notice;
mod panel;
mod render_fonts;
mod request;
mod resume;
mod resume_writer;
mod templates_mode;
#[cfg(test)]
pub(crate) mod test_support;
mod text;
mod views;
mod widget_preview;
mod widgets_deck;
mod widgets_edit;
mod workspace;

pub use appearance::{Appearance, appearance};
pub use deck_pack::adopt_default_pack;
use error::{
    BuiltinTemplateSetSnafu, ChartSnafu, EditingTemplatesSnafu, InHistorySnafu, NoChartSnafu, NoSlideSnafu,
    NotRenderedSnafu, PdfSnafu, RegistrySnafu, RenderSnafu, ResponseSnafu, SourcesSnafu, WriteSnafu,
};
pub use error::{Result, SessionError, SessionErrorKind};
pub use fields::{FieldKind, FieldView, OwnerKind};
pub use files::{EXTENSION, Opened, deck_uid_of, read_document};
pub use library::{Library, StoredVersion, builtin_versions};
pub use merge::Saved;
pub use pack::{DEFAULT_PACK, GRID_PACK, PackInfo, PackManifest, Relation, Template};
pub use pack_notice::PackNoticeView;
pub use panel::PANEL_SURFACE;
pub use request::Request;
pub use text::{interp, paragraphs_to_text};
pub use views::{
    HistoryView, PageHash, SlideSummary, SlideView, Snapshot, TabInfo, Target, TemplateMode, TemplateView,
    TemplatesView, UiState,
};
pub use widget_preview::{PreviewFormat, WidgetPreview};
pub use widgets_edit::{WidgetChange, WidgetDiagnostic, WidgetStatus};
pub use workspace::{Rect, TabView, WindowView, Workspace, WorkspaceOutcome, WorkspaceRequest, WorkspaceView};

/// Un document ouvert : ses sources (deck et son historique, assets,
/// polices) et son fichier.
struct Document {
    sources: SourceBundle,
    path: Option<PathBuf>,
    dirty: bool,
    /// Signalé à l'ouverture : PDF modifié par un autre outil, parties illisibles.
    notice: Option<String>,
    /// Empreinte du fichier à l'ouverture ou au dernier enregistrement : la
    /// reprise voit ainsi s'il a changé depuis.
    saved_hash: Option<String>,
    /// Deck neuf de la session, jamais remplacé : l'onglet est vierge tant
    /// que rien n'y est fait.
    blank: bool,
    /// Le dernier enregistrement a été refusé : le fichier est devenu un
    /// autre deck. Signalé une fois, avec le prochain snapshot.
    disk_replaced: bool,
}

impl Document {
    /// `defaults` : étapes d'annulation gardées et auteur (réglages
    /// `history.limit` et `[author]`).
    fn new(mut sources: SourceBundle, path: Option<PathBuf>, defaults: &DocDefaults) -> Self {
        sources.set_undo_limit(defaults.undo_limit);
        sources.set_author(defaults.author.clone());
        Self { sources, path, dirty: false, notice: None, saved_hash: None, blank: false, disk_replaced: false }
    }
}

/// Réglages appliqués à chaque document ouvert par la session
/// (`history.limit`, `[author]`).
#[derive(Debug, Clone)]
struct DocDefaults {
    /// Étapes d'annulation gardées.
    undo_limit: usize,
    /// Auteur inscrit dans l'historique du deck ; absent = anonyme.
    author: Option<String>,
}

/// Ce que la session édite : le deck, ou un ensemble de modèles pendant que
/// le deck attend de côté ; ou ce qu'elle montre : une version du deck, en
/// lecture seule, le deck de côté aussi.
pub(crate) enum Mode {
    Deck,
    Templates {
        /// Le deck mis de côté, retrouvé tel quel à la fin.
        stash: Box<Document>,
        edit: TemplateMode,
    },
    History {
        /// Le deck mis de côté ; ses images et polices sont prêtées à l'aperçu.
        stash: Box<Document>,
        history: history::HistoryMode,
    },
}

// ---- Session ----------------------------------------------------------------------

pub struct Session {
    doc: Document,
    mode: Mode,
    library: Library,
    /// Où s'enregistre un deck sans fichier ; `None` : il faut choisir (CLI, tests).
    decks_dir: Option<PathBuf>,
    storage: Storage,
    localizer: Arc<Localizer>,
    /// En-têtes des graphiques reçus, dans la langue de l'utilisateur.
    chart_labels: ChartLabels,
    /// Réglages appliqués à chaque document ouvert (`history.limit`, `[author]`).
    doc_defaults: DocDefaults,
    /// Plafonds des sources lues dans un PDF ou dans la reprise (`[sources]`).
    source_limits: SourceLimits,
    /// Processus de rendu partagé par les rendus de la session : sondes des
    /// packs, packs fautifs.
    worker: RenderWorker,
    /// Registre des widgets du document ouvert : les ops et le rendu s'en
    /// servent (voir `deck_widgets` pour l'invariant).
    registry: Arc<WidgetRegistry>,
    /// Pack dont les widgets surchargent la base ; `None` : la base seule.
    pack: Option<PackWidgets>,
    /// Référence de pack (`deck.templates`) pour laquelle `registry` et
    /// `pack` ont été résolus.
    widgets_templates: Option<PackRef>,
    /// Rendu des aperçus de modèles, séparé du deck, avec leur clé de cache.
    template_renderer: Option<RenderSlot>,
    template_cache: Option<templates_mode::Previews>,
    /// Rendu des variantes proposées par l'agent, séparé du deck.
    variant_renderer: Option<RenderSlot>,
    renderer: RenderSlot,
    /// Polices livrées avec l'app, et celles du rendu (app + deck).
    app_fonts: Fonts,
    render_fonts: Fonts,
    /// Nombre de polices du deck dans `render_fonts` : s'il diffère de celui
    /// des sources, le rendu est à refaire (même après un échec).
    render_deck_fonts: usize,
    /// Apparence tirée du dernier rendu.
    appearance: Appearance,
    compiled: Option<RemoteCompiled>,
    svg_cache: HashMap<u128, Arc<str>>,
    /// Slide courante et grille de l'interface, gardées pour la reprise.
    ui: UiState,
    /// Reprise activée (app de bureau) : l'état est écrit après chaque
    /// commande, par un fil dédié (`resume_writer`).
    resume: Option<resume_writer::Writer>,
    /// Images et polices déjà envoyées au fil de reprise : partagées tant
    /// qu'elles ne changent pas (voir `resume::DocSnapshot`).
    sent_blobs: Vec<Arc<resume::Blobs>>,
    /// Ce que l'utilisateur a fait sans l'agent depuis son dernier tour.
    agent_notes: Vec<String>,
    /// Bandeau du pack du deck ouvert (autre version que la courante).
    pack_notice: Option<pack_notice::PackNotice>,
    /// Bandeaux fermés : ils ne reviennent plus dans cette session.
    pack_notices_closed: HashSet<PackRef>,
    /// Bornes des fichiers de widgets (`[render]`).
    widget_limits: widgets_edit::WidgetLimits,
    /// Une écriture de widgets est en cours : l'avis des widgets écartés se tait.
    widget_edit: bool,
    /// Séance de modification des widgets en mode deck.
    widget_session: Option<widgets_deck::WidgetSession>,
    /// Emplacement d'essai de l'aperçu des widgets.
    widget_preview: Option<widget_preview::PreviewSlot>,
    #[cfg(test)]
    failpoints: deck_widgets::Failpoints,
}

/// Un emplacement de `worker` avec les widgets de `pack` (de registre
/// `registry`), dont l'image absente se dit dans la langue de l'utilisateur.
///
/// Un pack fautif n'est pas posé : la base reste. À l'appelant de s'être
/// replié d'abord (`Session::usable_pack`), pour que la session suive.
pub(crate) fn new_renderer(
    worker: &RenderWorker,
    fonts: Fonts,
    l: &Localizer,
    pack: Option<&PackWidgets>,
    registry: &Arc<WidgetRegistry>,
) -> Result<RenderSlot> {
    let base = oku_slides_render::base_registry(&fonts).context(RegistrySnafu)?;
    let mut slot = worker.slot(&fonts, l, base).context(RenderSnafu)?;
    let pack = pack.filter(|p| !worker.is_faulty(p));
    if pack.is_some() {
        slot.set_widgets(pack, registry.clone()).context(RenderSnafu)?;
    }
    Ok(slot)
}

/// Deck vide : une couverture (`title`) et une slide de contenu (`content`)
/// avec un `text`, dans la langue de l'utilisateur.
pub fn blank_deck(l: &Localizer) -> Deck {
    let mut content = Slide::new("s-1", "content");
    content.params.set("title", ParamValue::Text(l.tr("deck-new-slide-title")));
    let text = Element::text("b-1", vec![Paragraph::plain(l.tr("deck-new-slide-text"))]);
    content.params.set("blocks", ParamValue::Slot(vec![text]));
    Deck {
        version: oku_slides_core::DECK_VERSION,
        theme: Some("papier".into()),
        params: DeckParams {
            title: Some(l.tr("deck-new-title")),
            // Code de langue seul (`zh-Hant` → `zh`) : Typst en déduit la
            // direction du texte (droite à gauche en arabe) et les césures.
            lang: l.locale().split('-').next().map(str::to_owned),
            ..Default::default()
        },
        vars: BTreeMap::new(),
        tokens: Value::Null,
        templates: None,
        slides: vec![Slide::new("s-titre", "title"), content],
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("path", &self.doc.path)
            .field("slides", &self.doc.sources.deck().slides.len())
            .field("dirty", &self.doc.dirty)
            .field("editing", &self.editing())
            .finish_non_exhaustive()
    }
}

impl Session {
    /// `app_fonts` : polices livrées avec l'app, toujours disponibles ;
    /// `platform` : réglages (historique, dossier des modèles), fichiers et
    /// langue.
    pub fn new(app_fonts: Fonts, platform: &Platform) -> Result<Self> {
        let (settings, storage, l) = (&platform.settings, platform.storage.clone(), &platform.localizer);
        let doc_defaults = DocDefaults { undo_limit: settings.history.limit, author: settings.author.resolved() };
        let worker = RenderWorker::new(platform.render_launch.clone(), settings.render.clone());
        let registry = oku_slides_render::base_registry(&app_fonts).context(RegistrySnafu)?;
        let library = Library::new(Some(settings.paths.templates()), storage.clone());
        let mut blank = SourceBundle::new(blank_deck(l), registry.clone()).context(SourcesSnafu)?;
        deck_pack::adopt_default_pack(&library, &mut blank)?;
        let mut s = Self {
            doc: Document { blank: true, ..Document::new(blank, None, &doc_defaults) },
            mode: Mode::Deck,
            library,
            decks_dir: None,
            storage,
            doc_defaults,
            source_limits: files::source_limits(settings),
            renderer: new_renderer(&worker, app_fonts.clone(), l, None, &registry)?,
            worker,
            registry,
            pack: None,
            widgets_templates: None,
            localizer: l.clone(),
            chart_labels: ChartLabels::new(l),
            render_fonts: app_fonts.clone(),
            render_deck_fonts: 0,
            appearance: Appearance::default(),
            template_renderer: None,
            template_cache: None,
            variant_renderer: None,
            app_fonts,
            compiled: None,
            svg_cache: HashMap::new(),
            ui: UiState::default(),
            resume: None,
            agent_notes: Vec::new(),
            pack_notice: None,
            pack_notices_closed: HashSet::new(),
            widget_limits: widgets_edit::WidgetLimits::of(&settings.render),
            widget_edit: false,
            widget_session: None,
            widget_preview: None,
            sent_blobs: Vec::new(),
            #[cfg(test)]
            failpoints: Default::default(),
        };
        // Le pack par défaut de l'utilisateur peut avoir des widgets.
        s.sync_widgets()?;
        s.rerender()?;
        Ok(s)
    }

    pub fn deck(&self) -> &Deck {
        self.doc.sources.deck()
    }

    /// Sources du document courant (tests seulement : réplica Loro, auteurs…).
    #[doc(hidden)]
    pub fn sources_for_tests(&self) -> &SourceBundle {
        &self.doc.sources
    }

    /// Le deck de l'utilisateur, même pendant l'édition des modèles.
    fn user_deck(&self) -> &Deck {
        match &self.mode {
            Mode::Deck => self.doc.sources.deck(),
            Mode::Templates { stash, .. } | Mode::History { stash, .. } => stash.sources.deck(),
        }
    }

    /// Les sources du deck de l'utilisateur, même pendant l'édition des modèles.
    fn user_sources(&self) -> &SourceBundle {
        match &self.mode {
            Mode::Deck => &self.doc.sources,
            Mode::Templates { stash, .. } | Mode::History { stash, .. } => &stash.sources,
        }
    }

    fn editing(&self) -> Option<&TemplateMode> {
        match &self.mode {
            Mode::Deck | Mode::History { .. } => None,
            Mode::Templates { edit, .. } => Some(edit),
        }
    }

    /// Travail non enregistré : le deck, ou l'ensemble de modèles en cours
    /// (pas l'aperçu de l'historique).
    fn unsaved(&self) -> bool {
        match &self.mode {
            Mode::Deck => self.doc.dirty,
            Mode::Templates { stash, .. } => self.doc.dirty || stash.dirty,
            Mode::History { stash, .. } => stash.dirty,
        }
    }

    /// Nom d'un document : son fichier, sinon le titre de son deck.
    fn document_name(&self, doc: &Document) -> String {
        let deck = doc.sources.deck();
        doc.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().trim_end_matches(EXTENSION).to_owned())
            .or_else(|| deck.params.title.as_deref().map(|t| interp(t, &deck.vars)))
            .unwrap_or_else(|| self.localizer.tr("deck-untitled"))
    }

    /// Ce que la barre d'onglets montre de cette session.
    ///
    /// Pendant l'édition des modèles ou l'historique, le titre et le fichier
    /// restent ceux du deck mis de côté.
    pub fn tab_info(&self) -> TabInfo {
        let deck = match &self.mode {
            Mode::Deck => &self.doc,
            Mode::Templates { stash, .. } | Mode::History { stash, .. } => stash,
        };
        TabInfo {
            title: self.document_name(deck),
            dirty: self.unsaved(),
            editing_templates: self.editing().is_some(),
            path: deck.path.as_deref().map(oku_slides_platform::storage::resolve),
            pristine: matches!(self.mode, Mode::Deck)
                && deck.blank
                && deck.path.is_none()
                && !deck.dirty
                && !deck.sources.replica().can_undo()
                && !deck.sources.replica().can_redo(),
            deck_uid: Some(deck.sources.replica().deck_uid()),
        }
    }

    /// Un avis à montrer une fois, avec le prochain snapshot ; ajouté après
    /// celui qui attend encore, une ligne vide entre les deux.
    pub fn set_notice(&mut self, text: String) {
        self.doc.notice = Some(match self.doc.notice.take() {
            Some(pending) => format!("{pending}\n\n{text}"),
            None => text,
        });
    }

    /// Plus rien n'est écrit pour la reprise (l'onglet est fermé) : ce qui
    /// était en cours est écrit d'abord, avant de rendre la main.
    pub(crate) fn stop_resume(&mut self) {
        self.resume = None;
        self.sent_blobs.clear();
    }

    /// Attend que l'état de reprise envoyé soit écrit (avant de quitter).
    pub(crate) fn flush_resume(&self) {
        if let Some(writer) = &self.resume {
            writer.flush();
        }
    }

    /// Dossier des packs de modèles de l'utilisateur.
    pub fn set_templates_dir(&mut self, dir: impl Into<PathBuf>) {
        self.library = Library::new(Some(dir.into()), self.storage.clone());
        self.template_cache = None;
    }

    /// Dossier où s'enregistre de lui-même un deck qui n'a pas de fichier.
    pub fn set_decks_dir(&mut self, dir: impl Into<PathBuf>) {
        self.decks_dir = Some(dir.into());
    }

    /// Traite une requête de l'interface et renvoie la réponse, en JSON.
    pub fn handle(&mut self, req: Request) -> Result<String> {
        fn json(v: &(impl Serialize + ?Sized)) -> Result<String> {
            Ok(serde_json::to_string(v).context(ResponseSnafu)?)
        }
        if matches!(self.mode, Mode::Templates { .. })
            && matches!(
                req,
                Request::Save { .. }
                    | Request::Merge { .. }
                    | Request::Move { .. }
                    | Request::SaveCopy { .. }
                    | Request::PackUseMine
                    | Request::PackMakeCurrent
            )
        {
            return EditingTemplatesSnafu.fail()?;
        }
        if matches!(self.mode, Mode::History { .. }) && req.edits() {
            return InHistorySnafu.fail()?;
        }
        // Les commandes modifient la session et renvoient le nouvel état ; les
        // requêtes renvoient ce qu'elles lisent.
        // Tout sauf la lecture du snapshot change l'état ; `ui` ne touche pas aux
        // documents ; `apply` écrit déjà son propre état de reprise (l'agent
        // l'appelle aussi directement, hors de `handle`), pas la peine de le refaire.
        // L'historique ne change pas le deck mis de côté, sauf `HistoryRestore`.
        let (changes, documents) = (
            !matches!(req, Request::Snapshot | Request::Apply { .. }),
            !matches!(
                req,
                Request::Snapshot
                    | Request::Ui(_)
                    | Request::HistoryOpen
                    | Request::HistorySeek { .. }
                    | Request::HistoryClose
                    | Request::PackMakeCurrent
                    | Request::PackNoticeClose
            ),
        );
        // La séance des widgets du mode deck finit à la première autre modification.
        if req.changes_deck() && !req.is_widget_edit() {
            self.end_widget_session();
        }
        match req {
            Request::Snapshot => {}
            Request::Slide { id } => return json(&self.slide(&id)?),
            Request::Svg { id } => return json(&*self.svg(&id)?),
            Request::Panel { slide } => return json(&self.panel(&slide)?),
            Request::Schema => return json(&oku_slides_core::ops_schema().context(ResponseSnafu)?),
            Request::Widgets => return json(&*self.registry),
            Request::Templates { set } => return json(&self.templates_view(set)?),
            Request::TemplateSets => return json(&self.library.packs()),
            Request::WidgetStatus => return json(&self.widget_status()?),
            Request::WidgetSource { name } => return json(&self.widget_source(&name)?),
            Request::SetWidget { name, source } => {
                let change = self.set_widget(&name, source)?;
                self.persist(true);
                return json(&change);
            }
            Request::RemoveWidget { name } => {
                let statuses = self.remove_widget(&name)?;
                self.persist(true);
                return json(&statuses);
            }
            Request::RenameWidget { from, to } => {
                let statuses = self.rename_widget(&from, &to)?;
                self.persist(true);
                return json(&statuses);
            }
            Request::PreviewWidget { name, params } => {
                return json(&self.preview_widget(&name, &params, PreviewFormat::Svg)?);
            }
            Request::DeleteTemplateSet { set } => {
                ensure!(!Library::is_builtin(set), BuiltinTemplateSetSnafu { id: set.to_string() });
                self.library.delete(set)?;
                self.template_cache = None;
                return json(&self.library.packs());
            }
            Request::Save { path, overwrite } => {
                self.save_checked(path.as_deref(), overwrite)?;
            }
            Request::Move { path } => self.move_to(&path)?,
            Request::SaveCopy { path } => self.save_copy(&path)?,
            Request::Merge { path } => {
                self.merge_file(&path)?;
            }
            Request::Apply { label, ops, merge } => self.apply(&label, ops, merge.as_deref())?,
            Request::ChartKind { label, slide, element, kind } => self.set_chart_kind(&label, slide, &element, kind)?,
            Request::Undo => self.undo()?,
            Request::Redo => self.redo()?,
            Request::EditTemplates { set } => self.edit_templates(set)?,
            Request::SaveTemplates { as_new } => self.save_templates(as_new)?,
            Request::CloseTemplates => self.close_templates()?,
            Request::Ui(ui) => self.ui = ui,
            Request::HistoryOpen => self.history_open()?,
            Request::HistorySeek { index } => self.history_seek(index)?,
            Request::HistoryRestore => self.history_restore()?,
            Request::HistoryClose => self.history_close()?,
            Request::HistoryVersions => return json(self.history_versions()?),
            Request::PackUseMine => self.pack_use_mine()?,
            Request::PackMakeCurrent => self.pack_make_current()?,
            Request::PackNoticeClose => self.pack_notice_close(),
        }
        if changes {
            self.persist(documents);
        }
        let out = json(&self.snapshot())?;
        // Les avis ne s'affichent qu'une fois : celui du document affiché
        // vient d'être envoyé, il ne le sera plus. (Celui du deck mis de
        // côté, s'il y en a un, n'est pas encore montré : il reste, pour
        // apparaître à son tour quand `close_templates` le rendra visible.)
        self.doc.notice = None;
        self.doc.disk_replaced = false;
        Ok(out)
    }

    pub fn new_deck(&mut self) -> Result<()> {
        let sources = SourceBundle::new(blank_deck(&self.localizer), self.registry.clone()).context(SourcesSnafu)?;
        self.load(sources, None)?;
        self.doc.blank = true;
        self.persist(true);
        Ok(())
    }

    /// Charge un document (déjà lu) dans la session.
    pub fn load(&mut self, sources: SourceBundle, path: Option<PathBuf>) -> Result<()> {
        self.ensure_not_in_history()?;
        self.load_document(Document::new(sources, path, &self.doc_defaults))
    }

    /// Charge un document ; ses graphiques sont mis sous forme canonique,
    /// pour que la grille puisse éditer leurs données. Il se lit et se rend
    /// avec les widgets du pack qu'il embarque ; sans pack, il adopte le pack
    /// par défaut (voir `deck_pack`).
    fn load_document(&mut self, mut doc: Document) -> Result<()> {
        self.end_widget_session();
        deck_pack::adopt_default_pack(&self.library, &mut doc.sources)?;
        let (widgets, notice) = self.resolve_widgets(doc.sources.deck().templates.as_ref(), &doc.sources.packs)?;
        doc.sources.set_registry(widgets.registry.clone());
        let mut deck = doc.sources.deck().clone();
        charts::canonicalize_deck(&mut deck, &self.chart_labels, &widgets.registry);
        // Les options réécrites reprennent la forme canonique (clés triées),
        // celle que relit le replica.
        oku_slides_core::canonicalize(&mut deck);
        if deck != *doc.sources.deck() {
            // Forme canonique des graphiques : pas une action de l'utilisateur.
            doc.sources.apply_silent(deck).context(SourcesSnafu)?;
        }
        // Le rendu des nouvelles polices (avec les nouveaux widgets) et les
        // widgets sont prêts avant de remplacer le document : si l'un
        // échoue, l'ancien document, son registre et son rendu restent.
        let fonts = (doc.sources.fonts != self.doc.sources.fonts)
            .then(|| self.fonts_renderer(&doc.sources.fonts, widgets.pack.as_ref(), &widgets.registry))
            .transpose()?;
        self.install_widgets(widgets)?;
        self.doc = doc;
        if let Some(fonts) = fonts {
            self.install_fonts(fonts);
        }
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
        self.ui.slide = None;
        self.pack_notice = None;
        self.rerender()
    }

    /// Ouvre un `.oku-slides.pdf` (ou un deck JSON, pour le développement).
    pub fn open(&mut self, path: &Path) -> Result<()> {
        let opened = read_document(&self.storage, path, self.source_limits, &self.registry)?;
        let (pdf, modified_elsewhere) = (opened.pdf, opened.modified_elsewhere);
        let notice = opened.notice(&self.localizer);
        // Un deck JSON n'est pas un document Oku Slides : on ne l'écrasera pas.
        self.load(opened.sources, pdf.then(|| path.to_owned()))?;
        // Les versions reçues entrent en silence ; une autre version que la courante : le bandeau.
        self.receive_packs();
        self.pack_notice = self.pack_notice_for_deck();
        tracing::info!(path = %path.display(), pdf, modified_elsewhere, "{}", self.localizer.tr("log-document-opened"));
        // Après l'avis d'un pack cassé, s'il y en a un.
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
        self.doc.saved_hash = pdf.then_some(opened.hash);
        self.persist(true);
        Ok(())
    }

    /// Écrit en `.oku-slides.pdf` (`path` déjà normalisé, voir `save_path`) :
    /// PDF + blob des sources, écriture atomique.
    fn write_to(&mut self, path: PathBuf) -> Result<()> {
        let hash = self.write_file(&path)?;
        self.doc.path = Some(path);
        self.doc.dirty = false;
        self.doc.saved_hash = Some(hash);
        Ok(())
    }

    /// Écrit le document (PDF et sources) dans `path` ; son empreinte.
    pub(crate) fn write_file(&mut self, path: &Path) -> Result<String> {
        let pdf = self.with_compiled(RemoteCompiled::pdf)?;
        let zip = self.doc.sources.to_zip().context(SourcesSnafu)?;
        let out = oku_slides_pdf::embed_sources(&pdf, &zip).context(PdfSnafu)?;
        let bytes = out.len();
        let hash = oku_slides_core::asset_id(&out);
        self.storage.write_atomic(path, out).context(WriteSnafu { path })?;
        tracing::info!(path = %path.display(), bytes, "{}", self.localizer.tr("log-document-saved"));
        Ok(hash)
    }

    /// Installe un rendu : le cache ne garde que les pages encore présentes.
    fn install(&mut self, compiled: RemoteCompiled) {
        let live: HashSet<u128> = (0..compiled.page_count()).filter_map(|i| compiled.page_hash(i)).collect();
        self.svg_cache.retain(|h, _| live.contains(h));
        self.appearance = compiled.layout().tokens.as_ref().map(appearance).unwrap_or_default();
        self.compiled = Some(compiled);
    }

    /// `export` sur le rendu installé. Une poignée périmée (processus de
    /// rendu relancé depuis) : le document est re-rendu une fois et `export`
    /// refait ; seul l'échec de ce re-rendu ou de ce second essai remonte.
    pub(crate) fn with_compiled<T>(
        &mut self,
        export: impl Fn(&RemoteCompiled) -> std::result::Result<T, RenderError>,
    ) -> Result<T> {
        let compiled = self.compiled.as_ref().context(NotRenderedSnafu)?;
        match export(compiled) {
            Err(e) if e.kind() == RenderErrorKind::Stale => {
                tracing::debug!("rendu périmé (processus de rendu relancé) : re-rendu");
                self.rerender()?;
                let compiled = self.compiled.as_ref().context(NotRenderedSnafu)?;
                Ok(export(compiled).context(RenderSnafu)?)
            }
            other => Ok(other.context(RenderSnafu)?),
        }
    }

    pub(crate) fn rerender(&mut self) -> Result<()> {
        let started = std::time::Instant::now();
        let compiled = self.render_deck(&self.doc.sources.deck().clone())?;
        tracing::debug!(pages = compiled.page_count(), ms = started.elapsed().as_millis(), "rendu");
        self.install(compiled);
        Ok(())
    }

    /// Redessine le graphique `element` en `kind` : ses données sont
    /// converties (en-têtes dans la langue de l'utilisateur), l'élément
    /// remplacé en une transaction.
    pub fn set_chart_kind(
        &mut self,
        label: &str,
        slide: String,
        element: &str,
        kind: oku_slides_chart::SeriesType,
    ) -> Result<()> {
        let deck = self.doc.sources.deck();
        let located = deck.slide(&slide).context(NoSlideSnafu { id: &slide })?.locate(element);
        let mut chart = located.map(|l| l.element.clone()).context(NoChartSnafu { id: element })?;
        // L'option est le paramètre `host(chart)` du widget.
        let param = charts::chart_param(&self.registry, &chart.kind).context(NoChartSnafu { id: element })?;
        let Some(ParamValue::Value(value)) = chart.params.get(param) else {
            return Err(NoChartSnafu { id: element }.build().into());
        };
        let option: oku_slides_chart::ChartOption =
            serde_json::from_value(value.clone()).ok().context(NoChartSnafu { id: element })?;
        let option = oku_slides_chart::convert(&option, kind, &self.chart_labels).context(ChartSnafu)?;
        let value = serde_json::to_value(option).context(ResponseSnafu)?;
        chart.params.set(param.to_owned(), ParamValue::Value(value));
        self.apply(label, vec![Op::ReplaceElement { slide, element: chart }], None)
    }

    /// Annule par le replica ; un rendu raté garde l'état et remonte l'erreur.
    pub fn undo(&mut self) -> Result<()> {
        self.end_widget_session();
        self.ensure_not_in_history()?;
        if self.doc.sources.undo().context(SourcesSnafu)? {
            self.doc.dirty = true;
            self.sync_widgets()?;
            self.rerender()?;
        }
        Ok(())
    }

    /// Rétablit par le replica ; un rendu raté garde l'état et remonte l'erreur.
    pub fn redo(&mut self) -> Result<()> {
        self.end_widget_session();
        self.ensure_not_in_history()?;
        if self.doc.sources.redo().context(SourcesSnafu)? {
            self.doc.dirty = true;
            self.sync_widgets()?;
            self.rerender()?;
        }
        Ok(())
    }

    fn page_of(&self, id: &str) -> Option<usize> {
        self.compiled.as_ref()?.layout().page_of(id)
    }

    pub fn snapshot(&self) -> Snapshot<'_> {
        let compiled = self.compiled.as_ref();
        let (deck, doc) = (self.doc.sources.deck(), &self.doc);
        let pages = compiled.map(|c| c.layout().slides.as_slice()).unwrap_or_default();
        let slides = deck
            .slides
            .iter()
            .enumerate()
            .map(|(i, s)| {
                // Une slide par page, dans l'ordre du deck : la i-ième page d'abord.
                let page = pages
                    .get(i)
                    .filter(|p| p.slide == s.id)
                    .or_else(|| pages.iter().find(|p| p.slide == s.id))
                    .map_or(0, |p| p.page);
                SlideSummary {
                    id: &s.id,
                    kind: &s.kind,
                    title: fields::slide_text(deck, s, &self.registry, "title").map(|t| interp(t, &deck.vars)),
                    name: s.name.as_deref(),
                    page,
                    hash: PageHash(compiled.and_then(|c| c.page_hash(page.checked_sub(1)?))),
                }
            })
            .collect();
        let name = self.document_name(doc);
        Snapshot {
            path: doc.path.as_deref(),
            name,
            dirty: doc.dirty,
            theme: deck.theme.as_deref(),
            themes: oku_slides_render::THEMES,
            title: deck.params.title.as_deref().map(|t| interp(t, &deck.vars)),
            title_source: deck.params.title.as_deref(),
            slides,
            can_undo: doc.sources.replica().can_undo(),
            undo_label: doc.sources.replica().undo_label(),
            can_redo: doc.sources.replica().can_redo(),
            redo_label: doc.sources.replica().redo_label(),
            warnings: compiled
                .map(|c| c.warnings.iter().map(|w| w.localize(&self.localizer)).collect())
                .unwrap_or_default(),
            notice: doc.notice.as_deref(),
            disk_replaced: doc.disk_replaced,
            appearance: &self.appearance,
            templates: self.deck_pack(),
            editing: self.editing(),
            pack_notice: self.pack_notice_view(),
            ui: &self.ui,
            history: match &self.mode {
                Mode::History { history, .. } => Some(history.view()),
                _ => None,
            },
        }
    }

    /// SVG d'une slide, mis en cache par empreinte de page.
    pub fn svg(&mut self, id: &str) -> Result<Arc<str>> {
        let page = self.page_of(id).context(NoSlideSnafu { id })?;
        let hash = |s: &Self| s.compiled.as_ref().and_then(|c| c.page_hash(page - 1)).unwrap_or_default();
        if let Some(svg) = self.svg_cache.get(&hash(self)) {
            return Ok(svg.clone());
        }
        let svg: Arc<str> = self.with_compiled(|c| c.svg(page - 1))?.into();
        // Empreinte du rendu installé : un re-rendu (poignée périmée) a pu le remplacer.
        self.svg_cache.insert(hash(self), svg.clone());
        Ok(svg)
    }

    /// Détail d'une slide : son contenu et ses boîtes sélectionnables.
    /// Messages A2UI du panneau de droite pour la slide `id`.
    pub fn panel(&self, id: &str) -> Result<Vec<oku_slides_a2ui::ServerMessage>> {
        let view = self.slide(id)?;
        let editing = matches!(self.mode, Mode::Templates { .. });
        Ok(panel::panel(view.slide, view.zone.and_then(|z| z.grid.as_ref()), editing, &self.localizer))
    }

    pub fn slide(&self, id: &str) -> Result<SlideView<'_>> {
        let no_slide = || -> SessionError { NoSlideSnafu { id }.build().into() };
        let deck = self.doc.sources.deck();
        let slide = deck.slide(id).ok_or_else(no_slide)?;
        let compiled = self.compiled.as_ref().ok_or_else(no_slide)?;
        let layout = compiled.layout();
        let page = layout.page_of(id).ok_or_else(no_slide)?;
        let targets = layout
            .boxes_on(page)
            .iter()
            .filter_map(|b| {
                let l = slide.locate(&b.id)?;
                Some(Target::element(b, &l.element.kind, l.is_free()))
            })
            .collect();
        let fields = fields::field_views(deck, slide, &self.registry, layout.fields_on(page));
        Ok(SlideView {
            slide,
            page,
            hash: PageHash(compiled.page_hash(page - 1)),
            width: 1920.0,
            height: 1080.0,
            targets,
            fields,
            zone: layout.zone_on(page),
        })
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
