//! Pack mode: a widget or layout pack is edited, the deck waits aside.
//!
//! The working deck of a layout pack has one slide per layout (id
//! `layout-<name>`, in file order), at the layout's example value; that of a
//! widget pack has one read-only preview slide per widget
//! (`pack_mode_preview`). Its slides are the examples: none is added,
//! removed, moved or changes widget (`PackModeSlides`); writing a layout adds
//! its slide, deleting it removes it (`realign_layouts`). They only preview
//! the manifest's examples: an op on one is written into its layout's example
//! (`layout_examples`), then the slide is rebuilt from it.
//!
//! The working deck pins the edited pack, its requirements and `base` (the
//! deck's, else the built-in one), plus the built-in `essentiel` for a widget
//! pack (its host slides), nothing else. A deck-only pack (import) starts from
//! the version the deck embeds; its versions, copy included, stay in the deck
//! (`pack_library`). The working deck keeps the deck's theme, pinned themes
//! and token overrides, which are not edited there: a theme is edited in its
//! own tab (`theme_tab`).

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{
    Deck, Namespace, Op, PackFiles, PackKind, PackPin, PackVersion, ParamValue, Params, Slide, SourceBundle,
    WidgetFile, kinds, qualify,
};
use oku_slides_i18n::Localizer;

use crate::deck_closure::{Pinned, closure_with_files, combined_pins};
use crate::deck_pack::builtin_pin;
use crate::error::{
    InHistorySnafu, InPackTabSnafu, NamespaceTakenSnafu, PackModeSlidesSnafu, PackVersionMissingSnafu, ReadOnlySnafu,
    SourcesSnafu,
};
use crate::example_text::localized_examples;
use crate::library_packs::PackLibrary;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK, PackManifest};
use crate::pack_lease::PackLease;
use crate::pack_library::restore_session_version;
use crate::pack_mode_lease::Settled;
use crate::pack_mode_preview::preview_slides;
use crate::pack_mode_save::adopt_merge_key;
use crate::pack_scratch::PackScratch;
use crate::views::PackHost;
use crate::{Document, Mode, PackMode, Result, Session};

/// L'id de la slide du layout `name` dans le deck de travail.
pub(crate) fn layout_slide_id(name: &WidgetFile) -> String {
    format!("layout-{}", name.as_str())
}

/// Les paramètres d'une valeur d'exemple ; une valeur illisible est journalisée et laissée de côté.
pub(crate) fn example_params(kind: &str, example: Option<&Map<String, Value>>) -> Params {
    let values = example.into_iter().flatten();
    let params = values.filter_map(|(key, value)| match serde_json::from_value::<ParamValue>(value.clone()) {
        Ok(v) => Some((key.clone(), v)),
        Err(e) => {
            tracing::warn!(layout = %kind, param = %key, error = %e, "valeur d'exemple illisible");
            None
        }
    });
    Params(params.collect())
}

/// Les paramètres de la valeur d'exemple de `name` dans `manifest`, traduite s'il est intégré.
pub(crate) fn manifest_example_params(manifest: &PackManifest, name: &WidgetFile, kind: &str, l: &Localizer) -> Params {
    let example = manifest.examples.get(name).map(|v| localized_examples(manifest.id, v, l));
    example_params(kind, example.as_ref())
}

/// La slide de `name`, layout ou widget du pack `manifest`, à sa valeur d'exemple dans la langue de `l`.
pub(crate) fn example_slide(manifest: &PackManifest, name: &WidgetFile, l: &Localizer) -> Slide {
    let mut slide = Slide::new(layout_slide_id(name), &qualify(&manifest.namespace, name.as_str()));
    slide.params = manifest_example_params(manifest, name, &slide.kind, l);
    slide
}

/// Les layouts de `files`, dans l'ordre des fichiers ; aucun pour un pack de widgets.
fn layout_names(files: &PackFiles) -> impl Iterator<Item = &WidgetFile> {
    let layouts = files.kind() == PackKind::Layouts;
    files.widgets().keys().filter(move |n| layouts && !n.is_common())
}

