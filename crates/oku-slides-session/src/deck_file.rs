//! Le fichier du deck : chargement (`load`, `load_document`), ouverture
//! (`open`), deck neuf (`new_deck`) et écriture (`write_to`, `write_file`).
//!
//! Pour un deck qui n'a pas encore de fichier : son nom, tiré du titre, et
//! une place libre dans le dossier des decks (`Budget.oku-slides.pdf`,
//! sinon `Budget 2.oku-slides.pdf`…).

use std::path::{Path, PathBuf};

use oku_slides_core::SourceBundle;
use oku_slides_render_worker::RemoteCompiled;
use snafu::ResultExt;

use crate::deck_closure::combined_pins;
use crate::error::{PdfSnafu, SourcesSnafu, WriteSnafu};
use crate::files::{EXTENSION, read_document};
use crate::{Document, Result, Session, blank_deck, charts, deck_pack, deck_theme};

/// Au-delà, le nom est coupé : les systèmes de fichiers limitent à 255 octets.
const MAX_STEM_CHARS: usize = 120;

/// Un nom de fichier sûr tiré de `title` ; `fallback` s'il n'en reste rien.
pub(crate) fn safe_stem(title: &str, fallback: &str) -> String {
    let cleaned: String = title
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '-' } else { c })
        .take(MAX_STEM_CHARS)
        .collect();
    // Pas de fichier caché ni de nom fait de points.
    let stem = cleaned.trim().trim_start_matches('.').trim();
    if stem.is_empty() { fallback.to_owned() } else { stem.to_owned() }
}

/// Le premier `dir/stem[ n].oku-slides.pdf` libre selon `taken`.
pub(crate) fn free_path(dir: &Path, stem: &str, taken: impl Fn(&Path) -> bool) -> PathBuf {
    let first = dir.join(format!("{stem}{EXTENSION}"));
    if !taken(&first) {
        return first;
    }
    (2..u64::MAX).map(|n| dir.join(format!("{stem} {n}{EXTENSION}"))).find(|p| !taken(p)).unwrap_or(first)
}

impl Session {
    /// Replaces the document with a blank deck, marked as such.
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable, if the blank deck's sources cannot be created, or if its
    /// rendering fails.
    pub fn new_deck(&mut self) -> Result<()> {
        let sources = SourceBundle::new(blank_deck(&self.localizer), self.registry.clone()).context(SourcesSnafu)?;
        self.load(sources, None)?;
        self.doc.blank = true;
        self.persist(true);
        Ok(())
    }

    /// Loads an (already read) document into the session.
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable (history, help), or if the document does not load or
    /// render.
    pub fn load(&mut self, sources: SourceBundle, path: Option<PathBuf>) -> Result<()> {
        self.ensure_editable()?;
        self.load_document(Document::new(sources, path, &self.doc_defaults))
    }

