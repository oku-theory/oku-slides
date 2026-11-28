//! What the interface shows of a session: its snapshot, a slide's SVG and its detail.

use std::sync::Arc;

use oku_slides_i18n::Localize;
use snafu::OptionExt;

use crate::error::NoSlideSnafu;
use crate::views::{PageHash, SlideSummary, SlideView, Snapshot, Target};
use crate::{Mode, Result, Session, SessionError, color_warnings, fields, interp, items, views};

impl Session {
    /// The full state the interface shows, read without touching the disk.
    pub fn snapshot(&self) -> Snapshot<'_> {
        let compiled = self.compiled.as_ref();
        let (deck, doc) = (self.doc.sources.deck(), &self.doc);
        let pages = compiled.map(|c| c.layout().slides.as_slice()).unwrap_or_default();
        let slides = deck
            .slides
            .iter()
            .enumerate()
            .map(|(i, s)| {
                // Une slide par page, dans l'ordre du deck : la i-ième page d'abord.
                let page = pages
                    .get(i)
                    .filter(|p| p.slide == s.id)
                    .or_else(|| pages.iter().find(|p| p.slide == s.id))
                    .map_or(0, |p| p.page);
                SlideSummary {
                    id: &s.id,
                    kind: &s.kind,
                    title: fields::slide_text(deck, s, &self.registry, "title").map(|t| interp(t, &deck.vars)),
                    page,
                    hash: PageHash(compiled.and_then(|c| c.page_hash(page.checked_sub(1)?))),
                }
            })
            .collect();
        let name = self.document_name(doc);
        Snapshot {
            path: doc.path.as_deref(),
            name,
            dirty: doc.dirty,
            theme: deck.theme.as_ref(),
            theme_pins: &deck.themes,
            theme_name: deck.theme.as_ref().and_then(|ns| self.theme_manifest(deck.themes.get(ns)?)).map(|m| m.name),
            themes: &self.theme_list,
            title: deck.params.title.as_deref().map(|t| interp(t, &deck.vars)),
            title_source: deck.params.title.as_deref(),
            lang: deck.params.lang.as_deref(),
            slides,
            can_undo: doc.sources.replica().can_undo(),
            undo_label: doc.sources.replica().undo_label(),
            can_redo: doc.sources.replica().can_redo(),
            redo_label: doc.sources.replica().redo_label(),
            undo_turn: self.undo_turn(),
            warnings: compiled
                .map(|c| {
                    let typst = c.warnings.iter().map(|w| w.localize(&self.localizer));
                    let tokens = c.layout().tokens.as_ref();
                    typst.chain(color_warnings::unknown_color_warnings(deck, tokens, &self.localizer)).collect()
                })
                .unwrap_or_default(),
            notice: doc.notice.as_deref(),
            disk_replaced: doc.disk_replaced,
            appearance: &self.appearance,
            widgets: self.deck_pins(),
            layouts: &self.user_deck().layouts,
            editing: self.editing_view(),
            theming: self.theme_tab_view(),
            deck_pack: self.deck_pack_is_deck_only(),
            pack_updates: self.pack_updates(),
            pack_widgets: self.pack_widget_names(),
            add_packs: self.add_packs(),
            free_slots: self.free_slots(),
            widget_trials: self.examples_revision(),
            preview_set: hex::encode(self.packs.fingerprint()),
            ui: &self.ui,
            history: match &self.mode {
                Mode::History { history, .. } => Some(history.view()),
                _ => None,
            },
            read_only: self.help().is_some(),
            can_present: self.can_present().is_ok(),
            host_params: self.registry.host_params(),
            guides: self.guides_view(),
            received_to_review: self.received_to_review(),
            knowledge: self.knowledge_summary(),
            attach_limits: self.attachments.ui_limits(),
        }
    }

    /// SVG of a slide, cached by page hash.
    ///
    /// # Errors
    ///
    /// Fails (`NoSlide`) for an unknown slide, or if rendering the page to SVG fails.
    pub fn svg(&mut self, id: &str) -> Result<Arc<str>> {
        let page = self.page_of(id).context(NoSlideSnafu { id })?;
        let hash = |s: &Self| s.compiled.as_ref().and_then(|c| c.page_hash(page - 1)).unwrap_or_default();
        if let Some(svg) = self.svg_cache.get(&hash(self)) {
            return Ok(svg.clone());
        }
        let svg: Arc<str> = self.with_compiled(|c| c.svg(page - 1))?.into();
        // Empreinte du rendu installé : un re-rendu (poignée périmée) a pu le remplacer.
        self.svg_cache.insert(hash(self), svg.clone());
        Ok(svg)
    }

    /// Detail of a slide: its content and its selectable boxes.
    ///
    /// # Errors
    ///
    /// Fails (`NoSlide`) if the slide is unknown to the deck or to its rendered layout.
    pub fn slide(&self, id: &str) -> Result<SlideView<'_>> {
        let no_slide = || -> SessionError { NoSlideSnafu { id }.build().into() };
        let deck = self.doc.sources.deck();
        let slide = deck.slide(id).ok_or_else(no_slide)?;
        let compiled = self.compiled.as_ref().ok_or_else(no_slide)?;
        let layout = compiled.layout();
        let page = layout.page_of(id).ok_or_else(no_slide)?;
        let targets = layout
            .boxes_on(page)
            .iter()
            .filter_map(|b| {
                let l = slide.locate(&b.id)?;
                Some(Target::element(b, &l.element.kind, l.is_free()))
            })
            .collect();
        let fields = fields::field_views(deck, slide, &self.registry, layout.fields_on(page), self.localizer.locale());
        Ok(SlideView {
            slide,
            page,
            hash: PageHash(compiled.page_hash(page - 1)),
            width: views::SLIDE_WIDTH,
            height: views::SLIDE_HEIGHT,
            targets,
            fields,
            items: items::item_views(slide, &self.registry, layout.items_on(page)),
            lists: items::list_views(slide, &self.registry),
            zone: layout.zone_on(page),
        })
    }
}
