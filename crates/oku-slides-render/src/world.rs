//! Le monde Typst d'Oku Slides : un système de fichiers virtuel (bibliothèque
//! embarquée, deck, assets) et un jeu de polices fixe, sans polices système.

use std::collections::{BTreeMap, HashMap};

use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{DiagSpan, FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};

use oku_slides_core::{Deck, WidgetRegistry};

use snafu::ResultExt;

use oku_slides_chart::ChartLabels;
use oku_slides_i18n::Localizer;

use crate::charts::{Charts, DEFAULT_SIZE, SVG_FIELD, text_sizes};
use crate::diagnostic::{Diagnostic, Severity, SourceAt};
use crate::error::{DeckJsonSnafu, PathSnafu, RenderError};
use crate::fonts::Fonts;
use crate::palette::{body_font, chart_palette};
use crate::registry::PackWidgets;

macro_rules! lib_files {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../../../typst/oku-slides/", $path)))),*]
    };
}

/// Les sources Typst de la bibliothèque `oku-slides`, embarquées.
const LIBRARY: &[(&str, &str)] = lib_files!(
    "lib.typ",
    "src/tokens.typ",
    "src/core.typ",
    "src/slides.typ",
    "src/content.typ",
    "src/zone.typ",
    "src/numbers.typ",
    "src/params.typ",
    "src/widget.typ",
    "src/widgets.typ",
    "src/data.typ",
);

/// Les données JSON de la bibliothèque (jetons par défaut, thèmes intégrés), lues par `json()`.
const LIBRARY_DATA: &[(&str, &[u8])] = &[
    ("src/default-tokens.json", include_bytes!("../../../typst/oku-slides/src/default-tokens.json")),
    ("themes/papier.json", include_bytes!("../../../typst/oku-slides/themes/papier.json")),
    ("themes/pro.json", include_bytes!("../../../typst/oku-slides/themes/pro.json")),
];

pub(crate) fn file_id(path: &str) -> Result<FileId, RenderError> {
    let vpath = VirtualPath::new(path).context(PathSnafu { path })?;
    Ok(FileId::new(RootedPath::new(VirtualRoot::Project, vpath)))
}

/// Source principale : importe la bibliothèque et les widgets (base, puis
/// ceux du pack s'il y en a un), et rend `/deck.json` avec les jetons de
/// `/theme.json` si un thème est posé (`has_theme`).
/// `assets` : empreintes (hex) des assets disponibles.
/// `labels` : textes des cadres de repli (échappés ici, pas du code).
fn main_source<'a>(
    has_theme: bool,
    pack: Option<&PackWidgets>,
    assets: impl Iterator<Item = &'a str>,
    labels: &Labels,
) -> String {
    let theme = if has_theme { r#"json("/theme.json")"# } else { "none" };
    // Un tableau Typst à un élément s'écrit `("x",)` : chaque entrée finit par une virgule.
    let mut available = String::from("(");
    for hex in assets {
        available.push('"');
        available.push_str(hex);
        available.push_str("\",");
    }
    available.push(')');
    let missing = typst_string(&labels.missing_image);
    let chart_failed = typst_string(&labels.chart_failed);
    // L'identifiant d'un pack est contrôlé (`PackWidgets::new`) : il peut
    // entrer dans un chemin.
    let (pack_import, pack_registry) = match pack {
        Some(p) => (format!("#import \"{}\": registry as pack-registry\n", p.path()), " + pack-registry"),
        None => (String::new(), ""),
    };
    format!(
        r#"#import "/oku-slides/lib.typ": *
#import "/oku-slides/src/widgets.typ": registry as base-registry
{pack_import}#let available = {available}
#let fallback(label) = rect(width: 100%, height: 100%, fill: luma(230),
  align(center + horizon, text(size: 20pt, fill: luma(90), label)))
#let load-image(e, asset: none, fit: "cover", chart: false) = {{
  if chart {{
    if "{svg_field}" not in e {{ fallback({chart_failed}) }}
    else if "frame" in e {{
      image("/charts/" + e.at("{svg_field}") + ".svg", width: 100%, height: 100%, fit: "contain")
    }} else {{
      // Dans le flux : toute la largeur, sans dépasser la hauteur disponible.
      layout(size => {{
        let h = size.width * {aspect}
        if size.height < h {{ h = size.height }}
        image("/charts/" + e.at("{svg_field}") + ".svg", width: 100%, height: h, fit: "contain")
      }})
    }}
  }} else {{
    let hex = if asset == none {{ "" }} else {{ asset.trim("sha256:", at: start) }}
    if hex in available {{
      let path = "/assets/" + hex
      if e.at("kind", default: none) == "image" and "frame" not in e {{
        // Dans le flux : toute la largeur, à ses proportions, sans dépasser la hauteur disponible.
        layout(size => {{
          let h = measure(image(path, width: size.width)).height
          if size.height < h {{ h = size.height }}
          image(path, width: 100%, height: h, fit: fit)
        }})
      }} else {{ image(path, width: 100%, height: 100%, fit: fit) }}
    }} else {{ fallback({missing}) }}
  }}
}}
#render-deck(json("/deck.json"), theme: {theme}, image-loader: load-image,
  widgets: by-name(base-registry{pack_registry}))
