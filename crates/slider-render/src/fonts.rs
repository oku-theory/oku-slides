//! Polices du rendu : celles intégrées à Typst, plus celles fournies (app,
//! deck). Chargées une fois, partagées entre les rendus sans copie.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use typst::foundations::Bytes;
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;

/// Polices intégrées à Typst, lues une fois par processus.
static BUILTIN: LazyLock<Vec<Font>> =
    LazyLock::new(|| typst_assets::fonts().flat_map(|data| Font::iter(Bytes::new(data))).collect());

/// Un jeu de polices et son catalogue. Se clone sans rien copier.
#[derive(Clone)]
pub struct Fonts(Arc<Inner>);

struct Inner {
    fonts: Vec<Font>,
    book: LazyHash<FontBook>,
}

impl Fonts {
    /// Polices de Typst, plus les fichiers `data` (TTF, OTF, TTC).
    pub fn new(data: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self::from_fonts(BUILTIN.iter().cloned().chain(parse(data)).collect())
    }

    /// Ce jeu, plus d'autres fichiers de police (celles d'un deck).
    pub fn with(&self, data: impl IntoIterator<Item = Vec<u8>>) -> Self {
        Self::from_fonts(self.0.fonts.iter().cloned().chain(parse(data)).collect())
    }

    fn from_fonts(fonts: Vec<Font>) -> Self {
        let book = LazyHash::new(FontBook::from_fonts(&fonts));
        Self(Arc::new(Inner { fonts, book }))
    }

    pub(crate) fn get(&self, index: usize) -> Option<Font> {
        self.0.fonts.get(index).cloned()
    }

    pub(crate) fn book(&self) -> &LazyHash<FontBook> {
        &self.0.book
    }
}

impl Default for Fonts {
    /// Les seules polices de Typst.
    fn default() -> Self {
        Self::new(std::iter::empty())
    }
}

impl std::fmt::Debug for Fonts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Fonts").field("count", &self.0.fonts.len()).finish()
    }
}

fn parse(data: impl IntoIterator<Item = Vec<u8>>) -> impl Iterator<Item = Font> {
    data.into_iter().flat_map(|bytes| Font::iter(Bytes::new(bytes)))
}

/// Fichiers de police d'un dossier (TTF, OTF, TTC, OTC), par nom de fichier.
pub fn read_font_dir(dir: &Path) -> std::io::Result<BTreeMap<String, Vec<u8>>> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let is_font = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| ["ttf", "otf", "ttc", "otc"].iter().any(|x| e.eq_ignore_ascii_case(x)));
        if let (true, Some(name)) = (is_font, path.file_name()) {
            out.insert(name.to_string_lossy().into_owned(), std::fs::read(&path)?);
        }
    }
    Ok(out)
}
