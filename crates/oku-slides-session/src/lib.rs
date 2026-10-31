//! Editing session and workspace: the deck, history, rendering and file of
//! each tab.
//!
//! - [`Session`] and [`Request`]: one deck (one session per tab).
//! - [`Workspace`], [`WorkspaceRequest`], [`WorkspaceView`], [`TabInfo`],
//!   [`TabDocument`], [`DocumentKind`]: the windows, the tabs and what each edits (a deck,
//!   the help, a pack or a theme, each in its own tab: [`ThemeTab`], [`ThemeTabView`]),
//!   and the resume of the whole.
//!
//! The agent reaches a tab's session through [`agent::TabSession`], provided by
//! the host through [`agent::Sessions`] (see [`Workspace::with_session`]).
//!
//! The Tauri app and the development server expose the same API. Lock order: a pack's or theme's opening
//! lock, then the workspace, then one session, never the reverse; eight functions take both in sequence:
//! `Workspace::apply`, `request`, `open_files`, `settle`, `post_notice`, `open_pack_with`, `open_theme_ui`
//! and `pack_opener`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::Value;
use snafu::{OptionExt, ResultExt};

use oku_slides_chart::ChartLabels;
use oku_slides_core::{
    Deck, DeckParams, Element, Namespace, PackPin, Paragraph, ParamValue, Slide, SourceBundle, SourceLimits,
    WidgetRegistry, kinds, qualify,
};
pub use oku_slides_i18n::{Localize, Localizer};
pub use oku_slides_platform::Platform;
pub use oku_slides_platform::config::Settings;
pub use oku_slides_platform::storage::Storage;
pub use oku_slides_render::Fonts;
use oku_slides_render::{PackSet, RenderError, RenderErrorKind};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot, RenderWorker};

mod add_grid;
pub mod agent;
mod appearance;
mod carried_assets;
mod charts;
mod color_warnings;
mod data_edit;
mod data_panel;
mod deck_adopt;
mod deck_closure;
mod deck_fallback;
mod deck_file;
mod deck_offer;
mod deck_pack;
mod deck_pack_link;
mod deck_pack_receive;
mod deck_pack_tab;
mod deck_packs;
mod deck_packs_panel;
mod deck_theme;
mod deck_theme_edit;
mod deck_usage;
mod deck_widgets;
mod document_kind;
mod error;
mod example_text;
mod fields;
mod files;
mod help;
mod history;
pub mod import;
mod inline;
mod items;
mod layout_examples;
#[cfg(test)]
pub(crate) mod lib_tests_support;
pub mod library;
mod library_index;
mod library_kind;
mod library_lookup;
mod library_packs;
mod merge;
mod namespace_slug;
pub mod pack;
mod pack_lease;
mod pack_library;
mod pack_mode;
mod pack_mode_lease;
mod pack_mode_preview;
mod pack_mode_previews;
mod pack_mode_save;
mod pack_receive;
mod pack_scratch;
mod pack_tab;
mod panel;
mod param_fields;
mod preview_crop;
mod preview_theme;
mod probed_widgets;
mod render_fonts;
mod request;
mod resume;
mod resume_agent;
mod resume_writer;
mod search;
mod tab_document;
mod tables;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
pub(crate) mod test_support_packs;
mod text;
mod theme;
mod theme_closure;
mod theme_composed;
mod theme_controls;
mod theme_draft;
mod theme_edit;
mod theme_extends;
mod theme_panel;
mod theme_picker;
pub mod theme_schema;
mod theme_tab;
mod theme_tab_save;
mod undo;
mod views;
#[cfg(any(test, feature = "test-support"))]
pub mod voice_fixture;
mod voice_panel;
mod voice_view;
mod widget_examples;
mod widget_params;
mod widget_preview;
mod widget_preview_items;
mod widgets_deck;
mod widgets_edit;
mod workspace;