    /// Charge un document ; ses graphiques sont mis sous forme canonique,
    /// pour que la grille puisse éditer leurs données. Il se lit et se rend
    /// avec les widgets des packs qu'il épingle ; sans pack, il adopte le pack
    /// par défaut et ses besoins (voir `deck_pack`).
    pub(crate) fn load_document(&mut self, mut doc: Document) -> Result<()> {
        self.end_widget_session();
        deck_pack::adopt_default_packs(&self.library, &mut doc.sources)?;
        deck_theme::adopt_default_theme(&self.themes, &mut doc.sources)?;
        // Une reprise ou un fichier peut porter une fermeture cassée par une fusion.
        if let Err(e) = deck_theme::repair_theme_closure(&self.themes, &mut doc.sources) {
            tracing::warn!(error = %e.one_line(), "épinglages des thèmes non réparés");
        }
        let (widgets, notice) = self.resolve_widgets(&combined_pins(doc.sources.deck()), &doc.sources.packs)?;
        doc.sources.set_registry(widgets.registry.clone());
        let mut deck = doc.sources.deck().clone();
        charts::canonicalize_deck(&mut deck, &self.chart_labels, &widgets.registry);
        // Les options réécrites reprennent la forme canonique (clés triées),
        // celle que relit le replica.
        oku_slides_core::canonicalize(&mut deck);
        if deck != *doc.sources.deck() {
            // Forme canonique des graphiques : pas une action de l'utilisateur.
            doc.sources.apply_silent(deck).context(SourcesSnafu)?;
        }
        // Le rendu des nouvelles polices (avec les nouveaux widgets) et les
        // widgets sont prêts avant de remplacer le document : si l'un
        // échoue, l'ancien document, son registre et son rendu restent.
        let fonts = (doc.sources.fonts != self.doc.sources.fonts)
            .then(|| self.fonts_renderer(&doc.sources.fonts, &widgets.set, &widgets.registry))
            .transpose()?;
        self.install_widgets(widgets)?;
        // L'avis des thèmes écartés posé sur le deck vierge, pas encore vu, suit le document (reprise d'onglet).
        let carried = self.carried_themes_notice();
        self.doc = doc;
        if let Some(text) = carried {
            self.set_notice(text);
        }
        // Un autre document : l'état d'un import ne vaut plus (`load_import` le pose après) ;
        // un document refusé plus haut laisse l'onglet et son import tels quels.
        self.import = None;
        // Un autre replica : ses rangs d'étapes ne sont plus ceux des tours notés.
        self.turn_steps = crate::agent::turn_undo::TurnSteps::default();
        if let Some(fonts) = fonts {
            self.install_fonts(fonts);
        }
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
        self.ui.slide = None;
        self.ui = std::mem::take(&mut self.ui).sanitized(false);
        self.theme_list = self.themes.themes();
        self.refresh_pack_updates();
        self.rerender()
    }

    /// Opens a `.oku-slides.pdf` (or a JSON deck, for development).
    ///
    /// # Errors
    ///
    /// Fails if the file cannot be read or is not a recognized document, or if it does not load (see
    /// [`Self::load`]).
    pub fn open(&mut self, path: &Path) -> Result<()> {
        let opened = read_document(&self.storage, path, self.source_limits, &self.registry)?;
        let (pdf, modified_elsewhere) = (opened.pdf, opened.modified_elsewhere);
        let notice = opened.notice(&self.localizer);
        // Un deck JSON n'est pas un document Oku Slides : on ne l'écrasera pas.
        self.load(opened.sources, pdf.then(|| path.to_owned()))?;
        // Les versions reçues entrent en silence ; leur relation à ma version se lit dans le panneau des packs.
        self.receive_packs();
        self.refresh_pack_updates();
        tracing::info!(path = %path.display(), pdf, modified_elsewhere, "{}", self.localizer.tr("log-document-opened"));
        // Après l'avis d'un pack cassé, s'il y en a un.
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
        self.doc.saved_hash = pdf.then_some(opened.hash);
        self.persist(true);
        Ok(())
    }

    /// Écrit en `.oku-slides.pdf` (`path` déjà normalisé, voir `save_path`) :
    /// PDF + blob des sources, écriture atomique.
    pub(crate) fn write_to(&mut self, path: PathBuf) -> Result<()> {
        let hash = self.write_file(&path)?;
        self.doc.path = Some(path);
        self.doc.dirty = false;
        self.doc.saved_hash = Some(hash);
        Ok(())
    }

    /// Écrit le document (PDF et sources) dans `path` ; son empreinte.
    pub(crate) fn write_file(&mut self, path: &Path) -> Result<String> {
        let pdf = self.with_compiled(RemoteCompiled::pdf)?;
        let zip = self.doc.sources.to_zip().context(SourcesSnafu)?;
        let out = oku_slides_pdf::embed_sources(&pdf, &zip).context(PdfSnafu)?;
        let bytes = out.len();
        let hash = oku_slides_core::asset_id(&out);
        self.storage.write_atomic(path, out).context(WriteSnafu { path })?;
        tracing::info!(path = %path.display(), bytes, "{}", self.localizer.tr("log-document-saved"));
        Ok(hash)
    }
}

#[cfg(test)]
#[path = "deck_file_tests.rs"]
mod deck_file_tests;
