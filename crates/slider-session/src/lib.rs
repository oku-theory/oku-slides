//! Session d'édition : le deck ouvert, son historique, son rendu et son
//! fichier. L'app Tauri et le serveur de développement exposent la même API
//! (voir [`Request`] et [`Session::handle`]).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use slider_core::{Deck, Element, ElementBody, History, Op, Paragraph, Params, Slide, SlideKind, SourceBundle};
use slider_render::{Compiled, Renderer};
pub use slider_render::{Fonts, read_font_dir};

pub mod templates;
pub use templates::{DEFAULT_SET, Library, SetInfo, Template, TemplateSet};

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
}

impl Serialize for SessionError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, SessionError>;

// ---- Vues envoyées à l'interface ------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct SlideSummary {
    pub id: String,
    pub kind: SlideKind,
    pub title: Option<String>,
    /// Nom de la slide quand elle sert de modèle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Page dans le document, à partir de 1.
    pub page: usize,
    /// Empreinte du rendu : clé de cache du SVG.
    pub hash: String,
}

/// Apparence de l'interface, tirée des jetons du thème des slides.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Appearance {
    /// `light` ou `dark` : contrôles natifs, barres de défilement.
    pub scheme: String,
    /// Couleurs des slides (fond, encre, accent…), résolues en hex.
    pub slide: BTreeMap<String, String>,
    /// Couleurs de l'interface (`desk`, `sheet`, `piece`, `ink`…). Une couleur
    /// absente est dérivée par l'interface à partir des couleurs des slides.
    pub app: BTreeMap<String, String>,
    /// Formes de l'interface : `radius-sm`, `radius-md`, `radius-lg` (px), `elevation`.
    pub metrics: BTreeMap<String, f64>,
    /// Familles de police : titres et texte courant.
    pub font_display: Vec<String>,
    pub font_body: Vec<String>,
}

/// Couleur : hex, ou nom d'une couleur du thème (références suivies).
fn resolve_color(colors: &serde_json::Map<String, Value>, v: &Value) -> Option<String> {
    let mut v = v.as_str()?;
    for _ in 0..8 {
        if v.starts_with('#') {
            return Some(v.to_ascii_lowercase());
        }
        v = colors.get(v)?.as_str()?;
    }
    None
}

fn token_fonts(fonts: &serde_json::Map<String, Value>, name: &str) -> Vec<String> {
    let mut v = fonts.get(name);
    for _ in 0..8 {
        match v {
            Some(Value::String(s)) if fonts.contains_key(s) => v = fonts.get(s),
            Some(Value::String(s)) => return vec![s.clone()],
            Some(Value::Array(a)) => return a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect(),
            _ => break,
        }
    }
    Vec::new()
}

/// Apparence à partir des jetons résolus par Typst.
pub fn appearance(tokens: &Value) -> Appearance {
    let empty = serde_json::Map::new();
    let colors = tokens.get("color").and_then(Value::as_object).unwrap_or(&empty);
    let fonts = tokens.get("font").and_then(Value::as_object).unwrap_or(&empty);
    let app = tokens.get("app").and_then(Value::as_object).unwrap_or(&empty);
    let slide = colors.iter().filter_map(|(k, v)| Some((k.clone(), resolve_color(colors, v)?))).collect();
    let app_colors = app
        .iter()
        .filter(|(k, _)| *k != "scheme")
        .filter_map(|(k, v)| Some((k.clone(), resolve_color(colors, v)?)))
        .collect();
    let metrics = app.iter().filter_map(|(k, v)| Some((k.clone(), v.as_f64()?))).collect();
    Appearance {
        metrics,
        scheme: app.get("scheme").and_then(Value::as_str).unwrap_or("light").to_owned(),
        slide,
        app: app_colors,
        font_display: token_fonts(fonts, "display"),
        font_body: token_fonts(fonts, "body"),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub path: Option<PathBuf>,
    pub name: String,
    pub dirty: bool,
    pub theme: Option<String>,
    pub themes: Vec<String>,
    pub title: Option<String>,
    /// Titre tel que saisi (avec ses {{variables}}), pour l'éditer.
    pub title_source: Option<String>,
    pub slides: Vec<SlideSummary>,
    pub can_undo: bool,
    pub undo_label: Option<String>,
    pub can_redo: bool,
    pub redo_label: Option<String>,
    pub warnings: Vec<String>,
    /// Signalé à l'ouverture : le PDF a été modifié par un autre outil.
    pub notice: Option<String>,
    /// L'interface prend les couleurs et les polices du thème des slides.
    pub appearance: Appearance,
    /// Ensemble de modèles du deck.
    pub templates: String,
    /// Mode d'édition des modèles : l'ensemble en cours de modification.
    pub editing: Option<TemplateMode>,
}

/// Édition d'un ensemble de modèles : chaque slide est un modèle.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateMode {
    pub set: String,
    pub name: String,
    /// Ensemble intégré : l'enregistrer en crée une copie.
    pub builtin: bool,
}

