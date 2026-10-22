//! Mode pack : un pack de widgets ou de layouts s'édite, le deck attend de côté.
//!
//! Le deck de travail d'un pack de layouts a une slide par layout (id
//! `layout-<nom>`, dans l'ordre des fichiers), à la valeur d'exemple du
//! layout ; celui d'un pack de widgets n'en a pas. Ses slides sont les
//! exemples : aucune ne s'ajoute, ne se retire, ne se déplace ni ne change
//! de widget (`PackModeSlides`) ; écrire un layout ajoute sa slide, le
//! supprimer la retire (`realign_layouts`). Elles ne sont que l'aperçu des
//! exemples du manifeste : une op sur l'une s'écrit dans l'exemple de son
//! layout (`layout_examples`), puis la slide est refaite depuis lui.
//!
//! Le deck de travail épingle le pack édité, ses besoins et `base` (celui du
//! deck, sinon l'intégré), plus `essentiel` intégré pour un pack de widgets
//! (ses slides hôtes), rien d'autre. Un pack du deck seul (import) part de
//! la version qu'embarque le deck ; ses versions, copie comprise, restent
//! dans le deck (`pack_library`). Le thème du deck s'y édite en brouillon
//! (voir `theme_edit`).

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{
    Deck, Namespace, Op, PackFiles, PackKind, PackPin, PackVersion, ParamValue, Params, Slide, SourceBundle,
    WidgetFile, kinds, qualify,
};

use crate::deck_closure::{Pinned, closure_with_files, combined_pins};
use crate::deck_pack::builtin_pin;
use crate::error::{
    InHistorySnafu, NamespaceTakenSnafu, PackModeSlidesSnafu, PackVersionMissingSnafu, ReadOnlySnafu, SourcesSnafu,
};
use crate::library_packs::PackLibrary;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK, PackManifest};
use crate::pack_library::restore_session_version;
use crate::pack_mode_save::adopt_merge_key;
use crate::pack_scratch::PackScratch;
use crate::theme_edit::tokens_value;
use crate::{Document, Mode, PackMode, Result, Session};

/// L'id de la slide du layout `name` dans le deck de travail.
pub(crate) fn layout_slide_id(name: &WidgetFile) -> String {
    format!("layout-{}", name.as_str())
}

