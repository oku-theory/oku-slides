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

use std::collections::BTreeMap;
use std::sync::Arc;

use snafu::{OptionExt, ResultExt};
use uuid::Uuid;

use oku_slides_core::{Deck, PackFiles, PackRef, PackVersion, Slide, SourceBundle};

use crate::error::{
    InHistorySnafu, NotEditingTemplatesSnafu, PackJsonSnafu, RenderSnafu, SourcesSnafu, UnknownPackSnafu,
};
use crate::library::{Library, StoredVersion};
use crate::pack::{DEFAULT_PACK, PackInfo, PackManifest, Template, now_created};
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
    /// deck garde son thème, ses jetons et ses variables ; les modèles ne
    /// portent pas d'assets (un pack n'en enregistre pas).
    pub fn edit_templates(&mut self, id: Uuid) -> Result<()> {
        if matches!(self.mode, Mode::History { .. }) {
            // L'historique ouvert se ferme d'abord (`handle` refuse avant d'arriver ici).
            return InHistorySnafu.fail()?;
        }
        let stored = self.library.current_version(id).context(UnknownPackSnafu { id })?;
        let r = PackRef { id, version: stored.version.clone() };
        let slides = stored.manifest.templates.iter().cloned().map(template_slide).collect();
        let mut deck = styled_like(self.user_deck(), slides);
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
            builtin: Library::is_builtin(id),
            base: r.version.clone(),
            parent: Some(r.version),
            session_version: None,
        };
        // Les widgets du pack s'installent avant de changer de document : en
        // cas d'échec, rien ne change.
        self.install_widgets(widgets)?;
        self.mode = match std::mem::replace(&mut self.mode, Mode::Deck) {
            // Déjà en édition : on change de pack, le deck reste de côté.
            Mode::Templates { stash, .. } => {
                self.doc = templates;
                Mode::Templates { stash, edit }
            }
            Mode::Deck => Mode::Templates { stash: Box::new(std::mem::replace(&mut self.doc, templates)), edit },
            // L'historique ouvert se ferme d'abord (`handle` refuse avant d'arriver ici).
            history @ Mode::History { .. } => {
                self.mode = history;
                return InHistorySnafu.fail()?;
            }
        };
        if let Some(notice) = notice {
            self.set_notice(notice);
        }
        self.ui.slide = None;
        self.rerender()?;
        Ok(())
    }

    /// Le `pack.json` que montre `deck`, le deck des modèles en cours d'édition.
    fn edited_manifest(&self, deck: &Deck, id: Uuid, parent: Option<PackVersion>, description: String) -> PackManifest {
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
        PackManifest { id, name, description, parent, created: now_created(), templates }
    }

    /// Enregistre le pack édité dans la version de cette session d'édition.
    ///
    /// Créée au premier enregistrement, remplacée ensuite, elle devient
    /// courante ; le deck mis de côté l'embarque et y passe. `as_new` : une
    /// copie (nouvel id, sans parente), sur laquelle la session continue.
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
        // Description et widgets : ceux de la dernière version enregistrée.
        let base = self.doc.sources.packs.get(&edit.base);
        let description =
            base.and_then(|f| PackManifest::from_files(f).ok()).map(|m| m.description).unwrap_or_default();
        let widgets = base.map(|f| f.widgets().clone()).unwrap_or_default();
        let (id, parent) = if as_new { (Uuid::new_v4(), None) } else { (edit.set, edit.parent.clone()) };
        let manifest = self.edited_manifest(&deck, id, parent, description);
        let files = manifest.to_files(widgets).context(PackJsonSnafu { id })?;
        self.adopt_session_version(deck, files, &edit, as_new)?;
        self.rerender()?;
        self.doc.dirty = false;
        Ok(())
    }

    /// `files` devient la version de la séance d'édition `edit`.
    ///
    /// Créée la première fois (ou toujours, pour une copie `as_new`),
    /// remplacée ensuite, rendue courante ; `deck` (le document des modèles)
    /// et le deck mis de côté y passent, celui-ci en une étape d'annulation.
    /// Sans rendu : l'appelant rend ensuite le document des modèles.
    pub(crate) fn adopt_session_version(
        &mut self,
        mut deck: Deck,
        files: PackFiles,
        edit: &TemplateMode,
        as_new: bool,
    ) -> Result<PackVersion> {
        let manifest = PackManifest::from_files(&files).context(PackJsonSnafu { id: edit.set })?;
        let (id, parent) = (manifest.id, manifest.parent.clone());
        // Une version par séance : la première est créée, les suivantes la remplacent.
        let replaced = edit.session_version.as_ref().filter(|_| !as_new);
        let version = match replaced {
            Some(old) => self.library.replace_version(id, old, &files)?,
            None => {
                let version = files.version();
                self.library.add_version(id, &files)?;
                self.library.set_current(id, &version)?;
                version
            }
        };
        let r = PackRef { id, version: version.clone() };
        // Le document des modèles passe à la nouvelle version, sans étape d'annulation.
        self.doc.sources.add_pack(files.clone());
        deck.templates = Some(r.clone());
        self.doc.sources.apply_silent(deck).context(SourcesSnafu)?;
        let label = self.localizer.tr_with("templates-adopt-label", &[("name", manifest.name.as_str().into())]);
        let merge = adopt_merge_key(id, parent.as_ref());
        if let Mode::Templates { stash, edit } = &mut self.mode {
            // Le deck mis de côté embarque la version et y passe, en une étape d'annulation.
            stash.sources.add_pack(files);
            let mut user = stash.sources.deck().clone();
            user.templates = Some(r);
            let steps = stash.sources.undo_steps();
            stash.sources.apply(user, &label, Some(&merge)).context(SourcesSnafu)?;
            stash.dirty = true;
            if let Some(old) = replaced.filter(|old| **old != version && Some(*old) != parent.as_ref()) {
                if stash.sources.undo_steps() == steps {
                    // Fusionnée dans l'étape qui l'avait introduite : plus atteignable.
                    stash.sources.packs.remove(old);
                    self.doc.sources.packs.remove(old);
                } else if let Some(files) = stash.sources.packs.get(old) {
                    // Nouvelle étape : annuler y revient ; la bibliothèque la retrouve.
                    if let Err(e) = self.library.add_version(id, files) {
                        tracing::error!(error = %e.one_line(), "version de séance non rétablie dans la bibliothèque");
                    }
                }
            }
            *edit = TemplateMode {
                set: id,
                name: manifest.name,
                builtin: Library::is_builtin(id),
                base: version.clone(),
                parent,
                session_version: Some(version.clone()),
            };
        }
        self.sync_widgets()?;
        self.template_cache = None;
        Ok(version)
    }

    /// Quitte l'édition des modèles ; les changements non enregistrés sont perdus.
    pub fn close_templates(&mut self) -> Result<()> {
        match std::mem::replace(&mut self.mode, Mode::Deck) {
            Mode::Templates { stash, .. } => {
                self.doc = *stash;
                self.sync_widgets()?;
            }
            Mode::Deck => {}
            // Le deck est de côté pour l'historique : il n'est pas perdu ici.
            history @ Mode::History { .. } => {
                self.mode = history;
                return InHistorySnafu.fail()?;
            }
        }
        self.template_cache = None;
        self.ui.slide = None;
        self.rerender()?;
        Ok(())
    }

    /// La version d'un pack à montrer : celle qu'embarque le deck pour son
    /// propre pack, la courante pour un autre ; un pack disparu : celui du deck.
    fn shown_version(&self, set: Option<Uuid>) -> Option<StoredVersion> {
        let embedded = |r: &PackRef| {
            let files = self.user_sources().packs.get(&r.version)?.clone();
            let manifest = PackManifest::from_files(&files).ok()?;
            Some(StoredVersion { version: r.version.clone(), files, manifest })
        };
        let own = self.deck_pack().cloned();
        match (set, own) {
            (Some(id), Some(r)) if id != r.id => self.library.current_version(id).or_else(|| embedded(&r)),
            (Some(id), None) => self.library.current_version(id),
            (_, Some(r)) => embedded(&r).or_else(|| self.library.current_version(r.id)),
            (None, None) => self.library.current_version(DEFAULT_PACK),
        }
    }

    /// Aperçus d'un pack de modèles, rendus avec le style du deck.
    ///
    /// En cache tant que rien ne change (version du pack, thème, jetons,
    /// paramètres, variables). Un pack qui fait échouer l'aperçu (délai,
    /// mémoire, arrêt) : repli sur la base, avec son avis, et aperçu refait
    /// une fois.
    pub fn templates_view(&mut self, set: Option<Uuid>) -> Result<TemplatesView> {
        let shown = self.shown_version(set).context(UnknownPackSnafu { id: set.unwrap_or(DEFAULT_PACK) })?;
        let key = PackRef { id: shown.manifest.id, version: shown.version.clone() };
        let cached = match &self.template_cache {
            Some(p) if p.key == key && same_style(&p.deck, self.user_deck()) => Some(p.views.clone()),
            _ => None,
        };
        let views = match cached {
            Some(views) => views,
            None => {
                let templates = shown.manifest.templates.clone();
                let deck = styled_like(self.user_deck(), templates.iter().map(|t| t.slide.clone()).collect());
                let views = self.with_fallback(|s| s.render_previews(&deck, &templates))?;
                self.template_cache = Some(Previews { key, deck, views: views.clone() });
                views
            }
        };
        let info: PackInfo = shown.manifest.info(shown.version, Library::is_builtin(shown.manifest.id));
        Ok(TemplatesView { set: info, sets: self.library.packs(), templates: views })
    }

    /// Rend `deck`, les modèles `templates` au style du deck, dans l'emplacement des aperçus.
    fn render_previews(&mut self, deck: &Deck, templates: &[Template]) -> Result<Arc<[TemplateView]>> {
        let pack = self.usable_pack()?;
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
        let compiled = renderer.render(deck, &self.doc.sources.assets).context(RenderSnafu)?;
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