"#,
        svg_field = SVG_FIELD,
        aspect = DEFAULT_SIZE.height / DEFAULT_SIZE.width,
    )
}

/// Textes des cadres de repli, dans la langue de l'utilisateur.
#[derive(Debug)]
struct Labels {
    missing_image: String,
    chart_failed: String,
}

impl Default for Labels {
    fn default() -> Self {
        let l = oku_slides_i18n::english();
        Self { missing_image: l.tr("render-missing-image"), chart_failed: l.tr("render-chart-failed") }
    }
}

/// Littéral de chaîne Typst : guillemets et barres obliques échappés.
fn typst_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Empreinte (hex) d'un identifiant d'asset `sha256:<hex>`. Tout autre
/// identifiant est refusé : il deviendrait un chemin et un littéral Typst.
fn asset_hex(id: &str) -> Option<&str> {
    let hex = id.strip_prefix("sha256:").unwrap_or(id);
    (!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hex)
}

pub(crate) struct OkuSlidesWorld {
    library: LazyHash<Library>,
    fonts: Fonts,
    /// Source principale, recréée quand le thème ou les assets changent.
    main: Source,
    /// Bibliothèque `oku-slides`, embarquée.
    sources: HashMap<FileId, Source>,
    deck_id: FileId,
    deck: Bytes,
    /// Jetons du thème (JSON), servis sous `/theme.json` ; `None` : pas de thème.
    theme: Option<Bytes>,
    theme_id: FileId,
    /// Assets du dernier deck reçu, pour refaire la source principale.
    hexes: Vec<String>,
    labels: Labels,
    /// Widgets du pack : ses fichiers sous /packs/<id>/widgets/, l'assemblage sous /packs/<id>/widgets.typ.
    pack: Option<PackWidgets>,
    charts: Charts,
    /// Assets par fichier (`/assets/<hex>`). Leur identifiant est l'empreinte
    /// de leur contenu : un asset déjà là n'est jamais recopié.
    assets: HashMap<FileId, Bytes>,
    /// Données JSON de la bibliothèque, par fichier.
    data: HashMap<FileId, Bytes>,
}