pub use add_grid::{AddPackView, AddWidget, ElementSeed};
pub use agent::usage_table::{USAGE_COLUMNS, usage_cells, usage_headers, usage_total_cells};
pub use appearance::{Appearance, appearance};
pub use data_edit::{DataEdit, TableFlag};
pub use deck_pack::{adopt_default_packs, check_pack_id};
pub use deck_packs::{DeckPacksView, Drift};
pub use deck_packs_panel::{HOST_PACKS, PacksEvent, deck_packs_panel, packs_event};
pub use deck_theme::{adopt_default_theme, repair_theme_closure, sources_theme_layer};
pub use deck_theme_edit::{EditedTheme, TokenOp};
pub use document_kind::DocumentKind;
use error::{NoSlideSnafu, NotRenderedSnafu, RegistrySnafu, RenderSnafu, SourcesSnafu};
pub use error::{Result, SessionError, SessionErrorKind};
pub use fields::{FieldKind, FieldView, OwnerKind};
pub use files::{EXTENSION, Opened, deck_uid_of, read_document};
pub use items::{ItemView, ListView};
pub use library::{Library, builtin_versions};
pub use library_kind::{Layouts, LibraryKind, StoredVersion, VersionManifest, Widgets};
pub use library_lookup::{embed_theme, library_pack, read_pack_dir, resolve_pack};
pub use library_packs::PackLibrary;
pub use merge::{Saved, deck_path};
pub use pack::{BASE_PACK, ESSENTIEL_PACK, PackInfo, PackManifest, PackScope, Relation};
use pack_scratch::PackScratch;
pub use probed_widgets::{ProbedWidgets, sources_with_packs};
pub use request::{Request, UiOrder, View};
pub use search::SearchHit;
pub use tab_document::TabDocument;
pub use text::{interp, paragraphs_to_text};
pub use theme::{
    DEFAULT_THEME, NEUTRE_NAMESPACE, NEUTRE_THEME, PAPIER_NAMESPACE, PAPIER_THEME, PRO_NAMESPACE, PRO_THEME, ThemeInfo,
    ThemeLibrary, ThemeManifest, Themes,
};
pub use theme_edit::{ThemeEdit, ThemeEditView, ThemeParentView};
pub use theme_extends::ExtendsEdit;
pub use theme_panel::{THEME_EXTENDS_EVENT, TokenAction, TokenState};
pub use theme_picker::{HOST_THEME_PICKER, THEME_EVENT, ThemeCard, ThemeChoices, picked_theme, theme_picker};
pub use theme_tab::{ThemeTab, ThemeTabView};
pub use views::{
    EditingView, HistoryView, LayoutPackView, LayoutView, LayoutsView, PackMode, PageHash, PileSection, SlideSummary,
    SlideView, Snapshot, TabInfo, Target, UiState,
};
pub use voice_panel::{HOST_VOICE, VoicePanel, gigabytes_text, model_label, seconds_text, size_text};
pub use voice_view::{
    AgentModelView, AgentModelsView, AgentPresetView, AgentsView, ApiView, DICTATION_LANGUAGES, DictationLanguage,
    DictationPatch, DictationView, DownloadView, LocalFailure, LocalModelView, ModelKind, ModelView, VoiceEvent,
    VoiceView, voice_event,
};
pub use widget_preview::{PreviewBox, PreviewFormat, WidgetPreview};
pub use widgets_edit::{WidgetChange, WidgetDiagnostic, WidgetStatus};
pub use workspace::{
    Clip, ImportPick, Rect, TabView, WindowView, Workspace, WorkspaceOutcome, WorkspaceRequest, WorkspaceView,
    is_window_label,
};

/// Un document ouvert : ses sources (deck et son historique, assets,
/// polices) et son fichier.
#[expect(clippy::struct_excessive_bools, reason = "independent flags of a document, each written to document.json")]
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
    /// Agent usage recorded since the last save: the file does not hold it yet.
    ///
    /// Never makes the deck unsaved: a clean tab that closes writes it alone.
    usage_unsaved: bool,
    /// The full ledger's refusal is already logged: logged once per document.
    usage_full_warned: bool,
}

