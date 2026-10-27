//! Theme tabs: a library theme's draft, edited in its own tab.
//!
//! The session is in `Mode::Theme`. Its document is a read-only specimen
//! deck: one slide per layout of the built-in Essentiel (`layout-<name>`, at
//! its example value), the built-in `base` and `essentiel` pinned, no
//! theme, the draft's tokens as its tokens and the draft's name as its
//! title. Only token ops and the draft requests change it
//! (`ensure_caller_ops`, `Request::theme_tab_allows`); the draft itself
//! lives in [`ThemeTab`]. A user theme's writes are parented on the version
//! the tab opened; a built-in's draft is a child, with no parent.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{DECK_VERSION, Deck, DeckParams, PackFiles, PackVersion, SourceBundle};
use oku_slides_i18n::Localizer;

use crate::agent::bounds::label;
use crate::deck_closure::{Pinned, combined_pins};
use crate::deck_pack::builtin_pin;
use crate::error::{LeaseMissingSnafu, PackTabNotFreshSnafu, SourcesSnafu, UnknownPackSnafu, UnknownThemeSnafu};
use crate::library::builtin_versions;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK};
use crate::pack_lease::{LeaseScope, PackLease, covers};
use crate::pack_mode::{layout_slides, pin_deck};
use crate::theme_edit::{ThemeEdit, ThemeEditView, tokens_value};
use crate::{Document, Mode, Result, Session, TabDocument, TabInfo};

/// The theme a theme tab drafts, and the versions its writes hang on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeTab {
    /// The draft: base version, namespace, name and parents.
    pub draft: ThemeEdit,
    /// The version a write is parented on: the opened one; none for a built-in.
    pub parent: Option<PackVersion>,
    /// The version this tab wrote last, if it wrote one.
    pub session_version: Option<PackVersion>,
}

/// A theme tab as the interface sees it: the draft and its flags.
#[derive(Debug, Serialize)]
pub struct ThemeTabView {
    /// The draft, as the theme panel shows it.
    pub theme: ThemeEditView,
    /// The version this tab wrote last, if it wrote one.
    pub session_version: Option<PackVersion>,
    /// True when this tab lacks its theme's lease: it cannot save until it retakes it.
    pub read_only: bool,
    /// True when this tab may write without first taking the lease.
    pub leased: bool,
    /// The theme's name, cleaned for titles and dialogs.
    pub display_name: String,
}

/// The specimen deck of `draft` with `tokens`, and the packs it pins.
///
/// Essentiel's layouts, at their example values in the language of `l`;
/// the built-in `base` and `essentiel` pinned; no theme.
///
/// # Errors
///
/// `UnknownPack` if a built-in pack is missing (a broken build).
pub(crate) fn specimen_deck(
    draft: &ThemeEdit,
    tokens: Map<String, Value>,
    l: &Localizer,
) -> Result<(Deck, Vec<Pinned>)> {
    let essentiel = builtin_versions()
        .iter()
        .find(|b| b.manifest.id == ESSENTIEL_PACK)
        .context(UnknownPackSnafu { id: ESSENTIEL_PACK })?;
    let mut deck = Deck {
        version: DECK_VERSION,
        theme: None,
        themes: BTreeMap::new(),
        params: DeckParams { title: Some(draft.name.clone()), ..DeckParams::default() },
        vars: BTreeMap::new(),
        tokens: tokens_value(tokens),
        widgets: BTreeMap::new(),
        layouts: BTreeMap::new(),
        slides: layout_slides(&essentiel.manifest, &essentiel.files, l),
    };
    // Examples may keep `""` (cleared text): the slides take the replica's shape.
    oku_slides_core::canonicalize(&mut deck);
    let packs = vec![builtin_pin(BASE_PACK)?, builtin_pin(ESSENTIEL_PACK)?];
    pin_deck(&mut deck, &packs)?;
    Ok((deck, packs))
}

impl Session {
    /// Opens theme `theme` in this session, for a host without tabs.
    ///
    /// The tab takes the theme's lease; while another tab holds it, it opens read-only.
    ///
    /// # Errors
    ///
    /// As `init_theme_tab`.
    ///
    /// # Blocking
    ///
    /// Reads the theme library, takes `DRAFTS` for the lease and renders the specimen deck.
    pub fn open_theme_here(&mut self, theme: Uuid) -> Result<()> {
        let lease = self.opening_theme_lease(theme)?;
        self.init_theme_tab(lease, theme)
    }

