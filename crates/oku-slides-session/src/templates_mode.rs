//! Édition d'un pack de modèles : le deck attend de côté.
//!
//! On édite la version courante du pack. Une version par session
//! d'édition : le premier enregistrement crée une version (`parent` = la
//! courante au début de la session), la rend courante, l'embarque dans le
//! deck mis de côté, qui y passe (`SetTemplates`, une seule étape
//! d'annulation) ; les suivants la remplacent (`Library::replace_version`,
//! même parente, l'intermédiaire quitte la bibliothèque et les sources des
//! deux documents si l'enregistrement a rejoint son étape d'annulation ;
//! sinon il y reste, annuler y revenant),
//! jusqu'à la sortie du mode modèles.
//! « Enregistrer comme copie » : un nouvel id, sans parente, et la session
//! continue sur la copie. Un pack intégré suit la même règle, sous son id.
//! Un pack du deck seul (import) part de la version qu'embarque le deck ;
//! ses versions, copie comprise, restent dans le deck (`templates_library`).
//! Le thème du deck s'y édite en brouillon (voir `theme_edit`).

use std::collections::BTreeMap;
use std::sync::Arc;

use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{Deck, PackFiles, PackRef, PackVersion, Slide, SourceBundle};
use oku_slides_i18n::Localize;

use crate::error::{
    InHistorySnafu, NotEditingTemplatesSnafu, PackJsonSnafu, PackVersionMissingSnafu, ReadOnlySnafu, RenderSnafu,
    SourcesSnafu, UnknownPackSnafu,
};
use crate::library::PackLibrary;
use crate::library_kind::StoredVersion;
use crate::pack::{DEFAULT_PACK, PackInfo, PackManifest, PackScope, Template, now_created};
use crate::template_scratch::TemplateScratch;
use crate::templates_library::{restore_session_version, write_session_version};
use crate::theme_edit::tokens_value;
use crate::{Document, Mode, Result, Session, TemplateMode, TemplateView, TemplatesView, new_renderer};

/// Préfixe de la clé d'étape d'annulation des enregistrements du pack.
const ADOPT_MERGE: &str = "templates-adopt";

/// Clé d'étape d'annulation d'une session d'édition : son pack et la parente
/// de sa version. Ses enregistrements n'en font qu'une ; une autre session
/// (autre parente) ou une copie (autre id) en fait une autre.
pub(crate) fn adopt_merge_key(id: Uuid, parent: Option<&PackVersion>) -> String {
    format!("{ADOPT_MERGE}:{id}:{}", parent.map_or("", PackVersion::as_str))
}

/// Aperçus de modèles en cache, avec ce dont ils dépendent : la version du
/// pack et le style du deck (le deck d'aperçu les porte).
pub(crate) struct Previews {
    pub(crate) key: PackRef,
    pub(crate) deck: Deck,
    pub(crate) views: Arc<[TemplateView]>,
}

/// La slide d'un modèle, nommée comme lui, pour l'éditer.
fn template_slide(t: Template) -> Slide {
    let mut s = t.slide;
    s.id = t.id;
    s.name = Some(t.name);
    s.description = (!t.description.is_empty()).then_some(t.description);
    s
}

/// Un deck qui a le style de `base` (thème, jetons, paramètres, variables,
/// pack de modèles) et ces slides.
pub(crate) fn styled_like(base: &Deck, slides: Vec<Slide>) -> Deck {
    Deck {
        version: base.version,
        theme: base.theme.clone(),
        params: base.params.clone(),
        vars: base.vars.clone(),
        tokens: base.tokens.clone(),
        templates: base.templates.clone(),
        slides,
    }
}

/// Même style : même thème, mêmes jetons, paramètres et variables.
fn same_style(a: &Deck, b: &Deck) -> bool {
    a.theme == b.theme && a.tokens == b.tokens && a.params == b.params && a.vars == b.vars
}

