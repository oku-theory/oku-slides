//! Le monde Typst d'Oku Slides : un système de fichiers virtuel (bibliothèque
//! embarquée, deck, assets) et un jeu de polices fixe, sans polices système.

use std::collections::{BTreeMap, HashMap};

use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};

use oku_slides_core::{Deck, WidgetRegistry};

use snafu::ResultExt;

use oku_slides_chart::ChartLabels;
use oku_slides_i18n::Localizer;

use crate::THEMES;
use crate::charts::{Charts, DEFAULT_SIZE, SVG_FIELD};
use crate::diagnostic::{Diagnostic, Severity};
use crate::error::{DeckJsonSnafu, PathSnafu, RenderError};
use crate::fonts::Fonts;
use crate::registry::PackWidgets;

macro_rules! lib_files {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../../../typst/oku-slides/", $path)))),*]
    };
}

/// La bibliothèque Typst `oku-slides`, embarquée.
const LIBRARY: &[(&str, &str)] = lib_files!(
    "lib.typ",
    "src/tokens.typ",
    "src/core.typ",
    "src/slides.typ",
    "src/content.typ",
    "src/zone.typ",
    "src/params.typ",
    "src/widget.typ",
    "src/widgets.typ",
    "src/data.typ",
    "themes/papier.typ",
    "themes/pro.typ",
);

pub(crate) fn file_id(path: &str) -> Result<FileId, RenderError> {
    let vpath = VirtualPath::new(path).context(PathSnafu { path })?;
    Ok(FileId::new(RootedPath::new(VirtualRoot::Project, vpath)))
}

/// Source principale : importe la bibliothèque et les widgets (base, puis
/// ceux du pack s'il y en a un), et rend `/deck.json`.
/// `assets` : empreintes (hex) des assets disponibles.
/// `labels` : textes des cadres de repli (échappés ici, pas du code).
fn main_source<'a>(
    theme: Option<&str>,
    pack: Option<&PackWidgets>,
    assets: impl Iterator<Item = &'a str>,
    labels: &Labels,
) -> String {
    let theme = match theme {
        Some(t) if THEMES.contains(&t) => t,
        _ => "none",
    };
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
      image("/assets/" + hex, width: 100%, height: 100%, fit: fit)
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
    /// Dernier deck reçu : thème et assets, pour refaire la source principale.
    theme: Option<String>,
    hexes: Vec<String>,
    labels: Labels,
    /// Widgets du pack de modèles, servis sous `/packs/<id>/widgets.typ`.
    pack: Option<PackWidgets>,
    charts: Charts,
    /// Assets par fichier (`/assets/<hex>`). Leur identifiant est l'empreinte
    /// de leur contenu : un asset déjà là n'est jamais recopié.
    assets: HashMap<FileId, Bytes>,
}

impl OkuSlidesWorld {
    pub fn new(fonts: Fonts) -> Result<Self, RenderError> {
        let mut sources = HashMap::new();
        for (path, text) in LIBRARY {
            let id = file_id(&format!("/oku-slides/{path}"))?;
            sources.insert(id, Source::new(id, (*text).to_owned()));
        }
        let main_id = file_id("/main.typ")?;
        Ok(Self {
            library: LazyHash::new(Library::default()),
            fonts,
            main: Source::new(main_id, main_source(None, None, std::iter::empty(), &Labels::default())),
            sources,
            deck_id: file_id("/deck.json")?,
            deck: Bytes::new(br#"{"slides":[]}"#.as_slice()),
            theme: None,
            hexes: Vec::new(),
            labels: Labels::default(),
            pack: None,
            charts: Charts::default(),
            assets: HashMap::new(),
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
        self.charts.attach(&mut json, deck.theme.as_deref(), &self.fonts, registry)?;
        self.deck = Bytes::new(serde_json::to_vec(&json).context(DeckJsonSnafu)?);

        self.theme.clone_from(&deck.theme);
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
            self.sources.remove(&file_id(&old.path())?);
        }
        if let Some(p) = pack {
            self.add_source(&p.path(), p.source().to_owned())?;
        }
        self.pack = pack.cloned();
        self.redraw_charts(registry)
    }

    /// Redessine les graphiques du deck courant et refait la source principale.
    fn redraw_charts(&mut self, registry: &WidgetRegistry) -> Result<(), RenderError> {
        let mut json: serde_json::Value = serde_json::from_slice(&self.deck).context(DeckJsonSnafu)?;
        self.charts.attach(&mut json, self.theme.as_deref(), &self.fonts, registry)?;
        self.deck = Bytes::new(serde_json::to_vec(&json).context(DeckJsonSnafu)?);
        self.refresh_main();
        Ok(())
    }

    /// Palette des graphiques (jetons du thème) ; s'ils changent, les
    /// graphiques du deck courant sont redessinés. `true` si elle a changé.
    pub(crate) fn set_chart_palette(
        &mut self,
        palette: Vec<String>,
        registry: &WidgetRegistry,
    ) -> Result<bool, RenderError> {
        if !self.charts.set_palette(palette) {
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
            main_source(self.theme.as_deref(), self.pack.as_ref(), self.hexes.iter().map(String::as_str), &self.labels);
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

    /// Un message de Typst, avec sa place (`fichier:ligne`).
    pub(crate) fn diagnostic(&self, d: &SourceDiagnostic) -> Diagnostic {
        let place = d.span.id().and_then(|id| {
            let range = self.range(d.span)?;
            let source = self.source_of(id)?;
            let line = source.lines().byte_to_line(range.start)? + 1;
            Some(format!("{}:{line}", id.vpath().get_without_slash()))
        });
        Diagnostic {
            severity: match d.severity {
                typst::diag::Severity::Error => Severity::Error,
                typst::diag::Severity::Warning => Severity::Warning,
            },
            place,
            message: d.message.to_string(),
            hints: d.hints.iter().map(|h| h.v.to_string()).collect(),
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
        if let Some(b) = self.assets.get(&id).or_else(|| self.charts.file(id)) {
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