/// Les slides d'un pack de layouts : une par layout de `files`, à sa valeur d'exemple.
pub(crate) fn layout_slides(manifest: &PackManifest, files: &PackFiles, l: &Localizer) -> Vec<Slide> {
    layout_names(files).map(|name| example_slide(manifest, name, l)).collect()
}

/// Le deck de travail suit les layouts de `files`, version du pack `manifest`.
///
/// La slide d'un layout supprimé part ; celle d'un nouveau s'ajoute à la
/// fin ; chacune prend la valeur d'exemple de son layout dans `manifest`,
/// dont elle n'est que l'aperçu, traduite dans la langue de `l`. Rien pour un pack de widgets.
pub(crate) fn realign_layouts(deck: &mut Deck, manifest: &PackManifest, files: &PackFiles, l: &Localizer) {
    if files.kind() != PackKind::Layouts {
        return;
    }
    let ids: Vec<(String, &WidgetFile)> = layout_names(files).map(|n| (layout_slide_id(n), n)).collect();
    deck.slides.retain(|s| ids.iter().any(|(id, _)| *id == s.id));
    for (id, name) in ids {
        match deck.slides.iter_mut().find(|s| s.id == id) {
            Some(slide) => slide.params = manifest_example_params(manifest, name, &slide.kind, l),
            None => deck.slides.push(example_slide(manifest, name, l)),
        }
    }
    // Un exemple garde `""` (texte vidé) : la slide, elle, a la forme du replica (vide = absent).
    oku_slides_core::canonicalize(deck);
}

/// Un deck qui a le style de `base` (thème, jetons, paramètres, variables,
/// packs épinglés) et ces slides.
pub(crate) fn styled_like(base: &Deck, slides: Vec<Slide>) -> Deck {
    Deck {
        version: base.version,
        theme: base.theme.clone(),
        themes: base.themes.clone(),
        params: base.params.clone(),
        vars: base.vars.clone(),
        tokens: base.tokens.clone(),
        widgets: base.widgets.clone(),
        layouts: base.layouts.clone(),
        slides,
    }
}

/// L'épinglage de la version `files` du pack `manifest`, sous son espace de noms.
pub(crate) fn pinned(manifest: &PackManifest, version: PackVersion, files: PackFiles) -> Pinned {
    let pin = PackPin { id: manifest.id, version, requires: manifest.requires.clone() };
    (manifest.namespace.clone(), pin, files)
}

/// Épingle `packs` dans `deck`, chacun dans la carte de sa sorte, et les embarque dans `sources` (en ajout seul).
///
/// Un espace de noms déjà épinglé sous l'autre sorte est refusé (`NamespaceTaken`), rien n'est épinglé.
pub(crate) fn pin_all(deck: &mut Deck, sources: &mut SourceBundle, packs: Vec<Pinned>) -> Result<()> {
    pin_deck(deck, &packs)?;
    embed_packs(sources, packs);
    Ok(())
}

/// Épingle `packs` dans `deck`, chacun dans la carte de sa sorte, sans rien embarquer.
///
/// Un espace de noms déjà épinglé sous l'autre sorte est refusé (`NamespaceTaken`), rien n'est épinglé.
pub(crate) fn pin_deck(deck: &mut Deck, packs: &[Pinned]) -> Result<()> {
    for (ns, _, files) in packs {
        let other = match files.kind() {
            PackKind::Widgets => PackKind::Layouts,
            PackKind::Layouts => PackKind::Widgets,
        };
        if let Some(owner) = deck.pins(other).get(ns) {
            return NamespaceTakenSnafu { namespace: ns.as_str(), owner: owner.id.to_string() }.fail()?;
        }
    }
    for (ns, pin, files) in packs {
        deck.pins_mut(files.kind()).insert(ns.clone(), pin.clone());
    }
    Ok(())
}

/// Embarque les fichiers de `packs` dans `sources` (en ajout seul).
pub(crate) fn embed_packs(sources: &mut SourceBundle, packs: Vec<Pinned>) {
    for (_, _, files) in packs {
        sources.add_pack(files);
    }
}

