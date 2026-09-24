//! Le monde Typst d'Écho : un système de fichiers virtuel (bibliothèque
//! embarquée, deck, assets) et un jeu de polices fixe, sans polices système.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use chrono::Datelike;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use slider_core::Deck;

use crate::THEMES;

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

fn file_id(path: &str) -> FileId {
    let vpath = VirtualPath::new(path).expect("chemin virtuel valide");
    FileId::new(RootedPath::new(VirtualRoot::Project, vpath))
}

/// Source principale : importe la bibliothèque et rend `/deck.json`.
fn main_source(theme: Option<&str>, assets: &[String]) -> String {
    let theme = match theme {
        Some(t) if THEMES.contains(&t) => t,
        _ => "none",
    };
    let available = if assets.is_empty() {
        "()".to_owned()
    } else {
        format!("({},)", assets.iter().map(|h| format!("\"{h}\"")).collect::<Vec<_>>().join(", "))
    };
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

pub struct SliderWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    sources: HashMap<FileId, Source>,
    files: HashMap<FileId, Bytes>,
    /// Sources recréées à chaque changement ; protégées pour `source()` (&self).
    cache: Mutex<()>,
}

impl SliderWorld {
    pub fn new(extra_fonts: impl IntoIterator<Item = Vec<u8>>) -> Self {
        let mut fonts: Vec<Font> = typst_assets::fonts().flat_map(|data| Font::iter(Bytes::new(data))).collect();
        for data in extra_fonts {
            fonts.extend(Font::iter(Bytes::new(data)));
        }
        let book = FontBook::from_fonts(&fonts);

        let mut sources = HashMap::new();
        for (path, text) in LIBRARY {
            let id = file_id(&format!("/slider/{path}"));
            sources.insert(id, Source::new(id, (*text).to_owned()));
        }
        let main = file_id("/main.typ");
        sources.insert(main, Source::new(main, main_source(None, &[])));

        let mut files = HashMap::new();
        files.insert(file_id("/deck.json"), Bytes::from_string(r#"{"slides":[]}"#.to_owned()));

        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main,
            sources,
            files,
            cache: Mutex::new(()),
        }
    }

    /// Remplace le deck et les assets. La bibliothèque reste en cache.
    pub fn set_deck(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) {
        let _guard = self.cache.lock().expect("verrou");
        self.files.retain(|id, _| !id.vpath().get_with_slash().starts_with("/assets/"));
        let mut hexes = Vec::new();
        for (asset, bytes) in assets {
            let hex = asset.strip_prefix("sha256:").unwrap_or(asset).to_owned();
            self.files.insert(file_id(&format!("/assets/{hex}")), Bytes::new(bytes.clone()));
            hexes.push(hex);
        }
        self.files.insert(file_id("/deck.json"), Bytes::from_string(deck.to_json()));

        let text = main_source(deck.theme.as_deref(), &hexes);
        let main = self.sources.get_mut(&self.main).expect("source principale");
        if main.text() != text {
            main.replace(&text);
        }
    }

    pub fn font_count(&self) -> usize {
        self.fonts.len()
    }

    pub(crate) fn source_of(&self, id: FileId) -> Option<&Source> {
        self.sources.get(&id)
    }
}

impl World for SliderWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.sources.get(&id).cloned().ok_or_else(|| FileError::NotFound(id.vpath().get_with_slash().into()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(b) = self.files.get(&id) {
            return Ok(b.clone());
        }
        if let Some(s) = self.sources.get(&id) {
            return Ok(Bytes::from_string(s.text().to_owned()));
        }
        Err(FileError::NotFound(id.vpath().get_with_slash().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        let now = chrono::Local::now();
        let date = match offset {
            None => now.date_naive(),
            Some(o) => (now.naive_utc() + chrono::Duration::hours(o.hours() as i64)).date(),
        };
        Datetime::from_ymd(date.year(), date.month() as u8, date.day() as u8)
    }
}