impl Session {
    /// Passe en édition de la version courante du pack `id`.
    ///
    /// Ses modèles deviennent les slides, lus et rendus avec ses widgets. Le
    /// deck de travail garde les paramètres et variables du deck ; à la place
    /// de son thème, il porte tous les jetons du thème édité (le brouillon,
    /// gardé d'un pack à l'autre). Les modèles ne portent pas d'assets (un
    /// pack n'en enregistre pas).
    pub fn edit_templates(&mut self, id: Uuid) -> Result<()> {
        // L'historique ouvert se ferme d'abord ; l'aide ne passe jamais aux modèles.
        self.ensure_editable()?;
        // Le pack du deck seul (celui d'un import) part de sa version embarquée.
        let stored = self.templates_base(id)?;
        let deck_only = !stored.manifest.scope.is_library();
        let r = PackRef { id, version: stored.version.clone() };
        let slides = stored.manifest.templates.iter().cloned().map(template_slide).collect();
        // Le brouillon du thème : gardé d'un pack à l'autre, sinon le thème du deck.
        let (theme, tokens) = match &self.mode {
            Mode::Templates { theme, .. } => (theme.clone(), self.edited_tokens()),
            _ => self.initial_theme_edit()?,
        };
        // Sans thème, avec tous les jetons du thème édité : les surcharges du
        // deck ne s'y appliquent pas, les modèles se voient avec le thème pur.
        let mut deck = styled_like(self.user_deck(), slides);
        deck.theme = None;
        deck.tokens = tokens_value(tokens);
        deck.params.title = Some(stored.manifest.name.clone());
        deck.templates = Some(r.clone());
        let packs = BTreeMap::from([(r.version.clone(), stored.files.clone())]);
        let (widgets, notice) = self.resolve_widgets(Some(&r), &packs)?;
        let mut sources = SourceBundle::new(deck, widgets.registry.clone()).context(SourcesSnafu)?;
        sources.add_pack(stored.files);
        let templates = Document::new(sources, None, &self.doc_defaults);
        let edit = TemplateMode {
            set: id,
            name: stored.manifest.name,
            builtin: PackLibrary::is_builtin(id),
            base: r.version.clone(),
            parent: Some(r.version),
            session_version: None,
            deck_only,
        };
        // Les widgets du pack s'installent avant de changer de document : en
        // cas d'échec, rien ne change.
        self.install_widgets(widgets)?;
        self.mode = match std::mem::replace(&mut self.mode, Mode::Deck) {
            // Déjà en édition : on change de pack, le deck reste de côté.
            Mode::Templates { stash, .. } => {
                self.doc = templates;
                Mode::Templates { stash, edit, scratch: TemplateScratch::default(), theme, switches: Vec::new() }
            }
            Mode::Deck => Mode::Templates {
                stash: Box::new(std::mem::replace(&mut self.doc, templates)),
                edit,
                scratch: TemplateScratch::default(),
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

    /// Le `pack.json` que montre `deck`, le deck des modèles en cours d'édition.
    ///
    /// `kept` : la dernière version enregistrée, dont la description, les
    /// valeurs d'exemple et la portée restent ; sans elle, la portée de
    /// l'édition (`deck_only`).
    fn edited_manifest(
        &self,
        deck: &Deck,
        (id, parent): (Uuid, Option<PackVersion>),
        deck_only: bool,
        kept: Option<PackManifest>,
    ) -> PackManifest {
        let fallback = if deck_only { PackScope::Deck } else { PackScope::Library };
        let scope = kept.as_ref().map_or(fallback, |m| m.scope);
        let (description, examples) = kept.map(|m| (m.description, m.examples)).unwrap_or_default();
        let name = deck
            .params
            .title
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| self.localizer.tr("templates-default-name"));
        let templates = deck
            .slides
            .iter()
            .map(|s| {
                let mut slide = s.clone();
                let name = slide
                    .name
                    .take()
                    .or_else(|| slide.params.text("title").map(str::to_owned))
                    .unwrap_or_else(|| self.localizer.tr("template-default-name"));
                let description = slide.description.take().unwrap_or_default();
                slide.id = format!("tpl-{}", s.id);
                Template { id: s.id.clone(), name, description, slide }
            })
            .collect();
        PackManifest { id, name, description, parent, created: now_created(), scope, templates, examples }
    }

    /// Enregistre le pack édité dans la version de cette session d'édition.
    ///
    /// Créée au premier enregistrement, remplacée ensuite, elle devient
    /// courante ; le deck mis de côté l'embarque et y passe. `as_new` : une
    /// copie (nouvel id, sans parente), sur laquelle la session continue,
    /// sans historique d'annulation (un annuler ne ramène pas à l'original).
    /// Le brouillon du thème est enregistré aussi (`save_theme_edit`) ; le
    /// deck mis de côté passe à son thème dans l'étape du pack.
    /// Un rendu raté ensuite n'est pas une erreur : la version reste, un avis
    /// donne la cause (choix « diagnostics + succès », comme `set_widget`).
    pub fn save_templates(&mut self, as_new: bool) -> Result<()> {
        let Mode::Templates { edit, .. } = &self.mode else {
            return NotEditingTemplatesSnafu.fail()?;
        };
        let edit = edit.clone();
        // Le deck modifié (titre de la copie, version) remplace la vue d'un
        // coup, sans étape d'annulation : ce n'est pas une édition.
        let mut deck = self.doc.sources.deck().clone();
        if as_new && deck.params.title.as_deref() == Some(edit.name.as_str()) {
            let name = self.localizer.tr_with("templates-copy-name", &[("name", edit.name.as_str().into())]);
            deck.params.title = Some(name);
        }
        // Description, valeurs d'exemple et widgets : ceux de la dernière version enregistrée.
        let base = self.doc.sources.packs.get(&edit.base);
        let kept = base.and_then(|f| PackManifest::from_files(f).ok());
        let widgets = base.map(|f| f.widgets().clone()).unwrap_or_default();
        let (id, parent) = if as_new { (Uuid::new_v4(), None) } else { (edit.set, edit.parent.clone()) };
        let manifest = self.edited_manifest(&deck, (id, parent), edit.deck_only, kept);
        let files = manifest.to_files(widgets).context(PackJsonSnafu { id })?;
        self.adopt_session_version(deck, files, &edit, as_new, None)?;
        // Le brouillon du thème, s'il change le thème du deck, rejoint l'étape du pack.
        if let Some(theme) = self.save_theme_edit()? {
            self.stash_adopt_theme(theme)?;
        }
        if as_new {
            // Une copie est un autre pack : son document repart sans historique,
            // un annuler ne ramène jamais à l'original (l'édition suit la copie).
            self.doc.sources.clear_undo();
        }
        self.doc.dirty = false;
        // La version est adoptée : un rendu raté ne l'annule pas, son avis le dit.
        match self.rerender() {
            Err(e) if e.render_error().is_some() => {
                tracing::warn!(error = %e.one_line(), "modèles enregistrés, rendu en échec");
                self.set_notice(e.localize(&self.localizer));
                Ok(())
            }
            other => other,
        }
    }

    /// `files` devient la version de la séance d'édition `edit`.
    ///
    /// Créée la première fois (ou toujours, pour une copie `as_new`),
    /// remplacée ensuite, rendue courante dans la bibliothèque (jamais écrite
    /// pour un pack du deck seul) ; `deck` (le document des modèles)
    /// y passe en une étape `step` (silencieux sans), et le deck mis de côté
    /// en une étape d'annulation.
    /// Sans rendu : l'appelant rend ensuite le document des modèles.
    pub(crate) fn adopt_session_version(
        &mut self,
        mut deck: Deck,
        files: PackFiles,
        edit: &TemplateMode,
        as_new: bool,
        step: Option<&str>,
    ) -> Result<PackVersion> {
        let manifest = PackManifest::from_files(&files).context(PackJsonSnafu { id: edit.set })?;
        let (id, parent) = (manifest.id, manifest.parent.clone());
        // Une version par séance : la première est créée, les suivantes la remplacent.
        let replaced = edit.session_version.as_ref().filter(|_| !as_new);
        let version = write_session_version(&self.library, id, replaced, &files)?;
        let deck_only = !manifest.scope.is_library();
        let r = PackRef { id, version: version.clone() };
        let merge = adopt_merge_key(id, parent.as_ref());
        // Le document des modèles passe à la nouvelle version : une étape pour
        // une écriture de widget (clé de séance, ou groupe du tour), rien pour
        // un enregistrement. Ses versions restent dans ses sources : une
        // étape peut y ramener.
        self.doc.sources.add_pack(files.clone());
        deck.templates = Some(r.clone());
        match step {
            Some(label) => self.doc.sources.apply(deck, label, Some(&merge)),
            None => self.doc.sources.apply_silent(deck),
        }
        .context(SourcesSnafu)?;
        let label = self.localizer.tr_with("templates-adopt-label", &[("name", manifest.name.as_str().into())]);
        if let Mode::Templates { stash, edit, .. } = &mut self.mode {
            // Le deck mis de côté embarque la version et y passe, en une étape d'annulation.
            stash.sources.add_pack(files);
            let mut user = stash.sources.deck().clone();
            user.templates = Some(r);
            let steps = stash.sources.undo_steps();
            stash.sources.apply(user, &label, Some(&merge)).context(SourcesSnafu)?;
            stash.dirty = true;
            if let Some(old) = replaced.filter(|old| **old != version && Some(*old) != parent.as_ref()) {
                if stash.sources.undo_steps() == steps {
                    // Fusionnée dans l'étape qui l'avait introduite : plus
                    // atteignable depuis le deck mis de côté.
                    stash.sources.packs.remove(old);
                } else if let Some(files) = stash.sources.packs.get(old) {
                    // Nouvelle étape : annuler y revient ; la bibliothèque la retrouve.
                    restore_session_version(&self.library, id, files);
                }
            }
            *edit = TemplateMode {
                set: id,
                name: manifest.name,
                builtin: PackLibrary::is_builtin(id),
                base: version.clone(),
                parent,
                session_version: Some(version.clone()),
                deck_only,
            };
        }
        self.sync_widgets()?;
        self.template_cache = None;
        Ok(version)
    }

    /// Réaligne l'édition des modèles sur la version du document, après annuler ou rétablir.
    ///
    /// La bibliothèque retrouve cette version (une écriture de la séance l'a
    /// peut-être remplacée), sans qu'elle devienne courante ; celle d'un pack
    /// du deck seul n'y entre jamais. Le deck mis de côté passe à cette
    /// version, dans l'étape de la séance.
    pub(crate) fn realign_templates(&mut self) -> Result<()> {
        self.realign_theme_edit();
        let Some(r) = self.doc.sources.deck().templates.clone() else { return Ok(()) };
        let Mode::Templates { stash, edit, .. } = &mut self.mode else { return Ok(()) };
        // Une copie vide l'historique du document (`save_templates`) : aucune
        // étape ne ramène à un autre pack que celui de l'édition.
        debug_assert_eq!(r.id, edit.set, "annuler a ramené le document à un autre pack");
        if r.id != edit.set || r.version == edit.base {
            return Ok(());
        }
        let files = self
            .doc
            .sources
            .packs
            .get(&r.version)
            .cloned()
            .context(PackVersionMissingSnafu { id: r.id, version: r.version.clone() })?;
        restore_session_version(&self.library, r.id, &files);
        edit.base = r.version.clone();
        edit.session_version = (edit.parent.as_ref() != Some(&r.version)).then(|| r.version.clone());
        let label = self.localizer.tr_with("templates-adopt-label", &[("name", edit.name.as_str().into())]);
        let merge = adopt_merge_key(edit.set, edit.parent.as_ref());
        stash.sources.add_pack(files);
        let mut user = stash.sources.deck().clone();
        user.templates = Some(r);
        stash.sources.apply(user, &label, Some(&merge)).context(SourcesSnafu)?;
        stash.dirty = true;
        self.template_cache = None;
        Ok(())
    }

    /// Quitte l'édition des modèles ; les changements non enregistrés sont perdus.
    pub fn close_templates(&mut self) -> Result<()> {
        match std::mem::replace(&mut self.mode, Mode::Deck) {
            Mode::Templates { stash, .. } => {
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
        self.template_cache = None;
        self.ui.slide = None;
        self.ui = std::mem::take(&mut self.ui).sanitized(false);
        self.rerender()?;
        Ok(())
    }

    /// La version d'un pack à montrer : celle qu'embarque le deck pour son
    /// propre pack, la courante pour un autre ; un pack disparu : celui du deck.
    fn shown_version(&self, set: Option<Uuid>) -> Option<StoredVersion<PackManifest>> {
        let embedded = |r: &PackRef| {
            let files = self.user_blobs().packs.get(&r.version)?.clone();
            let manifest = PackManifest::from_files(&files).ok()?;
            let received = self.library.is_received(r.id, &r.version);
            Some(StoredVersion { version: r.version.clone(), files, manifest, received })
        };
        let own = self.deck_pack().cloned();
        match (set, own) {
            (Some(id), Some(r)) if id != r.id => self.library.current_version(id).or_else(|| embedded(&r)),
            (Some(id), None) => self.library.current_version(id),
            (_, Some(r)) => embedded(&r).or_else(|| self.library.current_version(r.id)),
            (None, None) => self.library.current_version(DEFAULT_PACK),
        }
    }

    /// Aperçus d'un pack de modèles, rendus avec le style du deck (en mode modèles, du brouillon du thème).
    ///
    /// En cache tant que rien ne change (version du pack, thème, jetons,
    /// paramètres, variables). Un pack qui fait échouer l'aperçu (délai,
    /// mémoire, arrêt) : repli sur la base, avec son avis, et aperçu refait
    /// une fois.
    pub fn templates_view(&mut self, set: Option<Uuid>) -> Result<TemplatesView> {
        let shown = self.shown_version(set).context(UnknownPackSnafu { id: set.unwrap_or(DEFAULT_PACK) })?;
        let key = PackRef { id: shown.manifest.id, version: shown.version.clone() };
        let cached = match &self.template_cache {
            Some(p) if p.key == key && same_style(&p.deck, self.style_deck()) => Some(p.views.clone()),
            _ => None,
        };
        let views = match cached {
            Some(views) => views,
            None => {
                let templates = shown.manifest.templates.clone();
                let deck = styled_like(self.style_deck(), templates.iter().map(|t| t.slide.clone()).collect());
                let views = self.with_fallback(|s| s.render_previews(&deck, &templates))?;
                self.template_cache = Some(Previews { key, deck, views: views.clone() });
                views
            }
        };
        let builtin = PackLibrary::is_builtin(shown.manifest.id);
        let info = PackInfo::of(&shown.manifest, shown.version, builtin, shown.received);
        Ok(TemplatesView { set: info, sets: self.library.packs(), templates: views })
    }

    /// Rend `deck`, les modèles `templates` au style du deck, dans l'emplacement des aperçus.
    fn render_previews(&mut self, deck: &Deck, templates: &[Template]) -> Result<Arc<[TemplateView]>> {
        let pack = self.usable_pack()?;
        let tokens = self.theme_tokens(deck);
        let renderer = match &mut self.template_renderer {
            Some(r) => r,
            None => self.template_renderer.insert(new_renderer(
                &self.worker,
                self.render_fonts.clone(),
                &self.localizer,
                pack.as_ref(),
                &self.registry,
            )?),
        };
        let compiled = crate::deck_theme::render_themed(renderer, deck, tokens.as_ref(), &self.doc.sources.assets)
            .context(RenderSnafu)?;
        let layout = compiled.layout();
        templates
            .iter()
            .map(|t| {
                let page = layout.page_of(&t.slide.id).unwrap_or(1);
                let svg = compiled.svg(page - 1).context(RenderSnafu)?;
                Ok(TemplateView { template: t.clone(), svg: svg.into() })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "templates_mode_tests.rs"]
mod templates_mode_tests;