/// Embeds in `sources` the theme versions of `held` that its deck pins.
pub(crate) fn embed_reached_themes(sources: &mut SourceBundle, held: &BTreeMap<PackVersion, PackFiles>) {
    let reached: Vec<PackFiles> =
        sources.deck().themes.values().filter_map(|pin| held.get(&pin.version).cloned()).collect();
    for files in reached {
        sources.add_theme(files);
    }
}

/// Une op qui change les slides du deck : refusée en mode pack.
fn is_structural(op: &Op) -> bool {
    match op {
        Op::AddSlide { .. } | Op::RemoveSlide { .. } | Op::MoveSlide { .. } => true,
        Op::SetKind { target, .. } => target.element.is_none(),
        _ => false,
    }
}

impl Session {
    /// Switches to editing the current version of pack `id` in this deck's workshop.
    ///
    /// Its layouts become the slides, at their example values, rendered with
    /// its widgets; a widget pack has its preview slides. The working deck
    /// keeps the deck's parameters, variables, theme and token overrides. It carries no assets (a pack does not save any). A
    /// library pack is leased meanwhile (`pack_lease`).
    ///
    /// # Errors
    ///
    /// Fails if the session is not editable, in a pack tab (`InPackTab`), if
    /// pack `id` is unknown or unreadable, held by another editor
    /// (`PackOpenInTab`), or if the working deck cannot be built or rendered.
    pub fn edit_pack(&mut self, id: Uuid) -> Result<()> {
        self.enter_pack(id, PackHost::Deck, None)
    }

    /// Enters the workshop of `pack` hosted by `host`, with `lease` if given.
    ///
    /// Any error before the final switch leaves the mode, and its lease, unchanged.
    ///
    /// # Errors
    ///
    /// As `edit_pack` (`UnknownPack`, `DeckOnly`, `Render`…), plus `PackOpenInTab`, `InPackTab` and `LeaseMissing`.
    ///
    /// # Blocking
    ///
    /// Reads the library and renders the workshop.
    pub(crate) fn enter_pack(&mut self, pack: Uuid, host: PackHost, lease: Option<PackLease>) -> Result<()> {
        // L'historique ouvert se ferme d'abord ; l'aide ne passe jamais en mode pack.
        self.ensure_editable()?;
        // A pack tab edits its own pack only: another one opens in its own tab.
        ensure!(host == PackHost::Tab || !self.is_pack_tab(), InPackTabSnafu);
        // Le pack du deck seul (celui d'un import) part de sa version embarquée.
        let stored = self.pack_base(pack)?;
        let settled = self.settle_lease(pack, host, &stored, lease)?;
        if let (PackHost::Tab, Settled::Hold(lease)) = (host, &settled) {
            self.freeze_base(lease, &stored.version)?;
        }
        let deck_only = !stored.manifest.scope.is_library();
        let version = stored.version.clone();
        let mut slides = layout_slides(&stored.manifest, &stored.files, &self.localizer);
        slides.extend(preview_slides(&stored.manifest, &stored.files, &self.localizer));
        // The deck's own style: its theme, pinned themes and token overrides.
        let mut deck = styled_like(self.user_deck(), slides);
        deck.params.title = Some(stored.manifest.name.clone());
        // Les exemples peuvent garder `""` (texte vidé) : les slides ont la forme du replica.
        oku_slides_core::canonicalize(&mut deck);
        // Le pack édité et ses besoins, seuls : les autres packs du deck n'y servent pas.
        deck.widgets.clear();
        deck.layouts.clear();
        let kind = stored.files.kind();
        let edited = pinned(&stored.manifest, version.clone(), stored.files);
        let mut packs = closure_with_files(&self.library, &deck, edited, &self.user_blobs().packs)?;
        self.add_host_packs(&deck, &mut packs, kind)?;
        for (ns, pin, f) in &packs {
            deck.pins_mut(f.kind()).insert(ns.clone(), pin.clone());
        }
        let files: BTreeMap<PackVersion, PackFiles> =
            packs.iter().map(|(_, pin, f)| (pin.version.clone(), f.clone())).collect();
        let (widgets, notice) = self.resolve_widgets(&combined_pins(&deck), &files)?;
        let mut sources = SourceBundle::new(deck, widgets.registry.clone()).context(SourcesSnafu)?;
        for f in files.into_values() {
            sources.add_pack(f);
        }
        embed_reached_themes(&mut sources, &self.user_blobs().themes);
        let working = Document::new(sources, None, &self.doc_defaults);
        let edit = PackMode {
            pack,
            name: stored.manifest.name,
            kind,
            builtin: PackLibrary::is_builtin(pack),
            base: version.clone(),
            parent: Some(version),
            session_version: None,
            deck_only,
            host,
        };
        // Les widgets du pack s'installent avant de changer de document : en
        // cas d'échec, rien ne change.
        self.install_widgets(widgets)?;
        self.mode = match std::mem::replace(&mut self.mode, Mode::Deck) {
            // Déjà en édition : on change de pack, le deck reste de côté.
            Mode::Pack { stash, lease, .. } => {
                self.doc = working;
                let lease = settled.into_lease(lease);
                Mode::Pack { stash, edit, scratch: PackScratch::default(), lease, orphaned: false }
            }
            Mode::Deck => Mode::Pack {
                stash: Box::new(std::mem::replace(&mut self.doc, working)),
                edit,
                scratch: PackScratch::default(),
                lease: settled.into_lease(None),
                orphaned: false,
            },
            // L'aide est en lecture seule (`ensure_editable` refuse avant d'arriver ici).
            help @ Mode::Help { .. } => {
                self.mode = help;
                return ReadOnlySnafu.fail()?;
            }
            // L'historique ouvert se ferme d'abord (`handle` refuse avant d'arriver ici).
            history @ Mode::History { .. } => {
                self.mode = history;
                return InHistorySnafu.fail()?;
            }
            // A theme tab never enters a workshop (`ensure_editable` refuses before here).
            theme @ Mode::Theme { .. } => {
                self.mode = theme;
                return crate::document_kind::refused_in_theme_tab();
            }
        };
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
        self.ui.slide = None;
        self.ui = std::mem::take(&mut self.ui).sanitized(false);
        self.rerender()?;
        Ok(())
    }

