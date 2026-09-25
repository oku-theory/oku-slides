//! Le monde Typst d'Écho : un système de fichiers virtuel (bibliothèque
//! embarquée, deck, assets) et un jeu de polices fixe, sans polices système.

use std::collections::{BTreeMap, HashMap};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use slider_core::Deck;

use snafu::ResultExt;

use crate::THEMES;
use crate::error::{DeckJsonSnafu, PathSnafu, RenderError};
use crate::fonts::Fonts;

macro_rules! lib_files {
    ($($path:literal),* $(,)?) => {
        &[$(($path, include_str!(concat!("../../../typst/slider/", $path)))),*]
    };
}

/// La bibliothèque Typst `slider`, embarquée.
const LIBRARY: &[(&str, &str)] = lib_files!(
    "lib.typ",
    "src/tokens.typ",
    "src/core.typ",
    "src/slides.typ",
    "src/data.typ",
    "themes/papier.typ",
    "themes/pro.typ",
);

fn file_id(path: &str) -> Result<FileId, RenderError> {
    let vpath = VirtualPath::new(path).context(PathSnafu { path })?;
    Ok(FileId::new(RootedPath::new(VirtualRoot::Project, vpath)))
}

/// Source principale : importe la bibliothèque et rend `/deck.json`.
/// `assets` : empreintes (hex) des assets disponibles.
fn main_source<'a>(theme: Option<&str>, assets: impl Iterator<Item = &'a str>) -> String {
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
    format!(
        r#"#import "/slider/lib.typ": *
#let available = {available}
#let load-image(e) = {{
  let hex = e.asset.trim("sha256:", at: start)
  if hex in available {{
    image("/assets/" + hex, width: 100%, height: 100%, fit: e.at("fit", default: "cover"))
  }} else {{
    rect(width: 100%, height: 100%, fill: luma(230),
      align(center + horizon, text(size: 20pt, fill: luma(90))[image manquante]))
  }}
}}
#render-deck(json("/deck.json"), theme: {theme}, image-loader: load-image)
"#
    )
}

/// Empreinte (hex) d'un identifiant d'asset `sha256:<hex>`. Tout autre
/// identifiant est refusé : il deviendrait un chemin et un littéral Typst.
fn asset_hex(id: &str) -> Option<&str> {
    let hex = id.strip_prefix("sha256:").unwrap_or(id);
    (!hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hex)
}

pub(crate) struct SliderWorld {
    library: LazyHash<Library>,
    fonts: Fonts,
    /// Source principale, recréée quand le thème ou les assets changent.
    main: Source,
    /// Bibliothèque `slider`, embarquée.
    sources: HashMap<FileId, Source>,
    deck_id: FileId,
    deck: Bytes,
    /// Assets par fichier (`/assets/<hex>`). Leur identifiant est l'empreinte
    /// de leur contenu : un asset déjà là n'est jamais recopié.
    assets: HashMap<FileId, Bytes>,
}

impl SliderWorld {
    pub fn new(fonts: Fonts) -> Result<Self, RenderError> {
        let mut sources = HashMap::new();
        for (path, text) in LIBRARY {
            let id = file_id(&format!("/slider/{path}"))?;
            sources.insert(id, Source::new(id, (*text).to_owned()));
        }
        let main_id = file_id("/main.typ")?;
        Ok(Self {
            library: LazyHash::new(Library::default()),
            fonts,
            main: Source::new(main_id, main_source(None, std::iter::empty())),
            sources,
            deck_id: file_id("/deck.json")?,
            deck: Bytes::new(br#"{"slides":[]}"#.as_slice()),
            assets: HashMap::new(),
        })
    }

    /// Remplace le deck et les assets. La bibliothèque et les assets déjà
    /// chargés restent en place ; un identifiant d'asset invalide est ignoré.
    pub fn set_deck(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) -> Result<(), RenderError> {
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
        self.deck = Bytes::new(serde_json::to_vec(deck).context(DeckJsonSnafu)?);

        let text = main_source(deck.theme.as_deref(), hexes.into_iter());
        if self.main.text() != text {
            self.main.replace(&text);
        }
        Ok(())
    }

    pub(crate) fn source_of(&self, id: FileId) -> Option<&Source> {
        if id == self.main.id() { Some(&self.main) } else { self.sources.get(&id) }
    }
}

impl World for SliderWorld {
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
        if let Some(b) = self.assets.get(&id) {
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
