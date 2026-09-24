//! Session d'édition : le deck ouvert, son historique, son rendu et son
//! fichier. L'app Tauri et le serveur de développement exposent la même API
//! (voir [`Request`] et [`Session::handle`]).

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use slider_core::{Deck, Element, ElementBody, History, Op, Paragraph, Params, Slide, SlideKind, SourceBundle};
use slider_render::{Compiled, ElementBox, Renderer};

pub mod templates;
pub use templates::{Template, templates};

pub const EXTENSION: &str = ".echo.pdf";
/// Extension des fichiers enregistrés sous l'ancien nom de l'app.
pub const LEGACY_EXTENSION: &str = ".slider.pdf";

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
}

#[derive(Debug, Clone, Serialize)]
pub struct SlideView {
    pub slide: Slide,
    pub page: usize,
    pub hash: String,
    pub width: f64,
    pub height: f64,
    pub targets: Vec<Target>,
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
    /// Modèles de slides, rendus avec le thème du deck.
    Templates,
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

/// Inverse de [`paragraphs_to_text`]. Les paragraphes inchangés gardent leur mise en forme.
pub fn text_to_paragraphs(text: &str, previous: &[Paragraph]) -> Vec<Paragraph> {
    let numbered = |l: &str| {
        let digits = l.chars().take_while(char::is_ascii_digit).count();
        (digits > 0 && l[digits..].starts_with(". ")).then(|| l[digits + 2..].to_owned())
    };
    text.lines()
        .map(|line| {
            let (list, body) = if let Some(rest) = line.strip_prefix("- ") {
                (Some(slider_core::ListKind::Bullet), rest.to_owned())
            } else if let Some(rest) = numbered(line) {
                (Some(slider_core::ListKind::Number), rest)
            } else {
                (None, line.to_owned())
            };
            let same = previous
                .iter()
                .find(|p| p.list == list && p.runs.iter().map(|r| r.t.as_str()).collect::<String>() == body);
            match same {
                Some(p) => p.clone(),
                None => Paragraph { runs: vec![slider_core::Run::plain(body)], list },
            }
        })
        .collect()
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
    /// Rendu des aperçus de modèles, séparé du deck, avec leur clé de cache.
    template_renderer: Option<Renderer>,
    template_cache: Option<(String, Vec<TemplateView>)>,
    deck: Deck,
    assets: BTreeMap<String, Vec<u8>>,
    fonts: BTreeMap<String, Vec<u8>>,
    history: History,
    renderer: Renderer,
    app_fonts: Vec<Vec<u8>>,
    compiled: Option<Compiled>,
    svg_cache: HashMap<u128, Arc<str>>,
    path: Option<PathBuf>,
    dirty: bool,
    notice: Option<String>,
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

fn hex(h: u128) -> String {
    format!("{h:032x}")
}

impl Session {
    /// `app_fonts` : polices livrées avec l'app, toujours disponibles.
    pub fn new(app_fonts: Vec<Vec<u8>>) -> Self {
        let mut s = Self {
            deck: blank_deck(),
            assets: BTreeMap::new(),
            fonts: BTreeMap::new(),
            history: History::default(),
            renderer: Renderer::new(app_fonts.clone()),
            template_renderer: None,
            template_cache: None,
            app_fonts,
            compiled: None,
            svg_cache: HashMap::new(),
            path: None,
            dirty: false,
            notice: None,
        };
        s.rerender().ok();
        s
    }

    pub fn deck(&self) -> &Deck {
        &self.deck
    }

    /// Traite une requête de l'interface et renvoie la réponse en JSON.
    pub fn handle(&mut self, req: Request) -> Result<Value> {
        let v = |x: &dyn erased::Ser| x.to_value();
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
            Request::Templates => v(&self.template_views()?),
        })
    }

    pub fn new_deck(&mut self) -> Result<Snapshot> {
        self.load(SourceBundle::new(blank_deck()), None)?;
        Ok(self.snapshot())
    }

    /// Charge un deck (déjà lu) dans la session.
    pub fn load(&mut self, bundle: SourceBundle, path: Option<PathBuf>) -> Result<()> {
        let SourceBundle { deck, assets, fonts, .. } = bundle;
        self.deck = deck.ok_or(SessionError::NotSlider)?;
        self.assets = assets;
        if fonts != self.fonts {
            // Nouvelles polices : nouveau monde Typst.
            let mut all = self.app_fonts.clone();
            all.extend(fonts.values().cloned());
            self.renderer = Renderer::new(all);
            self.template_renderer = None;
            self.template_cache = None;
            self.svg_cache.clear();
        }
        self.fonts = fonts;
        self.history = History::default();
        self.path = path;
        self.dirty = false;
        self.notice = None;
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
        self.notice = notice;
        Ok(self.snapshot())
    }