    /// Ajoute à `packs` les packs hôtes du deck de travail que ni lui ni `deck` n'épinglent.
    ///
    /// Les exemples écrivent des éléments `base/*` : le `base` du deck, sinon l'intégré. Les
    /// widgets d'un pack de widgets se montrent sur les slides d'`essentiel` (l'intégré). Une
    /// copie de `base` remplace son pin d'origine : `base` revient ici, `essentiel` en a besoin.
    pub(crate) fn add_host_packs(&self, deck: &Deck, packs: &mut Vec<Pinned>, kind: PackKind) -> Result<()> {
        let held = |name: &str, packs: &[Pinned]| {
            packs.iter().any(|(ns, _, _)| ns.as_str() == name)
                || deck.widgets.keys().chain(deck.layouts.keys()).any(|ns| ns.as_str() == name)
        };
        if !held(kinds::base::NAMESPACE, packs) {
            packs.insert(0, self.user_base_pin()?);
        }
        if kind == PackKind::Widgets && !held(kinds::essentiel::NAMESPACE, packs) {
            packs.push(builtin_pin(ESSENTIEL_PACK)?);
        }
        Ok(())
    }

    /// Le `base` que le deck de l'utilisateur épingle, avec ses fichiers ; sans épingle, le `base` intégré.
    ///
    /// Les fichiers viennent des sources du deck, sinon de la bibliothèque ; absents des deux : `PackVersionMissing`.
    fn user_base_pin(&self) -> Result<Pinned> {
        let ns = kinds::base::namespace();
        let Some(pin) = self.deck_pins().get(&ns) else { return builtin_pin(BASE_PACK) };
        let held = self.user_blobs().packs.get(&pin.version).cloned();
        let files = held
            .or_else(|| self.library.version(pin.id, &pin.version))
            .context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        Ok((ns, pin.clone(), files))
    }