/// Aperçus d'un ensemble de modèles, et la liste des ensembles.
#[derive(Debug, Clone, Serialize)]
pub struct TemplatesView {
    pub set: SetInfo,
    pub sets: Vec<SetInfo>,
    pub templates: Vec<TemplateView>,
}

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

/// Boîte sélectionnable : un élément du deck ou un champ de slide (`@title`…).
#[derive(Debug, Clone, Serialize)]
pub struct Target {
    pub id: String,
    /// `element` ou `field`.
    pub target: &'static str,
    /// Type d'élément (`text`, `card`, `image`…) ou nom du champ.
    pub kind: String,
    /// Placé librement (déplaçable, redimensionnable).
    pub free: bool,
    /// Texte éditable, un paragraphe par ligne (`- ` en tête pour une puce).
    pub text: Option<String>,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Zone disponible pour le texte : début et largeur (contient la boîte).
    pub ax: f64,
    pub aw: f64,
    /// Typographie du rendu, pour un éditeur calé sur le texte.
    pub style: Option<slider_render::TextStyle>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlideView {
    pub slide: Slide,
    pub page: usize,
    pub hash: String,
    pub width: f64,
    pub height: f64,
    pub targets: Vec<Target>,
    /// Zone des éléments libres (pt, page), avec la grille résolue.
    pub zone: Option<slider_render::ZoneBox>,
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

// ---- Conversion texte ↔ paragraphes ------------------------------------------------

/// Texte éditable : un paragraphe par ligne, `- ` pour une puce, `1. ` pour une liste numérotée.
pub fn paragraphs_to_text(paras: &[Paragraph]) -> String {
    let mut n = 0;
    paras
        .iter()
        .map(|p| {
            let t: String = p.runs.iter().map(|r| r.t.as_str()).collect();
            match p.list {
                Some(slider_core::ListKind::Bullet) => format!("- {t}"),
                Some(slider_core::ListKind::Number) => {
                    n += 1;
                    format!("{n}. {t}")
                }
                None => t,
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ---- Session ----------------------------------------------------------------------

/// Un modèle avec son aperçu (SVG) dans le thème courant.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateView {
    #[serde(flatten)]
    pub template: Template,
    pub svg: Arc<str>,
}

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

/// Remplace `{{nom}}` par la variable du deck (affichage dans l'interface).
pub fn interp(s: &str, vars: &BTreeMap<String, Value>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("{{") {
        let Some(len) = rest[start + 2..].find("}}") else { break };
        let key = rest[start + 2..start + 2 + len].trim();
        out.push_str(&rest[..start]);
        match vars.get(key) {
            Some(Value::String(v)) => out.push_str(v),
            Some(v) => out.push_str(&v.to_string()),
            None => out.push_str(&rest[start..start + len + 4]),
        }
        rest = &rest[start + len + 4..];
    }
    out.push_str(rest);
    out
}

fn op_error((index, e): (usize, slider_core::OpError)) -> SessionError {
    SessionError::Op { index, message: e.to_string() }
}

fn hex(h: u128) -> String {
    format!("{h:032x}")
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
    pub fn edit_templates(&mut self, id: &str) -> Result<Snapshot> {
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
        Ok(self.snapshot())
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
    pub fn save_templates(&mut self, as_new: bool) -> Result<Snapshot> {
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
        Ok(self.snapshot())
    }

    /// Quitte l'édition des modèles ; les changements non enregistrés sont perdus.
    pub fn close_templates(&mut self) -> Result<Snapshot> {
        if let Mode::Templates { stash, .. } = std::mem::replace(&mut self.mode, Mode::Deck) {
            self.doc = *stash;
        }
        self.template_cache = None;
        self.rerender()?;
        Ok(self.snapshot())
    }

    /// Traite une requête de l'interface et renvoie la réponse en JSON.
    pub fn handle(&mut self, req: Request) -> Result<Value> {
        let v = |x: &dyn erased::Ser| x.to_value();
        if matches!(self.mode, Mode::Templates { .. })
            && matches!(req, Request::New | Request::Open { .. } | Request::Save { .. })
        {
            return Err(SessionError::BadRequest("terminez d'abord l'édition des modèles".into()));
        }
        Ok(match req {
            Request::Snapshot => v(&self.snapshot()),
            Request::New => v(&self.new_deck()?),
            Request::Open { path } => v(&self.open(&path)?),
            Request::Save { path } => v(&self.save(path.as_deref())?),
            Request::Slide { id } => v(&self.slide(&id)?),
            Request::Svg { id } => Value::String(self.svg(&id)?.to_string()),
            Request::Apply { label, ops } => v(&self.apply(&label, ops)?),
            Request::Undo => v(&self.undo()?),
            Request::Redo => v(&self.redo()?),
            Request::Schema => slider_core::ops_schema(),
            Request::Templates { set } => v(&self.templates_view(set)?),
            Request::TemplateSets => v(&self.library.list()),
            Request::EditTemplates { set } => v(&self.edit_templates(&set)?),
            Request::SaveTemplates { as_new } => v(&self.save_templates(as_new)?),
            Request::CloseTemplates => v(&self.close_templates()?),
            Request::DeleteTemplateSet { set } => {
                if Library::is_builtin(&set) {
                    return Err(SessionError::BadRequest("un ensemble intégré ne se supprime pas".into()));
                }
                self.library.delete(&set)?;
                self.template_cache = None;
                v(&self.library.list())
            }
        })
    }

    pub fn new_deck(&mut self) -> Result<Snapshot> {
        self.load(SourceBundle::new(blank_deck()), None)?;
        Ok(self.snapshot())
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
    pub fn open(&mut self, path: &Path) -> Result<Snapshot> {
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
        Ok(self.snapshot())
    }

    /// Enregistre en `.echo.pdf` : PDF + blob des sources, écriture atomique.
    pub fn save(&mut self, path: Option<&Path>) -> Result<Snapshot> {
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
        Ok(self.snapshot())
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
    pub fn apply(&mut self, label: &str, ops: Vec<Op>) -> Result<Snapshot> {
        let doc = &mut self.doc;
        doc.history.commit(&mut doc.sources.deck, label, ops).map_err(op_error)?;
        if let Err(e) = self.rerender() {
            let doc = &mut self.doc;
            doc.history.undo(&mut doc.sources.deck).ok();
            self.rerender().ok();
            return Err(e);
        }
        self.doc.dirty = true;
        Ok(self.snapshot())
    }

    pub fn undo(&mut self) -> Result<Snapshot> {
        let doc = &mut self.doc;
        if doc.history.undo(&mut doc.sources.deck).map_err(op_error)?.is_some() {
            self.doc.dirty = true;
            self.rerender()?;
        }
        Ok(self.snapshot())
    }

    pub fn redo(&mut self) -> Result<Snapshot> {
        let doc = &mut self.doc;
        if doc.history.redo(&mut doc.sources.deck).map_err(op_error)?.is_some() {
            self.doc.dirty = true;
            self.rerender()?;
        }
        Ok(self.snapshot())
    }

    fn page_of(&self, id: &str) -> Option<usize> {
        self.compiled.as_ref()?.layout().page_of(id)
    }

    pub fn snapshot(&self) -> Snapshot {
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
                let hash = compiled.and_then(|c| c.page_hash(page.saturating_sub(1))).map(hex).unwrap_or_default();
                let title = s.title.as_deref().map(|t| interp(t, &deck.vars));
                SlideSummary { id: s.id.clone(), kind: s.kind, title, name: s.name.clone(), page, hash }
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
            path: doc.path.clone(),
            name,
            dirty: doc.dirty,
            theme: deck.theme.clone(),
            themes: slider_render::THEMES.iter().map(|t| t.to_string()).collect(),
            title: deck.params.title.as_deref().map(|t| interp(t, &deck.vars)),
            title_source: deck.params.title.clone(),
            slides,
            can_undo: doc.history.can_undo(),
            undo_label: doc.history.undo_label().map(str::to_owned),
            can_redo: doc.history.can_redo(),
            redo_label: doc.history.redo_label().map(str::to_owned),
            warnings: compiled.map(|c| c.warnings.clone()).unwrap_or_default(),
            notice: doc.notice.clone(),
            appearance: self.appearance.clone(),
            templates: self.deck_set().to_owned(),
            editing: self.editing().cloned(),
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
    pub fn slide(&self, id: &str) -> Result<SlideView> {
        let deck = &self.doc.sources.deck;
        let slide = deck.slide(id).ok_or_else(|| SessionError::NoSlide(id.to_owned()))?.clone();
        let page = self.page_of(id).ok_or_else(|| SessionError::NoSlide(id.to_owned()))?;
        let compiled = self.compiled.as_ref().ok_or_else(|| SessionError::NoSlide(id.to_owned()))?;
        let layout = compiled.layout();

        let mut targets = Vec::new();
        for b in layout.boxes_on(page) {
            if let Some(field) = b.id.strip_prefix('@') {
                let text = match field {
                    "title" => slide
                        .title
                        .clone()
                        .or_else(|| (slide.kind == SlideKind::Title).then(|| deck.params.title.clone()).flatten()),
                    "subtitle" => slide
                        .subtitle
                        .clone()
                        .or_else(|| (slide.kind == SlideKind::Title).then(|| deck.params.subtitle.clone()).flatten()),
                    "kicker" => slide.kicker.clone(),
                    "text" => slide.text.clone(),
                    _ => None,
                };
                targets.push(Target {
                    id: b.id.clone(),
                    target: "field",
                    kind: field.to_owned(),
                    free: false,
                    text: Some(text.unwrap_or_default()),
                    x: b.x,
                    y: b.y,
                    w: b.w,
                    h: b.h,
                    ax: b.ax.unwrap_or(b.x).min(b.x),
                    aw: b.aw.unwrap_or(b.w).max(b.w),
                    style: b.style.clone(),
                });
            } else if let Some(l) = slide.locate(&b.id) {
                let free = l.is_free();
                let (kind, text) = match &l.element.body {
                    ElementBody::Text { text } => ("text", Some(paragraphs_to_text(text))),
                    ElementBody::Card { text, .. } => ("card", Some(paragraphs_to_text(text))),
                    ElementBody::Image { .. } => ("image", None),
                    ElementBody::Shape { .. } => ("shape", None),
                    ElementBody::Group { .. } => ("group", None),
                };
                targets.push(Target {
                    id: b.id.clone(),
                    target: "element",
                    kind: kind.to_owned(),
                    free,
                    text,
                    x: b.x,
                    y: b.y,
                    w: b.w,
                    h: b.h,
                    ax: b.ax.unwrap_or(b.x).min(b.x),
                    aw: b.aw.unwrap_or(b.w).max(b.w),
                    style: b.style.clone(),
                });
            }
        }
        let hash = compiled.page_hash(page - 1).map(hex).unwrap_or_default();
        let zone = layout.zone_on(page).cloned();
        Ok(SlideView { slide, page, hash, width: 1920.0, height: 1080.0, targets, zone })
    }
}

/// Écrit dans un fichier temporaire à côté, puis renomme.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let tmp = dir.join(format!(".{}.tmp", path.file_name().unwrap_or_default().to_string_lossy()));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Sérialisation sans générique, pour `handle`.
mod erased {
    pub trait Ser {
        fn to_value(&self) -> serde_json::Value;
    }
    impl<T: serde::Serialize> Ser for T {
        fn to_value(&self) -> serde_json::Value {
            serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
        }
    }
}
