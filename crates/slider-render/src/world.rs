//! Le monde Typst d'Écho : un système de fichiers virtuel (bibliothèque
//! embarquée, deck, assets) et un jeu de polices fixe, sans polices système.

use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use slider_core::Deck;

use crate::THEMES;
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

/// Le deck, en JSON, tel que le lit `main.typ`.
static DECK: LazyLock<FileId> = LazyLock::new(|| file_id("/deck.json"));

fn file_id(path: &str) -> FileId {
    let vpath = VirtualPath::new(path).expect("chemin virtuel valide");
    FileId::new(RootedPath::new(VirtualRoot::Project, vpath))
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

/// Empreinte (hex) d'un identifiant d'asset `sha256:…`.
fn asset_hex(id: &str) -> &str {
    id.strip_prefix("sha256:").unwrap_or(id)
}

pub(crate) struct SliderWorld {
    library: LazyHash<Library>,
    fonts: Fonts,
    main: FileId,
    sources: HashMap<FileId, Source>,
    deck: Bytes,
    /// Assets par fichier (`/assets/<hex>`). Leur identifiant est l'empreinte
    /// de leur contenu : un asset déjà là n'est jamais recopié.
    assets: HashMap<FileId, Bytes>,
}

impl SliderWorld {
    pub fn new(fonts: Fonts) -> Self {
        let mut sources = HashMap::new();
        for (path, text) in LIBRARY {
            let id = file_id(&format!("/slider/{path}"));
            sources.insert(id, Source::new(id, (*text).to_owned()));
        }
        let main = file_id("/main.typ");
        sources.insert(main, Source::new(main, main_source(None, std::iter::empty())));

        Self {
            library: LazyHash::new(Library::default()),
            fonts,
            main,
            sources,
            deck: Bytes::new(br#"{"slides":[]}"#.as_slice()),
            assets: HashMap::new(),
        }
    }

    /// Remplace le deck et les assets. La bibliothèque et les assets déjà
    /// chargés restent en place.
    pub fn set_deck(&mut self, deck: &Deck, assets: &BTreeMap<String, Vec<u8>>) {
        let wanted: HashMap<FileId, &Vec<u8>> =
            assets.iter().map(|(id, bytes)| (file_id(&format!("/assets/{}", asset_hex(id))), bytes)).collect();
        self.assets.retain(|id, _| wanted.contains_key(id));
        for (id, bytes) in wanted {
            self.assets.entry(id).or_insert_with(|| Bytes::new(bytes.clone()));
        }
        self.deck = Bytes::new(serde_json::to_vec(deck).expect("un deck se sérialise toujours"));

        let text = main_source(deck.theme.as_deref(), assets.keys().map(|k| asset_hex(k)));
        let main = self.sources.get_mut(&self.main).expect("source principale");
        if main.text() != text {
            main.replace(&text);
        }
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
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.sources.get(&id).cloned().ok_or_else(|| FileError::NotFound(id.vpath().get_with_slash().into()))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == *DECK {
            return Ok(self.deck.clone());
        }
        if let Some(b) = self.assets.get(&id) {
            return Ok(b.clone());
        }
        if let Some(s) = self.sources.get(&id) {
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
