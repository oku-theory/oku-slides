//! Session d'édition : le deck ouvert, son historique, son rendu et son
//! fichier. L'app Tauri et le serveur de développement exposent la même API
//! (voir [`Request`] et [`Session::handle`]).

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_chart::ChartLabels;
use oku_slides_core::{Deck, Element, History, Op, Paragraph, Params, Slide, SlideKind, SourceBundle};
pub use oku_slides_i18n::{Localize, Localizer};
pub use oku_slides_platform::Platform;
pub use oku_slides_platform::config::Settings;
pub use oku_slides_platform::storage::Storage;
pub use oku_slides_render::Fonts;
use oku_slides_render::{Compiled, Renderer};

mod appearance;
mod charts;
mod error;
mod files;
mod panel;
pub mod templates;
mod text;
mod views;

pub use appearance::{Appearance, appearance};
use error::{
    BuiltinTemplateSetSnafu, ChartSnafu, EditingTemplatesSnafu, NoChartSnafu, NoPathSnafu, NoSlideSnafu,
    NoTemplateSetSnafu, NotEditingTemplatesSnafu, NotRenderedSnafu, OpSnafu, PdfSnafu, RenderSnafu, ResponseSnafu,
    SourcesSnafu, WriteSnafu,
};
pub use error::{Result, SessionError, SessionErrorKind};
pub use files::{EXTENSION, Opened, read_document};
pub use panel::PANEL_SURFACE;
pub use templates::{DEFAULT_SET, Library, SetInfo, Template, TemplateSet};
pub use text::{interp, paragraphs_to_text};
pub use views::{
    PageHash, SlideSummary, SlideView, Snapshot, Target, TargetKind, TemplateMode, TemplateView, TemplatesView,
};

/// Un document ouvert : ses sources (deck, assets, polices), son historique
/// et son fichier.
struct Document {
    sources: SourceBundle,
    history: History,
    path: Option<PathBuf>,
    dirty: bool,
    /// Signalé à l'ouverture : le PDF a été modifié par un autre outil.
    notice: Option<String>,
}

impl Document {
    fn new(sources: SourceBundle, path: Option<PathBuf>, history_limit: usize) -> Self {
        Self { sources, history: History::new(history_limit), path, dirty: false, notice: None }
    }
}

/// Ce que la session édite : le deck, ou un ensemble de modèles pendant que
/// le deck attend de côté.
enum Mode {
    Deck,
    Templates {
        /// Le deck mis de côté, retrouvé tel quel à la fin.
        stash: Box<Document>,
        edit: TemplateMode,
    },
}

// ---- Requêtes ----------------------------------------------------------------------

/// Une requête de l'interface (même forme pour Tauri et HTTP).
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    Snapshot,
    New,
    Open {
        path: PathBuf,
    },
    Save {
        path: Option<PathBuf>,
    },
    Slide {
        id: String,
    },
    Svg {
        id: String,
    },
    Apply {
        label: String,
        ops: Vec<Op>,
    },
    Undo,
    Redo,
    Schema,
    /// Langue de l'utilisateur et messages Fluent de l'interface.
    Messages,
    /// Surface A2UI du panneau de droite pour une slide (messages v0.9).
    Panel {
        slide: String,
    },
    /// Change le type d'un graphique ; ses données sont converties. Annulable.
    ChartKind {
        label: String,
        slide: String,
        element: String,
        kind: oku_slides_chart::SeriesType,
    },
    /// Modèles d'un ensemble (celui du deck par défaut), rendus avec le thème du deck.
    Templates {
        #[serde(default)]
        set: Option<String>,
    },
    /// Liste des ensembles de modèles.
    TemplateSets,
    /// Passe en édition d'un ensemble de modèles (le deck est mis de côté).
    EditTemplates {
        set: String,
    },
    /// Enregistre l'ensemble édité ; `as_new` (ou un ensemble intégré) : une copie.
    SaveTemplates {
        #[serde(default)]
        as_new: bool,
    },
    /// Quitte l'édition des modèles et retrouve le deck.
    CloseTemplates,
    /// Supprime un ensemble de l'utilisateur.
    DeleteTemplateSet {
        set: String,
    },
}

// ---- Session ----------------------------------------------------------------------