    /// Enregistre en `.echo.pdf` : PDF + blob des sources, écriture atomique.
    pub fn save(&mut self, path: Option<&Path>) -> Result<Snapshot> {
        let mut path = match path {
            Some(p) => p.to_owned(),
            None => self.path.clone().ok_or(SessionError::NoPath)?,
        };
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !name.ends_with(EXTENSION) {
            let stem =
                name.strip_suffix(LEGACY_EXTENSION).or_else(|| name.strip_suffix(".pdf")).unwrap_or(&name).to_owned();
            path.set_file_name(format!("{stem}{EXTENSION}"));
        }
        let compiled = self.compiled.as_ref().ok_or_else(|| SessionError::BadRequest("rendu indisponible".into()))?;
        let pdf = compiled.pdf()?;
        let bundle = SourceBundle {
            deck: Some(self.deck.clone()),
            assets: self.assets.clone(),
            fonts: self.fonts.clone(),
            history: None,
        };
        let out = slider_pdf::embed_sources(&pdf, &bundle.to_zip()?)?;
        write_atomic(&path, &out)?;
        self.path = Some(path);
        self.dirty = false;
        Ok(self.snapshot())
    }

    fn rerender(&mut self) -> Result<()> {
        self.renderer.set_deck(&self.deck, &self.assets);
        let compiled = self.renderer.compile()?;
        // Le cache ne garde que les pages encore présentes.
        let live: Vec<u128> = (0..compiled.page_count()).filter_map(|i| compiled.page_hash(i)).collect();
        self.svg_cache.retain(|h, _| live.contains(h));
        self.compiled = Some(compiled);
        Ok(())
    }

    /// Applique une transaction ; si le rendu échoue ensuite, elle est annulée.
    pub fn apply(&mut self, label: &str, ops: Vec<Op>) -> Result<Snapshot> {
        self.history
            .commit(&mut self.deck, label, ops)
            .map_err(|(index, e)| SessionError::Op { index, message: e.to_string() })?;
        if let Err(e) = self.rerender() {
            self.history.undo(&mut self.deck).ok();
            self.rerender().ok();
            return Err(e);
        }
        self.dirty = true;
        Ok(self.snapshot())
    }

    pub fn undo(&mut self) -> Result<Snapshot> {
        if self
            .history
            .undo(&mut self.deck)
            .map_err(|(index, e)| SessionError::Op { index, message: e.to_string() })?
            .is_some()
        {
            self.dirty = true;
            self.rerender()?;
        }
        Ok(self.snapshot())
    }

    pub fn redo(&mut self) -> Result<Snapshot> {
        if self
            .history
            .redo(&mut self.deck)
            .map_err(|(index, e)| SessionError::Op { index, message: e.to_string() })?
            .is_some()
        {
            self.dirty = true;
            self.rerender()?;
        }
        Ok(self.snapshot())
    }

    fn page_of(&self, id: &str) -> Option<usize> {
        let c = self.compiled.as_ref()?;
        c.slide_pages().into_iter().find(|p| p.slide == id).map(|p| p.page)
    }

