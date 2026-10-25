//! Polices du rendu : celles intégrées à Typst, plus celles fournies (app,
//! deck). Chargées une fois, partagées entre les rendus sans copie.

use std::sync::{Arc, LazyLock};

use typst::foundations::Bytes;
use typst::text::{Font, FontBook, FontStyle, FontWeight};
use typst::utils::LazyHash;

/// Polices intégrées à Typst, lues une fois par processus.
static BUILTIN: LazyLock<Vec<Font>> =
    LazyLock::new(|| typst_assets::fonts().flat_map(|data| Font::iter(Bytes::new(data))).collect());

/// Un jeu de polices et son catalogue. Se clone sans rien copier.
#[derive(Clone)]
pub struct Fonts(Arc<Inner>);

struct Inner {
    fonts: Vec<Font>,
    /// Fichiers fournis (hors polices intégrées), même s'ils ne sont pas des polices.
    files: Vec<Bytes>,
    book: LazyHash<FontBook>,
}

impl Fonts {
    /// Polices de Typst, plus les fichiers `data` (TTF, OTF, TTC).
    pub fn new(data: impl IntoIterator<Item = Vec<u8>>) -> Self {
        let files: Vec<Bytes> = data.into_iter().map(Bytes::new).collect();
        let fonts = BUILTIN.iter().cloned().chain(parse(&files)).collect();
        Self::from_fonts(fonts, files)
    }

    /// Ce jeu, plus d'autres fichiers de police (celles d'un deck).
    #[must_use]
    pub fn with(&self, data: impl IntoIterator<Item = Vec<u8>>) -> Self {
        let added: Vec<Bytes> = data.into_iter().map(Bytes::new).collect();
        let fonts = self.0.fonts.iter().cloned().chain(parse(&added)).collect();
        let files = self.0.files.iter().cloned().chain(added).collect();
        Self::from_fonts(fonts, files)
    }

    /// Les fichiers fournis (hors polices de Typst), dans l'ordre d'ajout.
    pub fn files(&self) -> impl Iterator<Item = &[u8]> {
        self.0.files.iter().map(Bytes::as_slice)
    }

    /// Les fichiers fournis, partagés sans copie (chacun vit autant que ses
    /// détenteurs), dans l'ordre d'ajout.
    pub fn shared_files(&self) -> impl Iterator<Item = impl AsRef<[u8]> + Send + Sync + 'static> + '_ {
        self.0.files.iter().cloned()
    }

    fn from_fonts(fonts: Vec<Font>, files: Vec<Bytes>) -> Self {
        let book = LazyHash::new(FontBook::from_fonts(&fonts));
        Self(Arc::new(Inner { fonts, files, book }))
    }

    pub(crate) fn get(&self, index: usize) -> Option<Font> {
        self.0.fonts.get(index).cloned()
    }

    /// Nom et fichier de la face normale (400, droite) de `family`, ou de sa
    /// première face.
    pub(crate) fn regular_face(&self, family: &str) -> Option<(String, Bytes)> {
        let faces = || self.0.fonts.iter().filter(|f| f.info().family.eq_ignore_ascii_case(family));
        let regular = |f: &&Font| {
            let v = f.info().variant;
            v.weight == FontWeight::REGULAR && v.style == FontStyle::Normal
        };
        let face = faces().find(regular).or_else(|| faces().next())?;
        Some((face.info().family.clone(), face.data().clone()))
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

fn parse(files: &[Bytes]) -> impl Iterator<Item = Font> + '_ {
    files.iter().flat_map(|bytes| Font::iter(bytes.clone()))
}

#[cfg(test)]
#[path = "fonts_tests.rs"]
mod fonts_tests;