/// Les paramètres d'une valeur d'exemple ; une valeur illisible est journalisée et laissée de côté.
fn example_params(kind: &str, example: Option<&Map<String, Value>>) -> Params {
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

/// La slide de `name`, layout ou widget du pack `manifest`, à sa valeur d'exemple.
pub(crate) fn example_slide(manifest: &PackManifest, name: &WidgetFile) -> Slide {
    let mut slide = Slide::new(layout_slide_id(name), &qualify(&manifest.namespace, name.as_str()));
    slide.params = example_params(&slide.kind, manifest.examples.get(name));
    slide
}

/// Les layouts de `files`, dans l'ordre des fichiers ; aucun pour un pack de widgets.
fn layout_names(files: &PackFiles) -> impl Iterator<Item = &WidgetFile> {
    let layouts = files.kind() == PackKind::Layouts;
    files.widgets().keys().filter(move |n| layouts && !n.is_common())
}

/// Les slides d'un pack de layouts : une par layout de `files`, à sa valeur d'exemple.
pub(crate) fn layout_slides(manifest: &PackManifest, files: &PackFiles) -> Vec<Slide> {
    layout_names(files).map(|name| example_slide(manifest, name)).collect()
}

/// Le deck de travail suit les layouts de `files`, version du pack `manifest`.
///
/// La slide d'un layout supprimé part ; celle d'un nouveau s'ajoute à la
/// fin ; chacune prend la valeur d'exemple de son layout dans `manifest`,
/// dont elle n'est que l'aperçu. Rien pour un pack de widgets.
pub(crate) fn realign_layouts(deck: &mut Deck, manifest: &PackManifest, files: &PackFiles) {
    if files.kind() != PackKind::Layouts {
        return;
    }
    let ids: Vec<(String, &WidgetFile)> = layout_names(files).map(|n| (layout_slide_id(n), n)).collect();
    deck.slides.retain(|s| ids.iter().any(|(id, _)| *id == s.id));
    for (id, name) in ids {
        match deck.slides.iter_mut().find(|s| s.id == id) {
            Some(slide) => slide.params = example_params(&slide.kind, manifest.examples.get(name)),
            None => deck.slides.push(example_slide(manifest, name)),
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
    for (ns, _, files) in &packs {
        let other = match files.kind() {
            PackKind::Widgets => PackKind::Layouts,
            PackKind::Layouts => PackKind::Widgets,
        };
        if let Some(owner) = deck.pins(other).get(ns) {
            return NamespaceTakenSnafu { namespace: ns.as_str(), owner: owner.id.to_string() }.fail()?;
        }
    }
    for (ns, pin, files) in packs {
        deck.pins_mut(files.kind()).insert(ns, pin);
        sources.add_pack(files);
    }
    Ok(())
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
    /// Passe en édition de la version courante du pack `id`.
    ///
    /// Ses layouts deviennent les slides, à leurs valeurs d'exemple, rendus avec ses widgets. Le
    /// deck de travail garde les paramètres et variables du deck ; à la place
    /// de son thème, il porte tous les jetons du thème édité (le brouillon,
    /// gardé d'un pack à l'autre). Il ne porte pas d'assets (un pack n'en
    /// enregistre pas).
    pub fn edit_pack(&mut self, id: Uuid) -> Result<()> {
        // L'historique ouvert se ferme d'abord ; l'aide ne passe jamais en mode pack.
        self.ensure_editable()?;
        // Le pack du deck seul (celui d'un import) part de sa version embarquée.
        let stored = self.pack_base(id)?;
        let deck_only = !stored.manifest.scope.is_library();
        let version = stored.version.clone();
        let slides = layout_slides(&stored.manifest, &stored.files);
        // Le brouillon du thème : gardé d'un pack à l'autre, sinon le thème du deck.
        let (theme, tokens) = match &self.mode {
            Mode::Pack { theme, .. } => (theme.clone(), self.edited_tokens()),
            _ => self.initial_theme_edit()?,
        };
        // Sans thème, avec tous les jetons du thème édité : les surcharges du
        // deck ne s'y appliquent pas, les layouts se voient avec le thème pur.
        let mut deck = styled_like(self.user_deck(), slides);
        deck.theme = None;
        deck.tokens = tokens_value(tokens);
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
        let working = Document::new(sources, None, &self.doc_defaults);
        let edit = PackMode {
            pack: id,
            name: stored.manifest.name,
            kind,
            builtin: PackLibrary::is_builtin(id),
            base: version.clone(),
            parent: Some(version),
            session_version: None,
            deck_only,
        };
        // Les widgets du pack s'installent avant de changer de document : en
        // cas d'échec, rien ne change.
        self.install_widgets(widgets)?;
        self.mode = match std::mem::replace(&mut self.mode, Mode::Deck) {
            // Déjà en édition : on change de pack, le deck reste de côté.
            Mode::Pack { stash, .. } => {
                self.doc = working;
                Mode::Pack { stash, edit, scratch: PackScratch::default(), theme, switches: Vec::new() }
            }
            Mode::Deck => Mode::Pack {
                stash: Box::new(std::mem::replace(&mut self.doc, working)),
                edit,
                scratch: PackScratch::default(),
                theme,
                switches: Vec::new(),
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
        };
        // Le deck de travail est neuf : la base du brouillon y est embarquée.
        self.embed_theme_base();
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
        self.realign_theme_edit();
        // Une copie vide l'historique du document (`save_pack`) : aucune
        // étape ne ramène à un autre pack que celui de l'édition.
        let Some((ns, pin)) = self.edited_pin().map(|(ns, pin)| (ns.clone(), pin.clone())) else { return Ok(()) };
        let held = self.doc.sources.packs.clone();
        let Mode::Pack { stash, edit, .. } = &mut self.mode else { return Ok(()) };
        if pin.version == edit.base {
            return Ok(());
        }
        let files = held
            .get(&pin.version)
            .cloned()
            .context(PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        restore_session_version(&self.library, pin.id, &files);
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

    /// Quitte le mode pack ; les changements non enregistrés sont perdus.
    pub fn close_pack(&mut self) -> Result<()> {
        match std::mem::replace(&mut self.mode, Mode::Deck) {
            Mode::Pack { stash, .. } => {
                self.doc = *stash;
                self.sync_widgets()?;
            }
            Mode::Deck => {}
            help @ Mode::Help { .. } => self.mode = help,
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