    pub fn snapshot(&self) -> Snapshot {
        let compiled = self.compiled.as_ref();
        let pages = compiled.map(|c| c.slide_pages()).unwrap_or_default();
        let slides = self
            .deck
            .slides
            .iter()
            .map(|s| {
                let page = pages.iter().find(|p| p.slide == s.id).map(|p| p.page).unwrap_or(0);
                let hash = compiled.and_then(|c| c.page_hash(page.saturating_sub(1))).map(hex).unwrap_or_default();
                let title = s.title.as_deref().map(|t| interp(t, &self.deck.vars));
                SlideSummary { id: s.id.clone(), kind: s.kind, title, page, hash }
            })
            .collect();
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().trim_end_matches(EXTENSION).trim_end_matches(LEGACY_EXTENSION).to_owned())
            .or_else(|| self.deck.params.title.as_deref().map(|t| interp(t, &self.deck.vars)))
            .unwrap_or_else(|| "Sans titre".into());
        Snapshot {
            path: self.path.clone(),
            name,
            dirty: self.dirty,
            theme: self.deck.theme.clone(),
            themes: slider_render::THEMES.iter().map(|t| t.to_string()).collect(),
            title: self.deck.params.title.as_deref().map(|t| interp(t, &self.deck.vars)),
            title_source: self.deck.params.title.clone(),
            slides,
            can_undo: self.history.can_undo(),
            undo_label: self.history.undo_label().map(str::to_owned),
            can_redo: self.history.can_redo(),
            redo_label: self.history.redo_label().map(str::to_owned),
            warnings: compiled.map(|c| c.warnings.clone()).unwrap_or_default(),
            notice: self.notice.clone(),
            appearance: compiled.and_then(|c| c.tokens()).map(|t| appearance(&t)).unwrap_or_default(),
        }
    }

    /// Aperçus des modèles, rendus avec le thème, les jetons, les paramètres et
    /// les variables du deck. Mis en cache tant que ceux-ci ne changent pas.
    pub fn template_views(&mut self) -> Result<Vec<TemplateView>> {
        let key = serde_json::to_string(&(&self.deck.theme, &self.deck.tokens, &self.deck.params, &self.deck.vars))
            .unwrap_or_default();
        if let Some((k, views)) = &self.template_cache
            && *k == key
        {
            return Ok(views.clone());
        }
        let list = templates();
        let mut preview = self.deck.clone();
        preview.slides = list.iter().map(|t| t.slide.clone()).collect();
        let renderer = self.template_renderer.get_or_insert_with(|| {
            let mut all = self.app_fonts.clone();
            all.extend(self.fonts.values().cloned());
            Renderer::new(all)
        });
        renderer.set_deck(&preview, &self.assets);
        let compiled = renderer.compile()?;
        let pages = compiled.slide_pages();
        let mut views = Vec::with_capacity(list.len());
        for t in list {
            let page = pages.iter().find(|p| p.slide == t.slide.id).map(|p| p.page).unwrap_or(1);
            let svg: Arc<str> = compiled.svg(page - 1)?.into();
            views.push(TemplateView { template: t, svg });
        }
        self.template_cache = Some((key, views.clone()));
        Ok(views)
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
        let slide = self.deck.slide(id).ok_or_else(|| SessionError::NoSlide(id.to_owned()))?.clone();
        let page = self.page_of(id).ok_or_else(|| SessionError::NoSlide(id.to_owned()))?;
        let compiled = self.compiled.as_ref().ok_or_else(|| SessionError::NoSlide(id.to_owned()))?;
        let boxes: Vec<ElementBox> = compiled.element_boxes().into_iter().filter(|b| b.page == page).collect();

        let mut targets = Vec::new();
        for b in boxes {
            if let Some(field) = b.id.strip_prefix('@') {
                let text = match field {
                    "title" => slide
                        .title
                        .clone()
                        .or_else(|| (slide.kind == SlideKind::Title).then(|| self.deck.params.title.clone()).flatten()),
                    "subtitle" => slide.subtitle.clone().or_else(|| {
                        (slide.kind == SlideKind::Title).then(|| self.deck.params.subtitle.clone()).flatten()
                    }),
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
                });
            } else if let Some((e, free)) = find_element(&slide, &b.id) {
                let (kind, text) = match &e.body {
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
                });
            }
        }
        let hash = compiled.page_hash(page - 1).map(hex).unwrap_or_default();
        Ok(SlideView { slide, page, hash, width: 1920.0, height: 1080.0, targets })
    }
}

/// Élément par id, avec « placé librement » (dans `elements` ou un groupe libre).
fn find_element<'a>(slide: &'a Slide, id: &str) -> Option<(&'a Element, bool)> {
    fn rec<'a>(list: &'a [Element], id: &str, free: bool) -> Option<(&'a Element, bool)> {
        for e in list {
            if e.id == id {
                return Some((e, free && e.frame.is_some()));
            }
            if let ElementBody::Group { layout, children } = &e.body {
                let child_free = layout.kind == slider_core::GroupLayoutKind::Free;
                if let Some(found) = rec(children, id, child_free) {
                    return Some(found);
                }
            }
        }
        None
    }
    rec(&slide.blocks, id, false).or_else(|| rec(&slide.elements, id, true))
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