impl Document {
    /// `defaults` : étapes d'annulation gardées et auteur (réglages
    /// `history.limit` et `[author]`).
    fn new(mut sources: SourceBundle, path: Option<PathBuf>, defaults: &DocDefaults) -> Self {
        sources.set_undo_limit(defaults.undo_limit);
        sources.set_author(defaults.author.clone());
        Self {
            sources,
            path,
            dirty: false,
            notice: None,
            saved_hash: None,
            blank: false,
            disk_replaced: false,
            usage_unsaved: false,
            usage_full_warned: false,
        }
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
    /// Agent usage is recorded and saved (`[usage] store_in_deck`).
    store_usage: bool,
}

/// What the session edits: the deck, a pack (pack mode) while the deck
/// waits aside, or a theme over its specimen deck (a theme tab); or what it
/// shows: a version of the deck, read-only, the deck aside too; or the help, read-only.
pub(crate) enum Mode {
    Deck,
    /// The shipped help (`help.rs`), in this locale: nothing waits aside.
    Help {
        locale: &'static str,
    },
    Pack {
        /// The deck set aside (a pack tab's preview copy), found unchanged at the end.
        preview: Box<Document>,
        edit: PackMode,
        /// Draft (trials), reset at each entry.
        scratch: PackScratch,
        /// The edited library pack's lease; `None`: read-only until taken.
        lease: Option<pack_lease::PackLease>,
        /// The pack left the library while dormant: read-only for the session's life.
        orphaned: bool,
        /// A deck-pack tab's owner and unsent save; `None` for any other pack mode.
        link: Option<Box<deck_pack_tab::TabLink>>,
    },
    /// A library theme edited in its own tab: its tokens are the specimen deck's.
    Theme {
        /// The draft, its writes' parent and its last write.
        edit: ThemeTab,
        /// Its parents' changes, tracked for undo.
        switches: Vec<theme_edit::ThemeSwitch>,
        /// The edited theme's lease; `None`: read-only until taken.
        lease: Option<pack_lease::PackLease>,
    },
    History {
        /// The deck set aside; its images and fonts are lent to the preview.
        stash: Box<Document>,
        history: history::HistoryMode,
    },
}

// ---- Session ----------------------------------------------------------------------

/// An open tab: its document, mode, rendering and libraries.
///
/// Each tab has its own session, under its own lock (ADR-0001).
pub struct Session {
    doc: Document,
    mode: Mode,
    library: PackLibrary,
    /// Thèmes de l'utilisateur et intégrés.
    themes: ThemeLibrary,
    /// Leur liste, relue au chargement et au snapshot demandé : `snapshot()` ne lit pas le disque.
    theme_list: Vec<ThemeInfo>,
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
    /// Packs utilisables, dont viennent les widgets ; vide : aucun widget.
    packs: PackSet,
    /// Packs épinglés des deux sortes (`deck_closure::combined_pins`) pour lesquels `registry` et `packs` ont été résolus.
    widgets_pins: BTreeMap<Namespace, PackPin>,
    /// Par pack installé, les fichiers qui restent à sonder (budget épuisé).
    widgets_unprobed: BTreeMap<Namespace, BTreeSet<oku_slides_core::WidgetFile>>,
    /// Packs épinglés écartés pour la session (absents, illisibles, fautifs), et leur nom.
    widgets_disabled: BTreeMap<Namespace, String>,
    /// Rendu des aperçus de layouts, séparé du deck, et leur cache par pack de layouts.
    pack_renderer: Option<RenderSlot>,
    pack_cache: pack_mode_previews::PreviewCache,
    /// Départs des widgets de la grille d'ajout, par version de pack et langue ; vidé avec `pack_cache`.
    seed_cache: add_grid::SeedCache,
    /// Rendu des variantes proposées par l'agent, séparé du deck.
    variant_renderer: Option<RenderSlot>,
    /// Rendu du deck, ouvert au premier rendu (voir `render_fonts`).
    renderer: Option<RenderSlot>,
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
    /// Ordre de la dernière sauvegarde de `ui` reçue avec un ordre (pas gardé pour la reprise).
    ui_order: Option<request::UiOrder>,
    /// Images collées dans le brouillon de l'agent de cet onglet (`[paste]`).
    attachments: agent::attachments::Attachments,
    /// Assets apportés par le dernier élément collé depuis un autre onglet (`carried_assets`).
    carried: carried_assets::CarriedAssets,
    /// Reprise activée (app de bureau) : l'état est écrit après chaque
    /// commande, par un fil dédié (`resume_writer`).
    resume: Option<resume_writer::Writer>,
    /// Images et polices déjà envoyées au fil de reprise : partagées tant
    /// qu'elles ne changent pas (voir `resume::DocSnapshot`).
    sent_blobs: Vec<Arc<resume::Blobs>>,
    /// Usage ledgers already sent to the resume thread: shared while unchanged,
    /// so the thread rewrites `usage.jsonl` only for a new one.
    sent_usage: Vec<Arc<oku_slides_core::UsageLedger>>,
    /// Ce que l'utilisateur a fait sans l'agent depuis son dernier tour.
    agent_notes: Vec<String>,
    /// Étapes d'annulation du deck prises par les derniers tours de l'agent (`agent::turn_undo`).
    turn_steps: agent::turn_undo::TurnSteps,
    /// La conversation de l'agent de l'onglet (`agent::conversation`).
    conversation: Option<agent::SavedConversation>,
    /// Épinglages (packs et thème) plus anciens que la bibliothèque (`deck_packs`), recalculés quand ils changent.
    pack_updates: u32,
    /// Les épinglages pour lesquels `pack_updates` a été calculé (`deck_packs::PinsKey`).
    pack_updates_pins: Option<deck_packs::PinsKey>,
    /// L'avis des thèmes écartés posé sur le deck vierge, tant qu'un autre document peut le remplacer avant qu'il soit vu.
    themes_notice: Option<String>,
    /// Bornes des fichiers de widgets (`[render]`).
    widget_limits: widgets_edit::WidgetLimits,
    /// Une écriture de widgets est en cours : l'avis des widgets écartés se tait.
    widget_edit: bool,
    /// Séance de modification des widgets en mode deck.
    #[expect(
        clippy::struct_field_names,
        reason = "the widget session (WidgetSession) from the rules vocabulary, not a Session"
    )]
    widget_session: Option<widgets_deck::WidgetSession>,
    /// Emplacement d'essai de l'aperçu des widgets.
    widget_preview: Option<widget_preview::PreviewSlot>,
    /// L'import d'où vient le deck montré, en mémoire tant que vit l'onglet ; jamais repris.
    import: Option<import::ImportState>,
    /// The deck-pack tabs linked to this owner deck, by tab id (`deck_pack_link`).
    deck_links: BTreeMap<String, deck_pack_link::DeckLink>,
    /// A turn end or history close asked for the links' parked sends.
    link_pull: bool,
    /// The layout generation of the last `set_deck_links` applied.
    links_generation: u64,
    #[cfg(test)]
    failpoints: deck_widgets::Failpoints,
}