pub struct Session {
    doc: Document,
    mode: Mode,
    library: Library,
    storage: Storage,
    localizer: Arc<Localizer>,
    /// En-têtes des graphiques reçus, dans la langue de l'utilisateur.
    chart_labels: ChartLabels,
    /// Transactions gardées pour annuler, dans chaque document.
    history_limit: usize,
    /// Rendu des aperçus de modèles, séparé du deck, avec leur clé de cache.
    template_renderer: Option<Renderer>,
    template_cache: Option<Previews>,
    renderer: Renderer,
    /// Polices livrées avec l'app, et celles du rendu (app + deck).
    app_fonts: Fonts,
    render_fonts: Fonts,
    /// Apparence tirée du dernier rendu.
    appearance: Appearance,
    compiled: Option<Compiled>,
    svg_cache: HashMap<u128, Arc<str>>,
}

/// Aperçus de modèles en cache, avec ce dont ils dépendent : l'ensemble et
/// le style du deck (le deck d'aperçu les porte).
struct Previews {
    set: TemplateSet,
    deck: Deck,
    views: Arc<[TemplateView]>,
}

/// Un deck qui a le style de `base` (thème, jetons, paramètres, variables,
/// ensemble de modèles) et ces slides.
fn styled_like(base: &Deck, slides: Vec<Slide>) -> Deck {
    Deck {
        version: base.version,
        theme: base.theme.clone(),
        params: base.params.clone(),
        vars: base.vars.clone(),
        tokens: base.tokens.clone(),
        templates: base.templates.clone(),
        slides,
    }
}

/// Même style : même thème, mêmes jetons, paramètres et variables.
fn same_style(a: &Deck, b: &Deck) -> bool {
    a.theme == b.theme && a.tokens == b.tokens && a.params == b.params && a.vars == b.vars
}

/// Un rendu dont l'image absente se dit dans la langue de l'utilisateur.
fn new_renderer(fonts: Fonts, l: &Localizer) -> Result<Renderer> {
    let mut renderer = Renderer::new(fonts).context(RenderSnafu)?;
    renderer.set_language(l).context(RenderSnafu)?;
    Ok(renderer)
}