    /// Turns this fresh session into the theme tab of `theme`, holding `lease` if given.
    ///
    /// Holding the lease of a user theme, it freezes its base. On an error
    /// the session stays a pristine deck tab: a failed render or freeze puts
    /// a blank deck back (unless that fails too, logged: the tab then keeps
    /// the specimen deck as a deck), and the library is left as it was.
    ///
    /// # Errors
    ///
    /// `PackTabNotFresh` unless the session is a pristine deck tab,
    /// `LeaseMissing` for a lease of another theme or library, `UnknownTheme`
    /// for a theme the library lacks, the widget, source and render errors of
    /// the specimen deck, and the `freeze_theme_base` errors.
    ///
    /// # Blocking
    ///
    /// Reads the theme library, renders the specimen deck and takes `DRAFTS` to freeze its base.
    pub(crate) fn init_theme_tab(&mut self, lease: Option<PackLease>, theme: Uuid) -> Result<()> {
        let fresh = matches!(self.mode, Mode::Deck) && self.doc.path.is_none() && self.tab_info().pristine;
        ensure!(fresh, PackTabNotFreshSnafu);
        if lease.as_ref().is_some_and(|l| !covers(Some(l), self.library.theme_scope(), theme)) {
            tracing::error!(theme = %theme, "lease of another theme or library given to its tab");
            return LeaseMissingSnafu.fail()?;
        }
        let (draft, tokens) = self.initial_theme_edit(theme)?;
        let parent = (!draft.child()).then(|| draft.base.version.clone());
        self.install_theme_tab(ThemeTab { draft, parent, session_version: None }, tokens, lease)?;
        // Frozen last: a tab that fails to open leaves the library as it was.
        if let Err(e) = self.rerender().and_then(|()| self.freeze_opened_theme()) {
            self.back_to_blank();
            return Err(e);
        }
        Ok(())
    }

    /// Installs the specimen deck of `edit` with `tokens`, in `Mode::Theme` holding `lease`.
    ///
    /// Not rendered; the draft's base is embedded. Opening and resuming share it.
    ///
    /// # Errors
    ///
    /// The widget, source and install errors of the specimen deck.
    pub(crate) fn install_theme_tab(
        &mut self,
        edit: ThemeTab,
        tokens: Map<String, Value>,
        lease: Option<PackLease>,
    ) -> Result<()> {
        let (deck, packs) = specimen_deck(&edit.draft, tokens, &self.localizer)?;
        let files: BTreeMap<PackVersion, PackFiles> = packs.into_iter().map(|(_, pin, f)| (pin.version, f)).collect();
        let (widgets, notice) = self.resolve_widgets(&combined_pins(&deck), &files)?;
        let mut sources = SourceBundle::new(deck, Arc::clone(&widgets.registry)).context(SourcesSnafu)?;
        for f in files.into_values() {
            sources.add_pack(f);
        }
        let doc = Document { blank: false, dirty: false, ..Document::new(sources, None, &self.doc_defaults) };
        self.install_document(doc, widgets, notice)?;
        self.mode = Mode::Theme { edit, switches: Vec::new(), lease };
        // The specimen deck is new: the draft's base is embedded in it.
        self.embed_theme_base();
        Ok(())
    }

    /// Makes this session a pristine deck tab again, after a theme tab failed to open.
    ///
    /// The lease drops with the theme mode. A blank deck that cannot be put
    /// back is logged as an error: the session then stays a deck tab on the
    /// specimen deck, no longer pristine.
    fn back_to_blank(&mut self) {
        self.mode = Mode::Deck;
        let blank = SourceBundle::new(crate::blank_deck(&self.localizer), Arc::clone(&self.registry));
        let reset = match blank.context(SourcesSnafu) {
            Ok(sources) => {
                let doc = Document { blank: true, ..Document::new(sources, None, &self.doc_defaults) };
                self.load_document(doc)
            }
            Err(e) => Err(e.into()),
        };
        if let Err(e) = reset {
            tracing::error!(error = %e.one_line(), "blank deck not put back after a theme tab failed to open");
        }
    }

    /// Routes opening theme `theme` from this tab: its lease scope and its label.
    ///
    /// Any tab kind may ask; nothing changes. The scope is `None` for a
    /// library without a theme folder (no lease to take).
    ///
    /// # Errors
    ///
    /// `UnknownTheme` for a theme the library lacks.
    ///
    /// # Blocking
    ///
    /// Reads the theme library.
    pub(crate) fn route_theme(&self, theme: Uuid) -> Result<(Option<LeaseScope>, String)> {
        let current = self.themes.current_version(theme).context(UnknownThemeSnafu { theme: theme.to_string() })?;
        Ok((self.library.theme_scope().cloned(), label(&current.manifest.name)))
    }

    /// The tab of a theme tab: its theme's name and document; `None` elsewhere.
    pub(crate) fn theme_tab_info(&self) -> Option<TabInfo> {
        let Mode::Theme { edit, .. } = &self.mode else { return None };
        Some(TabInfo {
            title: label(&edit.draft.name),
            dirty: self.doc.dirty,
            pristine: false,
            editing_pack: false,
            theme: Some(edit.draft.base.id),
            document: TabDocument::Theme { theme: edit.draft.base.id },
        })
    }

    /// A theme tab for the snapshot: the draft and its flags; `None` elsewhere.
    pub(crate) fn theme_tab_view(&self) -> Option<ThemeTabView> {
        let Mode::Theme { edit, .. } = &self.mode else { return None };
        let leased = self.theme_leased();
        Some(ThemeTabView {
            theme: self.theme_edit_view(&edit.draft),
            session_version: edit.session_version.clone(),
            read_only: !leased,
            leased,
            display_name: label(&edit.draft.name),
        })
    }
}

#[cfg(test)]
#[path = "theme_tab_tests.rs"]
pub(crate) mod theme_tab_tests;

#[cfg(test)]
#[path = "theme_tab_ops_tests.rs"]
mod theme_tab_ops_tests;