/// Un emplacement de `worker` avec les widgets des packs `set` (de registre
/// `registry`), dont l'image absente se dit dans la langue de l'utilisateur.
///
/// Un ensemble qui porte un pack fautif n'est pas posé : l'emplacement n'a
/// alors aucun widget, et toute slide ou tout élément y est inconnu.
/// À l'appelant d'avoir écarté ce pack d'abord (`Session::usable_packs`),
/// pour que l'ensemble soit sain et que la session suive.
///
/// Sans pack posé (vide ou fautif), `registry` n'est pas lu : l'emplacement
/// rend sans widget, à l'appelant d'y accorder son registre.
pub(crate) fn new_renderer(
    worker: &RenderWorker,
    fonts: &Fonts,
    l: &Localizer,
    set: &PackSet,
    registry: &Arc<WidgetRegistry>,
) -> Result<RenderSlot> {
    let mut slot = worker.slot(fonts, l, deck_pack::empty_registry()).context(RenderSnafu)?;
    if !set.is_empty() && !set.packs().iter().any(|p| worker.is_faulty(p)) {
        slot.set_widgets(set, registry.clone()).context(RenderSnafu)?;
    }
    Ok(slot)
}

/// Deck vide : une couverture (`essentiel/title`) et une slide de contenu
/// (`essentiel/content`) avec un `base/text`, dans la langue de l'utilisateur.
pub fn blank_deck(l: &Localizer) -> Deck {
    let mut content = Slide::new("s-1", kinds::essentiel::CONTENT);
    content.params.set("title", ParamValue::Text(l.tr("deck-new-slide-title")));
    let text = Element::text("b-1", vec![Paragraph::plain(l.tr("deck-new-slide-text"))]);
    content.params.set("blocks", ParamValue::Slot(vec![text]));
    Deck {
        version: oku_slides_core::DECK_VERSION,
        // Adoptés au chargement : la fermeture du thème par défaut, dans sa version courante.
        theme: None,
        themes: BTreeMap::new(),
        params: DeckParams {
            title: Some(l.tr("deck-new-title")),
            // Code de langue seul (`zh-Hant` → `zh`) : Typst en déduit la
            // direction du texte (droite à gauche en arabe) et les césures.
            lang: l.locale().split('-').next().map(str::to_owned),
            ..Default::default()
        },
        vars: BTreeMap::new(),
        tokens: Value::Null,
        // Adoptés au chargement : `base`, `essentiel` et leurs besoins.
        widgets: BTreeMap::new(),
        layouts: BTreeMap::new(),
        slides: vec![Slide::new("s-titre", kinds::essentiel::TITLE), content],
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
    /// A rendered session on a blank deck.
    ///
    /// `app_fonts`: fonts shipped with the app, always available;
    /// `platform`: settings (history, models folder), files and
    /// language.
    ///
    /// # Errors
    ///
    /// Fails if the session cannot be built (registry, libraries, blank deck) or if its first
    /// render fails.
    pub fn new(app_fonts: Fonts, platform: &Platform) -> Result<Self> {
        let mut s = Self::unrendered(app_fonts, platform)?;
        s.rerender()?;
        Ok(s)
    }

    /// [`Self::new`] sans son premier rendu : l'emplacement du deck, et avec lui
    /// le processus de rendu, attendent le premier rendu demandé (voir
    /// `render_fonts`). Seule la sonde d'un pack par défaut de l'utilisateur le lance avant.
    pub(crate) fn unrendered(app_fonts: Fonts, platform: &Platform) -> Result<Self> {
        let (settings, storage, l) = (&platform.settings, platform.storage.clone(), &platform.localizer);
        let doc_defaults = DocDefaults {
            undo_limit: settings.history.limit,
            author: settings.author.resolved(),
            store_usage: settings.usage.store_in_deck,
        };
        let worker = RenderWorker::new(platform.render_launch.clone(), settings.render.clone());
        // Aucun pack installé encore : aucun widget, jusqu'à `sync_widgets`.
        let registry = deck_pack::empty_registry();
        let themes = ThemeLibrary::user(platform);
        let library = PackLibrary::with_themes(platform, themes.clone());
        // Le premier replica connaît déjà les widgets de `base` et d'`essentiel` que le deck neuf épingle.
        let builtin = oku_slides_render::builtin_registry(&app_fonts).context(RegistrySnafu)?;
        let mut blank = SourceBundle::new(blank_deck(l), builtin).context(SourcesSnafu)?;
        deck_pack::adopt_default_packs(&library, &mut blank)?;
        deck_theme::adopt_default_theme(&themes, &mut blank)?;
        let mut s = Self {
            doc: Document { blank: true, ..Document::new(blank, None, &doc_defaults) },
            mode: Mode::Deck,
            library,
            theme_list: themes.themes(),
            themes,
            decks_dir: None,
            storage,
            doc_defaults,
            source_limits: files::source_limits(settings),
            renderer: None,
            worker,
            registry,
            packs: PackSet::default(),
            widgets_pins: BTreeMap::new(),
            widgets_unprobed: BTreeMap::new(),
            widgets_disabled: BTreeMap::new(),
            localizer: l.clone(),
            chart_labels: ChartLabels::new(l),
            render_fonts: app_fonts.clone(),
            render_deck_fonts: 0,
            appearance: Appearance::default(),
            pack_renderer: None,
            pack_cache: pack_mode_previews::PreviewCache::default(),
            seed_cache: add_grid::SeedCache::default(),
            variant_renderer: None,
            app_fonts,
            compiled: None,
            svg_cache: HashMap::new(),
            ui: UiState::default(),
            ui_order: None,
            attachments: agent::attachments::Attachments::new(settings.paste.clone()),
            carried: carried_assets::CarriedAssets::new(),
            resume: None,
            agent_notes: Vec::new(),
            turn_steps: agent::turn_undo::TurnSteps::default(),
            conversation: None,
            pack_updates: 0,
            pack_updates_pins: None,
            themes_notice: None,
            widget_limits: widgets_edit::WidgetLimits::of(&settings.render),
            widget_edit: false,
            widget_session: None,
            widget_preview: None,
            import: None,
            sent_blobs: Vec::new(),
            sent_usage: Vec::new(),
            deck_links: BTreeMap::new(),
            link_pull: false,
            links_generation: 0,
            #[cfg(test)]
            failpoints: deck_widgets::Failpoints::default(),
        };
        s.notice_skipped_themes();
        // Le décompte part des épinglages du deck neuf (`refresh_pack_updates_if_pins_changed` les compare ensuite).
        s.refresh_pack_updates();
        // Les packs par défaut de l'utilisateur (`base`, `essentiel` modifiés) se sondent ici.
        s.sync_widgets()?;
        Ok(s)
    }

    /// The deck shown: the tab's own, the pack mode's or the history's.
    pub fn deck(&self) -> &Deck {
        self.doc.sources.deck()
    }

    /// Sources du document courant (tests seulement : réplica Loro, auteurs…).
    #[doc(hidden)]
    pub fn sources_for_tests(&self) -> &SourceBundle {
        &self.doc.sources
    }

    /// Le deck de l'utilisateur, même pendant l'édition d'un pack.
    fn user_deck(&self) -> &Deck {
        match &self.mode {
            Mode::Deck | Mode::Help { .. } | Mode::Theme { .. } => self.doc.sources.deck(),
            Mode::Pack { preview: stash, .. } | Mode::History { stash, .. } => stash.sources.deck(),
        }
    }

    /// Les sources qui portent les images, polices et packs du deck de l'utilisateur.
    ///
    /// Le deck mis de côté en mode pack ; en historique, le document
    /// montré, qui les a pris au deck mis de côté (`history.rs`).
    fn user_blobs(&self) -> &SourceBundle {
        match &self.mode {
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } | Mode::Theme { .. } => &self.doc.sources,
            Mode::Pack { preview, .. } => &preview.sources,
        }
    }

    /// Les widgets des packs en cours, qualifiés, lus dans leurs fichiers sans rien construire ni sonder.
    ///
    /// En mode pack, ceux du pack édité ; sinon ceux de chaque pack épinglé par le deck.
    fn pack_widget_names(&self) -> Vec<String> {
        let named = |ns: &Namespace, f: &oku_slides_core::PackFiles| -> Vec<String> {
            f.widgets().keys().filter(|n| !n.is_common()).map(|n| qualify(ns, n.as_str())).collect()
        };
        match &self.mode {
            Mode::Pack { edit, .. } => {
                let files = self.doc.sources.packs.get(&edit.base);
                let ns = self.edited_pin().map(|(ns, _)| ns);
                files.zip(ns).map_or_else(Vec::new, |(f, ns)| named(ns, f))
            }
            Mode::Deck | Mode::Help { .. } | Mode::Theme { .. } => self
                .user_deck()
                .all_pins()
                .filter_map(|(_, ns, pin)| Some(named(ns, self.user_blobs().packs.get(&pin.version)?)))
                .flatten()
                .collect(),
            Mode::History { .. } => Vec::new(),
        }
    }

    fn editing(&self) -> Option<&PackMode> {
        match &self.mode {
            Mode::Deck | Mode::Help { .. } | Mode::History { .. } | Mode::Theme { .. } => None,
            Mode::Pack { edit, .. } => Some(edit),
        }
    }

    /// Unsaved work: the deck, or the pack a pack tab edits (not the history's preview).
    ///
    /// With history open, agent usage the file lacks counts too: no close
    /// writes it silently there, and a close must not drop it.
    fn unsaved(&self) -> bool {
        match &self.mode {
            Mode::Deck | Mode::Help { .. } | Mode::Theme { .. } => self.doc.dirty,
            Mode::Pack { .. } => self.doc.dirty || self.unsent(),
            Mode::History { stash, .. } => stash.dirty || stash.usage_unsaved,
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

    /// What the tab bar shows of this session.
    ///
    /// A pack tab reports its pack's name and a `Pack` (or `DeckPack`)
    /// document, a theme tab its theme's and a `Theme` document, a help tab a
    /// `Help` document. Otherwise, while history is open, the title and the
    /// file stay those of the stashed deck.
    pub fn tab_info(&self) -> TabInfo {
        if let Some(info) = self.pack_tab_info().or_else(|| self.theme_tab_info()) {
            return info;
        }
        let deck = match &self.mode {
            Mode::Deck | Mode::Help { .. } | Mode::Theme { .. } => &self.doc,
            Mode::Pack { preview: stash, .. } | Mode::History { stash, .. } => stash,
        };
        let document = match self.help() {
            Some(locale) => TabDocument::Help { locale: locale.to_owned() },
            None => TabDocument::Deck {
                path: deck.path.as_deref().map(oku_slides_platform::storage::resolve),
                deck_uid: Some(deck.sources.replica().deck_uid()),
            },
        };
        TabInfo {
            title: self.document_name(deck),
            dirty: self.unsaved(),
            pristine: matches!(self.mode, Mode::Deck)
                && deck.blank
                && deck.path.is_none()
                && !deck.dirty
                && !deck.sources.replica().can_undo()
                && !deck.sources.replica().can_redo(),
            theme: {
                let user = deck.sources.deck();
                user.theme.as_ref().and_then(|ns| user.themes.get(ns)).map(|pin| pin.id)
            },
            document,
            created: None,
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
        self.sent_usage.clear();
    }

    /// Attend que l'état de reprise envoyé soit écrit (avant de quitter).
    pub(crate) fn flush_resume(&self) {
        if let Some(writer) = &self.resume {
            writer.flush();
        }
    }

    /// Sets the user's widget and layout pack folders (the library).
    ///
    /// # Errors
    ///
    /// `EditingPack` in pack mode, whose lease belongs to the current library.
    pub fn set_pack_dirs(&mut self, widgets: impl Into<PathBuf>, layouts: impl Into<PathBuf>) -> Result<()> {
        self.move_pack_dirs(widgets.into(), layouts.into())
    }

    /// Dossier où s'enregistre de lui-même un deck qui n'a pas de fichier.
    pub fn set_decks_dir(&mut self, dir: impl Into<PathBuf>) {
        self.decks_dir = Some(dir.into());
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

    fn page_of(&self, id: &str) -> Option<usize> {
        self.compiled.as_ref()?.layout().page_of(id)
    }

    /// The full state the interface shows, read without touching the disk.
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
            theme: deck.theme.as_ref(),
            theme_pins: &deck.themes,
            theme_name: deck.theme.as_ref().and_then(|ns| self.theme_manifest(deck.themes.get(ns)?)).map(|m| m.name),
            themes: &self.theme_list,
            title: deck.params.title.as_deref().map(|t| interp(t, &deck.vars)),
            title_source: deck.params.title.as_deref(),
            lang: deck.params.lang.as_deref(),
            slides,
            can_undo: doc.sources.replica().can_undo(),
            undo_label: doc.sources.replica().undo_label(),
            can_redo: doc.sources.replica().can_redo(),
            redo_label: doc.sources.replica().redo_label(),
            undo_turn: self.undo_turn(),
            warnings: compiled
                .map(|c| {
                    let typst = c.warnings.iter().map(|w| w.localize(&self.localizer));
                    let tokens = c.layout().tokens.as_ref();
                    typst.chain(color_warnings::unknown_color_warnings(deck, tokens, &self.localizer)).collect()
                })
                .unwrap_or_default(),
            notice: doc.notice.as_deref(),
            disk_replaced: doc.disk_replaced,
            appearance: &self.appearance,
            widgets: self.deck_pins(),
            layouts: &self.user_deck().layouts,
            editing: self.editing_view(),
            theming: self.theme_tab_view(),
            deck_pack: self.deck_pack_is_deck_only(),
            pack_updates: self.pack_updates(),
            pack_widgets: self.pack_widget_names(),
            add_packs: self.add_packs(),
            free_slots: self.free_slots(),
            widget_trials: self.examples_revision(),
            ui: &self.ui,
            history: match &self.mode {
                Mode::History { history, .. } => Some(history.view()),
                _ => None,
            },
            read_only: self.help().is_some(),
            host_params: self.registry.host_params(),
        }
    }

    /// SVG of a slide, cached by page hash.
    ///
    /// # Errors
    ///
    /// Fails (`NoSlide`) for an unknown slide, or if rendering the page to SVG fails.
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

    /// Detail of a slide: its content and its selectable boxes.
    ///
    /// # Errors
    ///
    /// Fails (`NoSlide`) if the slide is unknown to the deck or to its rendered layout.
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
            width: views::SLIDE_WIDTH,
            height: views::SLIDE_HEIGHT,
            targets,
            fields,
            items: items::item_views(slide, &self.registry, layout.items_on(page)),
            lists: items::list_views(slide, &self.registry),
            zone: layout.zone_on(page),
        })
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
