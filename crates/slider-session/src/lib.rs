//! Session d'édition : le deck ouvert, son historique, son rendu et son
//! fichier. L'app Tauri et le serveur de développement exposent la même API
//! (voir [`Request`] et [`Session::handle`]).

use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use slider_core::{Deck, Element, History, Op, Paragraph, Params, Slide, SlideKind, SourceBundle};
use slider_render::{Compiled, Renderer};
pub use slider_render::{Fonts, read_font_dir};

mod appearance;
pub mod templates;
mod text;
mod views;

pub use appearance::{Appearance, appearance};
pub use templates::{DEFAULT_SET, Library, SetInfo, Template, TemplateSet};
pub use text::{interp, paragraphs_to_text};
pub use views::{
    PageHash, SlideSummary, SlideView, Snapshot, Target, TargetKind, TemplateMode, TemplateView, TemplatesView,
};

/// Identifiant de l'app : même dossier que `app_data_dir` de Tauri.
pub const APP_ID: &str = "me.dartiguenave.echo";

/// Dossier de données de l'app (`ECHO_DATA` le remplace) : modèles, réglages.
pub fn default_data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ECHO_DATA") {
        return dir.into();
    }
    let home =
        std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_default();
    let base = if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| home.join("AppData/Roaming"))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"))
    };
    base.join(APP_ID)
}