/// Deck vide : une couverture et une slide de contenu, dans la langue de
/// l'utilisateur.
pub fn blank_deck(l: &Localizer) -> Deck {
    let mut content = Slide::new("s-1", SlideKind::Content);
    content.title = Some(l.tr("deck-new-slide-title"));
    content.blocks.push(Element::text("b-1", vec![Paragraph::plain(l.tr("deck-new-slide-text"))]));
    Deck {
        version: oku_slides_core::DECK_VERSION,
        theme: Some("papier".into()),
        params: Params {
            title: Some(l.tr("deck-new-title")),
            // Code de langue seul (`zh-Hant` → `zh`) : Typst en déduit la
            // direction du texte (droite à gauche en arabe) et les césures.
            lang: l.locale().split('-').next().map(str::to_owned),
            ..Default::default()
        },
        vars: BTreeMap::new(),
        tokens: Value::Null,
        templates: None,
        slides: vec![Slide::new("s-titre", SlideKind::Title), content],
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("path", &self.doc.path)
            .field("slides", &self.doc.sources.deck.slides.len())
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
        let mut s = Self {
            doc: Document::new(SourceBundle::new(blank_deck(l)), None, settings.history.limit),
            mode: Mode::Deck,
            library: Library::new(Some(settings.paths.templates()), storage.clone()),
            storage,
            history_limit: settings.history.limit,
            renderer: new_renderer(app_fonts.clone(), l)?,
            localizer: l.clone(),
            chart_labels: ChartLabels::new(l),
            render_fonts: app_fonts.clone(),
            appearance: Appearance::default(),
            template_renderer: None,
            template_cache: None,
            app_fonts,
            compiled: None,
            svg_cache: HashMap::new(),
        };
        s.rerender()?;
        Ok(s)
    }

    pub fn deck(&self) -> &Deck {
        &self.doc.sources.deck
    }

    /// Le deck de l'utilisateur, même pendant l'édition des modèles.
    fn user_deck(&self) -> &Deck {
        match &self.mode {
            Mode::Deck => &self.doc.sources.deck,
            Mode::Templates { stash, .. } => &stash.sources.deck,
        }
    }

    fn editing(&self) -> Option<&TemplateMode> {
        match &self.mode {
            Mode::Deck => None,
            Mode::Templates { edit, .. } => Some(edit),
        }
    }

    /// Dossier des ensembles de modèles de l'utilisateur.
    pub fn set_templates_dir(&mut self, dir: impl Into<PathBuf>) {
        self.library = Library::new(Some(dir.into()), self.storage.clone());
        self.template_cache = None;
    }

    /// Ensemble de modèles du deck (celui par défaut s'il n'en choisit pas).
    fn deck_set(&self) -> &str {
        self.user_deck().templates.as_deref().unwrap_or(DEFAULT_SET)
    }

    fn get_set(&self, id: &str) -> Result<(Cow<'static, TemplateSet>, bool)> {
        Ok(self.library.get(id).context(NoTemplateSetSnafu { id })?)
    }

    /// Passe en édition d'un ensemble : ses modèles deviennent les slides.
    /// Le deck garde son thème, ses jetons et ses variables ; les modèles ne
    /// portent pas d'assets (un ensemble n'en enregistre pas).
    pub fn edit_templates(&mut self, id: &str) -> Result<()> {
        let (set, builtin) = self.get_set(id)?;
        let set = set.into_owned();
        let slides = set
            .templates
            .into_iter()
            .map(|t| {
                let mut s = t.slide;
                s.id = t.id;
                s.name = Some(t.name);
                s.description = (!t.description.is_empty()).then_some(t.description);
                s
            })
            .collect();
        let mut deck = styled_like(self.user_deck(), slides);
        deck.params.title = Some(set.name.clone());
        deck.templates = Some(set.id.clone());
        let templates = Document::new(SourceBundle::new(deck), None, self.history_limit);
        let edit = TemplateMode { set: set.id, name: set.name, builtin };
        self.mode = match std::mem::replace(&mut self.mode, Mode::Deck) {
            // Déjà en édition : on change d'ensemble, le deck reste de côté.
            Mode::Templates { stash, .. } => {
                self.doc = templates;
                Mode::Templates { stash, edit }
            }
            Mode::Deck => Mode::Templates { stash: Box::new(std::mem::replace(&mut self.doc, templates)), edit },
        };
        self.rerender()?;
        Ok(())
    }

    /// L'ensemble tel qu'il est en cours d'édition.
    fn edited_set(&self, id: String) -> TemplateSet {
        let deck = &self.doc.sources.deck;
        let name = deck
            .params
            .title
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| self.localizer.tr("templates-default-name"));
        let description = self
            .editing()
            .and_then(|m| self.library.get(&m.set))
            .map(|(s, _)| s.into_owned().description)
            .unwrap_or_default();
        let templates = deck
            .slides
            .iter()
            .map(|s| {
                let mut slide = s.clone();
                let name = slide
                    .name
                    .take()
                    .or_else(|| slide.title.clone())
                    .unwrap_or_else(|| self.localizer.tr("template-default-name"));
                let description = slide.description.take().unwrap_or_default();
                slide.id = format!("tpl-{}", s.id);
                Template { id: s.id.clone(), name, description, slide }
            })
            .collect();
        TemplateSet { id, name, description, templates }
    }

    /// Enregistre l'ensemble édité dans la bibliothèque de l'utilisateur.
    pub fn save_templates(&mut self, as_new: bool) -> Result<()> {
        let Mode::Templates { edit: mode, .. } = &self.mode else {
            return NotEditingTemplatesSnafu.fail()?;
        };
        let copy = as_new || mode.builtin;
        let deck = &mut self.doc.sources.deck;
        let mut name = deck.params.title.clone().unwrap_or_default();
        if copy && name == mode.name {
            name = self.localizer.tr_with("templates-copy-name", &[("name", name.as_str().into())]);
            deck.params.title = Some(name.clone());
        }
        let id = if copy { self.library.free_id(&name) } else { mode.set.clone() };
        let set = self.edited_set(id.clone());
        self.library.save(&set)?;
        self.doc.sources.deck.templates = Some(id);
        self.doc.dirty = false;
        if let Mode::Templates { edit, .. } = &mut self.mode {
            *edit = TemplateMode { set: set.id, name: set.name, builtin: false };
        }
        self.template_cache = None;
        self.rerender()?;
        Ok(())
    }

    /// Quitte l'édition des modèles ; les changements non enregistrés sont perdus.
    pub fn close_templates(&mut self) -> Result<()> {
        if let Mode::Templates { stash, .. } = std::mem::replace(&mut self.mode, Mode::Deck) {
            self.doc = *stash;
        }
        self.template_cache = None;
        self.rerender()?;
        Ok(())
    }

    /// Traite une requête de l'interface et renvoie la réponse, en JSON.
    pub fn handle(&mut self, req: Request) -> Result<String> {
        fn json(v: &(impl Serialize + ?Sized)) -> Result<String> {
            Ok(serde_json::to_string(v).context(ResponseSnafu)?)
        }
        if matches!(self.mode, Mode::Templates { .. })
            && matches!(req, Request::New | Request::Open { .. } | Request::Save { .. })
        {
            return EditingTemplatesSnafu.fail()?;
        }
        // Les commandes modifient la session et renvoient le nouvel état ; les
        // requêtes renvoient ce qu'elles lisent.
        match req {
            Request::Snapshot => {}
            Request::Slide { id } => return json(&self.slide(&id)?),
            Request::Svg { id } => return json(&*self.svg(&id)?),
            Request::Panel { slide } => return json(&self.panel(&slide)?),
            Request::Schema => return json(&oku_slides_core::ops_schema().context(ResponseSnafu)?),
            Request::Messages => {
                return json(&serde_json::json!({
                    "locale": self.localizer.locale(),
                    "resources": self.localizer.ui_messages(),
                }));
            }
            Request::Templates { set } => return json(&self.templates_view(set)?),
            Request::TemplateSets => return json(&self.library.list()),
            Request::DeleteTemplateSet { set } => {
                ensure!(!Library::is_builtin(&set), BuiltinTemplateSetSnafu { id: &set });
                self.library.delete(&set)?;
                self.template_cache = None;
                return json(&self.library.list());
            }
            Request::New => self.new_deck()?,
            Request::Open { path } => self.open(&path)?,
            Request::Save { path } => self.save(path.as_deref())?,
            Request::Apply { label, ops } => self.apply(&label, ops)?,
            Request::ChartKind { label, slide, element, kind } => self.set_chart_kind(&label, slide, &element, kind)?,
            Request::Undo => self.undo()?,
            Request::Redo => self.redo()?,
            Request::EditTemplates { set } => self.edit_templates(&set)?,
            Request::SaveTemplates { as_new } => self.save_templates(as_new)?,
            Request::CloseTemplates => self.close_templates()?,
        }
        json(&self.snapshot())
    }

    pub fn new_deck(&mut self) -> Result<()> {
        self.load(SourceBundle::new(blank_deck(&self.localizer)), None)
    }

    /// Charge un deck (déjà lu) dans la session.
    /// Charge un document ; ses graphiques sont mis sous forme canonique,
    /// pour que la grille puisse éditer leurs données.
    pub fn load(&mut self, mut sources: SourceBundle, path: Option<PathBuf>) -> Result<()> {
        charts::canonicalize_deck(&mut sources.deck, &self.chart_labels);
        if sources.fonts != self.doc.sources.fonts {
            // Nouvelles polices : nouveau monde Typst. Les fichiers restent aussi
            // dans les sources (ils sont enregistrés avec le deck), d'où la copie.
            self.render_fonts = self.app_fonts.with(sources.fonts.values().cloned());
            self.renderer = new_renderer(self.render_fonts.clone(), &self.localizer)?;
            self.template_renderer = None;
            self.template_cache = None;
            self.svg_cache.clear();
        }
        self.doc = Document::new(sources, path, self.history_limit);
        self.rerender()
    }

    /// Ouvre un `.oku-slides.pdf` (ou un deck JSON, pour le développement).
    pub fn open(&mut self, path: &Path) -> Result<()> {
        let Opened { sources, pdf, modified_elsewhere } = read_document(&self.storage, path)?;
        // Un deck JSON n'est pas un document Oku Slides : on ne l'écrasera pas.
        self.load(sources, pdf.then(|| path.to_owned()))?;
        tracing::info!(path = %path.display(), pdf, modified_elsewhere, "{}", self.localizer.tr("log-document-opened"));
        self.doc.notice = modified_elsewhere.then(|| self.localizer.tr("notice-modified-elsewhere"));
        Ok(())
    }

    /// Enregistre en `.oku-slides.pdf` : PDF + blob des sources, écriture atomique.
    pub fn save(&mut self, path: Option<&Path>) -> Result<()> {
        let mut path = match path {
            Some(p) => p.to_owned(),
            None => self.doc.path.clone().context(NoPathSnafu)?,
        };
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !name.ends_with(EXTENSION) {
            let stem = name.strip_suffix(".pdf").unwrap_or(&name).to_owned();
            path.set_file_name(format!("{stem}{EXTENSION}"));
        }
        let compiled = self.compiled.as_ref().context(NotRenderedSnafu)?;
        let pdf = compiled.pdf().context(RenderSnafu)?;
        let zip = self.doc.sources.to_zip().context(SourcesSnafu)?;
        let out = oku_slides_pdf::embed_sources(&pdf, &zip).context(PdfSnafu)?;
        let bytes = out.len();
        self.storage.write_atomic(&path, out).context(WriteSnafu { path: &path })?;
        tracing::info!(path = %path.display(), bytes, "{}", self.localizer.tr("log-document-saved"));
        self.doc.path = Some(path);
        self.doc.dirty = false;
        Ok(())
    }

    fn rerender(&mut self) -> Result<()> {
        let started = std::time::Instant::now();
        let compiled = self.renderer.render(&self.doc.sources.deck, &self.doc.sources.assets).context(RenderSnafu)?;
        tracing::debug!(pages = compiled.page_count(), ms = started.elapsed().as_millis(), "rendu");
        // Le cache ne garde que les pages encore présentes.
        let live: HashSet<u128> = (0..compiled.page_count()).filter_map(|i| compiled.page_hash(i)).collect();
        self.svg_cache.retain(|h, _| live.contains(h));
        self.appearance = compiled.layout().tokens.as_ref().map(appearance).unwrap_or_default();
        self.compiled = Some(compiled);
        Ok(())
    }

    /// Applique une transaction ; si le rendu échoue ensuite, elle est annulée.
    /// Les graphiques ajoutés sont d'abord mis sous forme canonique (données
    /// dans `dataset`, en-têtes traduits).
    pub fn apply(&mut self, label: &str, mut ops: Vec<Op>) -> Result<()> {
        charts::canonicalize_charts(&mut ops, &self.chart_labels);
        let doc = &mut self.doc;
        doc.history.commit(&mut doc.sources.deck, label, ops).context(OpSnafu)?;
        if let Err(e) = self.rerender() {
            let doc = &mut self.doc;
            doc.history.undo(&mut doc.sources.deck).ok();
            self.rerender().ok();
            return Err(e);
        }
        self.doc.dirty = true;
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
        let deck = &self.doc.sources.deck;
        let located = deck.slide(&slide).context(NoSlideSnafu { id: &slide })?.locate(element);
        let mut chart = located.map(|l| l.element.clone()).context(NoChartSnafu { id: element })?;
        let oku_slides_core::ElementBody::Chart { option } = &mut chart.body else {
            return Err(NoChartSnafu { id: element }.build().into());
        };
        *option = oku_slides_chart::convert(option, kind, &self.chart_labels).context(ChartSnafu)?;
        self.apply(label, vec![Op::ReplaceElement { slide, element: chart }])
    }

    pub fn undo(&mut self) -> Result<()> {
        let doc = &mut self.doc;
        if doc.history.undo(&mut doc.sources.deck).context(OpSnafu)?.is_some() {
            self.doc.dirty = true;
            self.rerender()?;
        }
        Ok(())
    }

    pub fn redo(&mut self) -> Result<()> {
        let doc = &mut self.doc;
        if doc.history.redo(&mut doc.sources.deck).context(OpSnafu)?.is_some() {
            self.doc.dirty = true;
            self.rerender()?;
        }
        Ok(())
    }

    fn page_of(&self, id: &str) -> Option<usize> {
        self.compiled.as_ref()?.layout().page_of(id)
    }

    pub fn snapshot(&self) -> Snapshot<'_> {
        let compiled = self.compiled.as_ref();
        let (deck, doc) = (&self.doc.sources.deck, &self.doc);
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
                    kind: s.kind,
                    title: s.title.as_deref().map(|t| interp(t, &deck.vars)),
                    name: s.name.as_deref(),
                    page,
                    hash: PageHash(compiled.and_then(|c| c.page_hash(page.checked_sub(1)?))),
                }
            })
            .collect();
        let name = doc
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().trim_end_matches(EXTENSION).to_owned())
            .or_else(|| deck.params.title.as_deref().map(|t| interp(t, &deck.vars)))
            .unwrap_or_else(|| self.localizer.tr("deck-untitled"));
        Snapshot {
            path: doc.path.as_deref(),
            name,
            dirty: doc.dirty,
            theme: deck.theme.as_deref(),
            themes: oku_slides_render::THEMES,
            title: deck.params.title.as_deref().map(|t| interp(t, &deck.vars)),
            title_source: deck.params.title.as_deref(),
            slides,
            can_undo: doc.history.can_undo(),
            undo_label: doc.history.undo_label(),
            can_redo: doc.history.can_redo(),
            redo_label: doc.history.redo_label(),
            warnings: compiled
                .map(|c| c.warnings.iter().map(|w| w.localize(&self.localizer)).collect())
                .unwrap_or_default(),
            notice: doc.notice.as_deref(),
            appearance: &self.appearance,
            templates: self.deck_set(),
            editing: self.editing(),
        }
    }

    /// Aperçus d'un ensemble de modèles, rendus avec le thème, les jetons, les
    /// paramètres et les variables du deck. En cache tant que rien ne change.
    pub fn templates_view(&mut self, set: Option<String>) -> Result<TemplatesView> {
        let id = set.unwrap_or_else(|| self.deck_set().to_owned());
        // Un ensemble disparu (supprimé, ou d'une autre machine) : celui par défaut.
        let (set, builtin) = self.get_set(&id).or_else(|_| self.get_set(DEFAULT_SET))?;
        let base = self.user_deck();
        let views = match &self.template_cache {
            Some(p) if p.set == *set && same_style(&p.deck, base) => p.views.clone(),
            _ => {
                let deck = styled_like(base, set.templates.iter().map(|t| t.slide.clone()).collect());
                let renderer = match &mut self.template_renderer {
                    Some(r) => r,
                    None => self.template_renderer.insert(new_renderer(self.render_fonts.clone(), &self.localizer)?),
                };
                let compiled = renderer.render(&deck, &self.doc.sources.assets).context(RenderSnafu)?;
                let layout = compiled.layout();
                let views = set
                    .templates
                    .iter()
                    .map(|t| {
                        let page = layout.page_of(&t.slide.id).unwrap_or(1);
                        let svg = compiled.svg(page - 1).context(RenderSnafu)?;
                        Ok(TemplateView { template: t.clone(), svg: svg.into() })
                    })
                    .collect::<Result<Arc<[TemplateView]>>>()?;
                self.template_cache = Some(Previews { set: set.clone().into_owned(), deck, views: views.clone() });
                views
            }
        };
        Ok(TemplatesView { set: set.info(builtin), sets: self.library.list(), templates: views })
    }

    /// SVG d'une slide, mis en cache par empreinte de page.
    pub fn svg(&mut self, id: &str) -> Result<Arc<str>> {
        let page = self.page_of(id).context(NoSlideSnafu { id })?;
        let compiled = self.compiled.as_ref().context(NoSlideSnafu { id })?;
        let hash = compiled.page_hash(page - 1).unwrap_or_default();
        if let Some(svg) = self.svg_cache.get(&hash) {
            return Ok(svg.clone());
        }
        let svg: Arc<str> = compiled.svg(page - 1).context(RenderSnafu)?.into();
        self.svg_cache.insert(hash, svg.clone());
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
        let deck = &self.doc.sources.deck;
        let slide = deck.slide(id).ok_or_else(no_slide)?;
        let compiled = self.compiled.as_ref().ok_or_else(no_slide)?;
        let layout = compiled.layout();
        let page = layout.page_of(id).ok_or_else(no_slide)?;
        // Sur la couverture, titre et sous-titre viennent des paramètres du deck.
        let cover = slide.kind == SlideKind::Title;
        let targets = layout
            .boxes_on(page)
            .iter()
            .filter_map(|b| {
                if let Some(field) = b.id.strip_prefix('@') {
                    let text = match field {
                        "title" => slide.title.as_deref().or(cover.then_some(deck.params.title.as_deref()).flatten()),
                        "subtitle" => {
                            slide.subtitle.as_deref().or(cover.then_some(deck.params.subtitle.as_deref()).flatten())
                        }
                        "kicker" => slide.kicker.as_deref(),
                        "text" => slide.text.as_deref(),
                        _ => None,
                    };
                    let text = Cow::Borrowed(text.unwrap_or_default());
                    Some(Target::at(b, TargetKind::Field(field), false, Some(text)))
                } else {
                    let l = slide.locate(&b.id)?;
                    let text = l.element.body.text().map(|t| Cow::Owned(paragraphs_to_text(t)));
                    Some(Target::at(b, TargetKind::Element(l.element.body.kind_name()), l.is_free(), text))
                }
            })
            .collect();
        Ok(SlideView {
            slide,
            page,
            hash: PageHash(compiled.page_hash(page - 1)),
            width: 1920.0,
            height: 1080.0,
            targets,
            zone: layout.zone_on(page),
        })
    }
}