impl OkuSlidesWorld {
    pub fn new(fonts: Fonts) -> Result<Self, RenderError> {
        let mut sources = HashMap::new();
        for (path, text) in LIBRARY {
            let id = file_id(&format!("/oku-slides/{path}"))?;
            sources.insert(id, Source::new(id, (*text).to_owned()));
        }
        let mut data = HashMap::new();
        for (path, bytes) in LIBRARY_DATA {
            data.insert(file_id(&format!("/oku-slides/{path}"))?, Bytes::new(*bytes));
        }
        let main_id = file_id("/main.typ")?;
        Ok(Self {
            library: LazyHash::new(Library::default()),
            fonts,
            main: Source::new(main_id, main_source(false, None, std::iter::empty(), &Labels::default())),
            sources,
            deck_id: file_id("/deck.json")?,
            deck: Bytes::new(br#"{"slides":[]}"#.as_slice()),
            theme: None,
            theme_id: file_id("/theme.json")?,
            hexes: Vec::new(),
            labels: Labels::default(),
            pack: None,
            charts: Charts::default(),
            assets: HashMap::new(),
            data,
        })
    }

    /// Remplace le deck et les assets. La bibliothèque et les assets déjà
    /// chargés restent en place ; un identifiant d'asset invalide est ignoré.
    pub fn set_deck(
        &mut self,
        deck: &Deck,
        assets: &BTreeMap<String, Vec<u8>>,
        registry: &WidgetRegistry,
    ) -> Result<(), RenderError> {
        let mut hexes = Vec::with_capacity(assets.len());
        let mut wanted: HashMap<FileId, &Vec<u8>> = HashMap::with_capacity(assets.len());
        for (id, bytes) in assets {
            let Some(hex) = asset_hex(id) else {
                tracing::warn!(asset = %id, "identifiant d'asset refusé");
                continue;
            };
            wanted.insert(file_id(&format!("/assets/{hex}"))?, bytes);
            hexes.push(hex);
        }
        self.assets.retain(|id, _| wanted.contains_key(id));
        for (id, bytes) in wanted {
            self.assets.entry(id).or_insert_with(|| Bytes::new(bytes.clone()));
        }
        let mut json = serde_json::to_value(deck).context(DeckJsonSnafu)?;
        crate::tables::sanitize(&mut json, registry);
        self.charts.attach(&mut json, &self.fonts, registry)?;
        self.deck = Bytes::new(serde_json::to_vec(&json).context(DeckJsonSnafu)?);

        self.hexes = hexes.into_iter().map(str::to_owned).collect();
        self.refresh_main();
        Ok(())
    }

    /// Textes des cadres de repli et libellés des graphiques ; les
    /// graphiques du deck courant sont redessinés.
    pub fn set_language(&mut self, l: &Localizer, registry: &WidgetRegistry) -> Result<(), RenderError> {
        self.labels = Labels { missing_image: l.tr("render-missing-image"), chart_failed: l.tr("render-chart-failed") };
        self.charts.set_labels(ChartLabels::new(l));
        self.redraw_charts(registry)
    }

    /// Widgets du pack (ou aucun) : sa source est servie, la source
    /// principale l'importe, et les graphiques du deck courant sont
    /// redessinés selon `registry`.
    pub(crate) fn set_pack(
        &mut self,
        pack: Option<&PackWidgets>,
        registry: &WidgetRegistry,
    ) -> Result<(), RenderError> {
        if let Some(old) = self.pack.take() {
            for (path, _) in old.sources() {
                self.sources.remove(&file_id(&path)?);
            }
        }
        if let Some(p) = pack {
            for (path, text) in p.sources() {
                self.add_source(&path, text)?;
            }
        }
        self.pack = pack.cloned();
        self.redraw_charts(registry)
    }

    /// Redessine les graphiques du deck courant et refait la source principale.
    fn redraw_charts(&mut self, registry: &WidgetRegistry) -> Result<(), RenderError> {
        let mut json: serde_json::Value = serde_json::from_slice(&self.deck).context(DeckJsonSnafu)?;
        self.charts.attach(&mut json, &self.fonts, registry)?;
        self.deck = Bytes::new(serde_json::to_vec(&json).context(DeckJsonSnafu)?);
        self.refresh_main();
        Ok(())
    }

    /// Jetons du thème (ou aucun) ; les graphiques du deck courant sont redessinés.
    pub(crate) fn set_theme(
        &mut self,
        tokens: Option<serde_json::Map<String, serde_json::Value>>,
        registry: &WidgetRegistry,
    ) -> Result<(), RenderError> {
        self.theme = match tokens {
            Some(map) => Some(Bytes::new(serde_json::to_vec(&map).context(DeckJsonSnafu)?)),
            None => None,
        };
        self.redraw_charts(registry)
    }

    /// Ce qu'une compilation dit des graphiques : les jetons résolus du
    /// thème (palette, police `font.body`, tailles de texte) et les boîtes
    /// `(élément, largeur, hauteur)`. Si l'un d'eux change pour un graphique,
    /// les graphiques du deck courant sont redessinés ; `true` dans ce cas.
    pub(crate) fn fit_charts<'a>(
        &mut self,
        tokens: Option<&serde_json::Value>,
        boxes: impl IntoIterator<Item = (&'a str, f64, f64)>,
        registry: &WidgetRegistry,
    ) -> Result<bool, RenderError> {
        let palette = tokens.map(chart_palette).unwrap_or_default();
        let font = tokens.and_then(body_font);
        let repainted = self.charts.set_palette(palette)
            | self.charts.set_font(font)
            | self.charts.set_text_sizes(tokens.and_then(text_sizes));
        let resized = self.charts.set_measured(boxes);
        if !(repainted || resized) {
            return Ok(false);
        }
        self.redraw_charts(registry)?;
        Ok(true)
    }

    pub(crate) fn chart_palette(&self) -> &[String] {
        self.charts.palette()
    }

    /// Refait la source principale si elle a changé (Typst garde son cache sinon).
    fn refresh_main(&mut self) {
        let text =
            main_source(self.theme.is_some(), self.pack.as_ref(), self.hexes.iter().map(String::as_str), &self.labels);
        if self.main.text() != text {
            self.main.replace(&text);
        }
    }

    pub(crate) fn source_of(&self, id: FileId) -> Option<&Source> {
        if id == self.main.id() { Some(&self.main) } else { self.sources.get(&id) }
    }

    /// Remplace la source principale (sonde du registre).
    pub(crate) fn set_main_text(&mut self, text: &str) {
        if self.main.text() != text {
            self.main.replace(text);
        }
    }

    /// Ajoute une source Typst au système de fichiers virtuel.
    pub(crate) fn add_source(&mut self, path: &str, text: String) -> Result<(), RenderError> {
        let id = file_id(path)?;
        self.sources.insert(id, Source::new(id, text));
        Ok(())
    }

    /// Fichier, ligne et colonne d'un emplacement de source, s'il est connu.
    pub(crate) fn source_at(&self, span: impl Into<DiagSpan>) -> Option<SourceAt> {
        let span = span.into();
        let id = span.id()?;
        let range = self.range(span)?;
        let source = self.source_of(id)?;
        let (line, column) = source.lines().byte_to_line_column(range.start)?;
        Some(SourceAt { path: id.vpath().get_without_slash().to_string(), line: line + 1, column: column + 1 })
    }

    /// Un message de Typst, avec sa place (`fichier:ligne`) et sa colonne.
    pub(crate) fn diagnostic(&self, d: &SourceDiagnostic) -> Diagnostic {
        let at = self.source_at(d.span);
        Diagnostic {
            severity: match d.severity {
                typst::diag::Severity::Error => Severity::Error,
                typst::diag::Severity::Warning => Severity::Warning,
            },
            place: at.as_ref().map(|a| format!("{}:{}", a.path, a.line)),
            message: d.message.to_string(),
            hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
            at,
        }
    }
}

impl World for OkuSlidesWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.source_of(id).cloned().ok_or_else(|| FileError::NotFound(id.vpath().get_with_slash().into()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.deck_id {
            return Ok(self.deck.clone());
        }
        if id == self.theme_id
            && let Some(theme) = &self.theme
        {
            return Ok(theme.clone());
        }
        if let Some(b) = self.assets.get(&id).or_else(|| self.charts.file(id)) {
            return Ok(b.clone());
        }
        if let Some(b) = self.data.get(&id) {
            return Ok(b.clone());
        }
        if let Some(s) = self.source_of(id) {
            return Ok(Bytes::from_string(s.text().to_owned()));
        }
        Err(FileError::NotFound(id.vpath().get_with_slash().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let utc = time::OffsetDateTime::now_utc();
        let date = match offset {
            // Heure locale si le système la donne, sinon UTC.
            None => time::OffsetDateTime::now_local().unwrap_or(utc).date(),
            Some(o) => (utc + time::Duration::seconds_f64(o.hours() * 3600.0)).date(),
        };
        Datetime::from_ymd(date.year(), u8::from(date.month()), date.day())
    }
}

#[cfg(test)]
#[path = "world_tests.rs"]
mod world_tests;