    /// Refuse, en mode pack, une transaction qui ajoute, retire, déplace une slide ou change son widget.
    pub(crate) fn ensure_pack_slides(&self, ops: &[Op]) -> Result<()> {
        let refused = matches!(self.mode, Mode::Pack { .. }) && ops.iter().any(is_structural);
        ensure!(!refused, PackModeSlidesSnafu);
        Ok(())
    }

    /// Réaligne le mode pack sur la version du document, après annuler ou rétablir.
    ///
    /// La bibliothèque retrouve cette version (une écriture de la séance l'a
    /// peut-être remplacée), sans qu'elle devienne courante ; celle d'un pack
    /// du deck seul n'y entre jamais. Le deck mis de côté passe à cette
    /// version, dans l'étape de la séance.
    pub(crate) fn realign_pack(&mut self) -> Result<()> {
        // Une copie vide l'historique du document (`save_pack`) : aucune
        // étape ne ramène à un autre pack que celui de l'édition.
        let Some((ns, pin)) = self.edited_pin().map(|(ns, pin)| (ns.clone(), pin.clone())) else { return Ok(()) };
        let held = self.doc.sources.packs.clone();
        let Mode::Pack { stash, edit, lease, .. } = &mut self.mode else { return Ok(()) };
        if pin.version == edit.base {
            return Ok(());
        }
        let files = held
            .get(&pin.version)
            .cloned()
            .context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        restore_session_version(&self.library, pin.id, &files, lease.as_ref());
        edit.base = pin.version.clone();
        edit.session_version = (edit.parent.as_ref() != Some(&pin.version)).then(|| pin.version.clone());
        let label = self.localizer.tr_with("pack-adopt-label", &[("name", edit.name.as_str().into())]);
        let merge = adopt_merge_key(edit.pack, edit.parent.as_ref());
        let mut user = stash.sources.deck().clone();
        let packs = closure_with_files(&self.library, &user, (ns, pin, files), &held)?;
        pin_all(&mut user, &mut stash.sources, packs)?;
        stash.sources.apply(user, &label, Some(&merge)).context(SourcesSnafu)?;
        stash.dirty = true;
        Ok(())
    }

    /// Leaves pack mode; unsaved changes are lost, the pack's lease is freed.
    ///
    /// # Errors
    ///
    /// Fails in a pack tab (`InPackTab`: it closes as a tab), if the widgets of
    /// the recovered deck do not resolve, or if its rendering fails.
    pub fn close_pack(&mut self) -> Result<()> {
        ensure!(!self.is_pack_tab(), InPackTabSnafu);
        match std::mem::replace(&mut self.mode, Mode::Deck) {
            Mode::Pack { stash, .. } => {
                self.doc = *stash;
                // Le deck retrouvé épingle la version de la séance : elle est figée.
                self.adopt_pins();
                self.sync_widgets()?;
            }
            Mode::Deck => {}
            help @ Mode::Help { .. } => self.mode = help,
            theme @ Mode::Theme { .. } => {
                self.mode = theme;
                return crate::document_kind::refused_in_theme_tab();
            }
            // Le deck est de côté pour l'historique : il n'est pas perdu ici.
            history @ Mode::History { .. } => {
                self.mode = history;
                return InHistorySnafu.fail()?;
            }
        }
        self.ui.slide = None;
        self.ui = std::mem::take(&mut self.ui).sanitized(false);
        self.rerender()?;
        Ok(())
    }

    /// Le pack édité, tel qu'épinglé par le deck de travail ; `None` hors du mode pack.
    pub(crate) fn edited_pin(&self) -> Option<(&Namespace, &PackPin)> {
        let Mode::Pack { edit, .. } = &self.mode else { return None };
        self.doc.sources.deck().all_pins().find(|(_, _, pin)| pin.id == edit.pack).map(|(_, ns, pin)| (ns, pin))
    }
}

#[cfg(test)]
#[path = "pack_mode_tests.rs"]
mod pack_mode_tests;