pub const EXTENSION: &str = ".echo.pdf";

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Sources(#[from] slider_core::SourcesError),
    #[error("{0}")]
    Pdf(#[from] slider_pdf::PdfError),
    #[error("{0}")]
    Render(#[from] slider_render::RenderError),
    #[error("opération {index} refusée : {message}")]
    Op { index: usize, message: String },
    #[error("ce PDF n'a pas été créé par Écho, il ne peut pas être modifié")]
    NotSlider,
    #[error("aucun fichier : choisissez où enregistrer")]
    NoPath,
    #[error("slide introuvable : {0}")]
    NoSlide(String),
    #[error("requête invalide : {0}")]
    BadRequest(String),
    #[error("réponse impossible à écrire en JSON : {0}")]
    Json(#[from] serde_json::Error),
}

impl Serialize for SessionError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SessionError>;

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
    fn new(sources: SourceBundle, path: Option<PathBuf>) -> Self {
        Self { sources, history: History::default(), path, dirty: false, notice: None }
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
    /// Rendu des aperçus de modèles, séparé du deck, avec leur clé de cache.
    template_renderer: Option<Renderer>,
    template_cache: Option<(String, Vec<TemplateView>)>,
    renderer: Renderer,
    /// Polices livrées avec l'app, et celles du rendu (app + deck).
    app_fonts: Fonts,
    render_fonts: Fonts,
    /// Apparence tirée du dernier rendu.
    appearance: Appearance,
    compiled: Option<Compiled>,
    svg_cache: HashMap<u128, Arc<str>>,
}

/// Deck vide : une couverture et une slide de contenu.
pub fn blank_deck() -> Deck {
    let mut content = Slide::new("s-1", SlideKind::Content);
    content.title = Some("Titre de la slide".into());
    content.blocks.push(Element::text("b-1", vec![Paragraph::plain("Dites ce que vous voulez montrer.")]));
    Deck {
        version: slider_core::DECK_VERSION,
        theme: Some("papier".into()),
        params: Params { title: Some("Nouvelle présentation".into()), ..Default::default() },
        vars: BTreeMap::new(),
        tokens: Value::Null,
        templates: None,
        slides: vec![Slide::new("s-titre", SlideKind::Title), content],
    }
}

fn op_error((index, e): (usize, slider_core::OpError)) -> SessionError {
    SessionError::Op { index, message: e.to_string() }
}

impl Session {
    /// `app_fonts` : polices livrées avec l'app, toujours disponibles.
    pub fn new(app_fonts: Fonts) -> Self {
        let mut s = Self {
            doc: Document::new(SourceBundle::new(blank_deck()), None),
            mode: Mode::Deck,
            library: Library::default(),
            renderer: Renderer::new(app_fonts.clone()),
            render_fonts: app_fonts.clone(),
            appearance: Appearance::default(),
            template_renderer: None,
            template_cache: None,
            app_fonts,
            compiled: None,
            svg_cache: HashMap::new(),
        };
        s.rerender().ok();
        s
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
        self.library = Library::new(Some(dir.into()));
        self.template_cache = None;
    }

    /// Ensemble de modèles du deck (celui par défaut s'il n'en choisit pas).
    fn deck_set(&self) -> &str {
        self.user_deck().templates.as_deref().unwrap_or(DEFAULT_SET)
    }

    fn get_set(&self, id: &str) -> Result<(TemplateSet, bool)> {
        self.library.get(id).ok_or_else(|| SessionError::BadRequest(format!("ensemble de modèles introuvable : {id}")))
    }

    /// Passe en édition d'un ensemble : ses modèles deviennent les slides.
    /// Le deck garde son thème, ses jetons et ses variables ; les modèles ne
    /// portent pas d'assets (un ensemble n'en enregistre pas).
    pub fn edit_templates(&mut self, id: &str) -> Result<()> {
        let (set, builtin) = self.get_set(id)?;
        let base = self.user_deck();
        let deck = Deck {
            version: base.version,
            theme: base.theme.clone(),
            params: Params { title: Some(set.name.clone()), ..base.params.clone() },
            vars: base.vars.clone(),
            tokens: base.tokens.clone(),
            templates: Some(set.id.clone()),
            slides: set
                .templates
                .into_iter()
                .map(|t| {
                    let mut s = t.slide;
                    s.id = t.id;
                    s.name = Some(t.name);
                    s.description = (!t.description.is_empty()).then_some(t.description);
                    s
                })
                .collect(),
        };
        let templates = Document::new(SourceBundle::new(deck), None);
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
        let name = deck.params.title.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Mes modèles".into());
        let description =
            self.editing().and_then(|m| self.library.get(&m.set)).map(|(s, _)| s.description).unwrap_or_default();
        let templates = deck
            .slides
            .iter()
            .map(|s| {
                let mut slide = s.clone();
                let name = slide.name.take().or_else(|| slide.title.clone()).unwrap_or_else(|| "Modèle".into());
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
            return Err(SessionError::BadRequest("pas de modèles en cours d'édition".into()));
        };
        let copy = as_new || mode.builtin;
        let deck = &mut self.doc.sources.deck;
        let mut name = deck.params.title.clone().unwrap_or_default();
        if copy && name == mode.name {
            name = format!("{name} (copie)");
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
            Ok(serde_json::to_string(v)?)
        }
        if matches!(self.mode, Mode::Templates { .. })
            && matches!(req, Request::New | Request::Open { .. } | Request::Save { .. })
        {
            return Err(SessionError::BadRequest("terminez d'abord l'édition des modèles".into()));
        }
        // Les commandes modifient la session et renvoient le nouvel état ; les
        // requêtes renvoient ce qu'elles lisent.
        match req {
            Request::Snapshot => {}
            Request::Slide { id } => return json(&self.slide(&id)?),
            Request::Svg { id } => return json(&*self.svg(&id)?),
            Request::Schema => return json(&slider_core::ops_schema()),
            Request::Templates { set } => return json(&self.templates_view(set)?),
            Request::TemplateSets => return json(&self.library.list()),
            Request::DeleteTemplateSet { set } => {
                if Library::is_builtin(&set) {
                    return Err(SessionError::BadRequest("un ensemble intégré ne se supprime pas".into()));
                }
                self.library.delete(&set)?;
                self.template_cache = None;
                return json(&self.library.list());
            }
            Request::New => self.new_deck()?,
            Request::Open { path } => self.open(&path)?,
            Request::Save { path } => self.save(path.as_deref())?,
            Request::Apply { label, ops } => self.apply(&label, ops)?,
            Request::Undo => self.undo()?,
            Request::Redo => self.redo()?,
            Request::EditTemplates { set } => self.edit_templates(&set)?,
            Request::SaveTemplates { as_new } => self.save_templates(as_new)?,
            Request::CloseTemplates => self.close_templates()?,
        }
        json(&self.snapshot())
    }

    pub fn new_deck(&mut self) -> Result<()> {
        self.load(SourceBundle::new(blank_deck()), None)
    }

    /// Charge un deck (déjà lu) dans la session.
    pub fn load(&mut self, sources: SourceBundle, path: Option<PathBuf>) -> Result<()> {
        if sources.fonts != self.doc.sources.fonts {
            // Nouvelles polices : nouveau monde Typst. Les fichiers restent aussi
            // dans les sources (ils sont enregistrés avec le deck), d'où la copie.
            self.render_fonts = self.app_fonts.with(sources.fonts.values().cloned());
            self.renderer = Renderer::new(self.render_fonts.clone());
            self.template_renderer = None;
            self.template_cache = None;
            self.svg_cache.clear();
        }
        self.doc = Document::new(sources, path);
        self.rerender()
    }

    /// Ouvre un `.echo.pdf` (ou un deck JSON, pour le développement).
    pub fn open(&mut self, path: &Path) -> Result<()> {
        let bytes = std::fs::read(path)?;
        let is_pdf = bytes.starts_with(b"%PDF");
        let (bundle, notice, keep_path) = if is_pdf {
            let sources = slider_pdf::extract_sources(&bytes)?.ok_or(SessionError::NotSlider)?;
            let notice = sources
                .modified_elsewhere()
                .then(|| "Ce PDF a été modifié par un autre outil après Écho : ses sources peuvent ne plus correspondre à ce qui s'affiche dans un lecteur PDF.".to_owned());
            (SourceBundle::from_zip(&sources.zip)?, notice, true)
        } else {
            let deck = Deck::from_json(&String::from_utf8_lossy(&bytes))
                .map_err(|e| SessionError::BadRequest(format!("deck JSON invalide : {e}")))?;
            (SourceBundle::new(deck), None, false)
        };
        self.load(bundle, keep_path.then(|| path.to_owned()))?;
        self.doc.notice = notice;
        Ok(())
    }

    /// Enregistre en `.echo.pdf` : PDF + blob des sources, écriture atomique.
    pub fn save(&mut self, path: Option<&Path>) -> Result<()> {
        let mut path = match path {
            Some(p) => p.to_owned(),
            None => self.doc.path.clone().ok_or(SessionError::NoPath)?,
        };
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !name.ends_with(EXTENSION) {
            let stem = name.strip_suffix(".pdf").unwrap_or(&name).to_owned();
            path.set_file_name(format!("{stem}{EXTENSION}"));
        }
        let compiled = self.compiled.as_ref().ok_or_else(|| SessionError::BadRequest("rendu indisponible".into()))?;
        let pdf = compiled.pdf()?;
        let out = slider_pdf::embed_sources(&pdf, &self.doc.sources.to_zip()?)?;
        write_atomic(&path, &out)?;
        self.doc.path = Some(path);
        self.doc.dirty = false;
        Ok(())
    }

    fn rerender(&mut self) -> Result<()> {
        self.renderer.set_deck(&self.doc.sources.deck, &self.doc.sources.assets);
        let compiled = self.renderer.compile()?;
        // Le cache ne garde que les pages encore présentes.
        let live: HashSet<u128> = (0..compiled.page_count()).filter_map(|i| compiled.page_hash(i)).collect();
        self.svg_cache.retain(|h, _| live.contains(h));
        self.appearance = compiled.layout().tokens.as_ref().map(appearance).unwrap_or_default();
        self.compiled = Some(compiled);
        Ok(())
    }

    /// Applique une transaction ; si le rendu échoue ensuite, elle est annulée.
    pub fn apply(&mut self, label: &str, ops: Vec<Op>) -> Result<()> {
        let doc = &mut self.doc;
        doc.history.commit(&mut doc.sources.deck, label, ops).map_err(op_error)?;
        if let Err(e) = self.rerender() {
            let doc = &mut self.doc;
            doc.history.undo(&mut doc.sources.deck).ok();
            self.rerender().ok();
            return Err(e);
        }
        self.doc.dirty = true;
        Ok(())
    }

    pub fn undo(&mut self) -> Result<()> {
        let doc = &mut self.doc;
        if doc.history.undo(&mut doc.sources.deck).map_err(op_error)?.is_some() {
            self.doc.dirty = true;
            self.rerender()?;
        }
        Ok(())
    }

    pub fn redo(&mut self) -> Result<()> {
        let doc = &mut self.doc;
        if doc.history.redo(&mut doc.sources.deck).map_err(op_error)?.is_some() {
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
            .unwrap_or_else(|| "Sans titre".into());
        Snapshot {
            path: doc.path.as_deref(),
            name,
            dirty: doc.dirty,
            theme: deck.theme.as_deref(),
            themes: slider_render::THEMES,
            title: deck.params.title.as_deref().map(|t| interp(t, &deck.vars)),
            title_source: deck.params.title.as_deref(),
            slides,
            can_undo: doc.history.can_undo(),
            undo_label: doc.history.undo_label(),
            can_redo: doc.history.can_redo(),
            redo_label: doc.history.redo_label(),
            warnings: compiled.map(|c| c.warnings.as_slice()).unwrap_or_default(),
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
        let base = self.user_deck().clone();
        let key =
            serde_json::to_string(&(&base.theme, &base.tokens, &base.params, &base.vars, &set)).unwrap_or_default();
        let views = match &self.template_cache {
            Some((k, views)) if *k == key => views.clone(),
            _ => {
                let mut preview = base;
                preview.slides = set.templates.iter().map(|t| t.slide.clone()).collect();
                let renderer = self.template_renderer.get_or_insert_with(|| Renderer::new(self.render_fonts.clone()));
                renderer.set_deck(&preview, &self.doc.sources.assets);
                let compiled = renderer.compile()?;
                let layout = compiled.layout();
                let mut views = Vec::with_capacity(set.templates.len());
                for t in &set.templates {
                    let page = layout.page_of(&t.slide.id).unwrap_or(1);
                    let svg: Arc<str> = compiled.svg(page - 1)?.into();
                    views.push(TemplateView { template: t.clone(), svg });
                }
                self.template_cache = Some((key, views.clone()));
                views
            }
        };
        Ok(TemplatesView { set: set.info(builtin), sets: self.library.list(), templates: views })
    }

    /// SVG d'une slide, mis en cache par empreinte de page.
    pub fn svg(&mut self, id: &str) -> Result<Arc<str>> {
        let page = self.page_of(id).ok_or_else(|| SessionError::NoSlide(id.to_owned()))?;
        let compiled = self.compiled.as_ref().ok_or_else(|| SessionError::NoSlide(id.to_owned()))?;
        let hash = compiled.page_hash(page - 1).unwrap_or_default();
        if let Some(svg) = self.svg_cache.get(&hash) {
            return Ok(svg.clone());
        }
        let svg: Arc<str> = compiled.svg(page - 1)?.into();
        self.svg_cache.insert(hash, svg.clone());
        Ok(svg)
    }

    /// Détail d'une slide : son contenu et ses boîtes sélectionnables.
    pub fn slide(&self, id: &str) -> Result<SlideView<'_>> {
        let no_slide = || SessionError::NoSlide(id.to_owned());
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

/// Écrit dans un fichier temporaire à côté, puis renomme.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let tmp = dir.join(format!(".{}.tmp", path.file_name().unwrap_or_default().to_string_lossy()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}
