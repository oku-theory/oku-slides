//! Édition d'un ensemble de modèles : le deck attend de côté.

use std::sync::Arc;

use snafu::ResultExt;

use oku_slides_core::{Deck, Slide, SourceBundle};

use crate::error::{InHistorySnafu, NotEditingTemplatesSnafu, RenderSnafu, SourcesSnafu};
use crate::templates::Template;
use crate::{
    DEFAULT_SET, Document, Mode, Result, Session, TemplateMode, TemplateSet, TemplateView, TemplatesView, new_renderer,
};

/// Aperçus de modèles en cache, avec ce dont ils dépendent : l'ensemble et
/// le style du deck (le deck d'aperçu les porte).
pub(crate) struct Previews {
    pub(crate) set: TemplateSet,
    pub(crate) deck: Deck,
    pub(crate) views: Arc<[TemplateView]>,
}

/// Un deck qui a le style de `base` (thème, jetons, paramètres, variables,
/// ensemble de modèles) et ces slides.
fn styled_like(base: &Deck, slides: Vec<Slide>) -> Deck {
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
    /// Passe en édition d'un ensemble : ses modèles deviennent les slides,
    /// lus et rendus avec les widgets de son pack. Le deck garde son thème,
    /// ses jetons et ses variables ; les modèles ne portent pas d'assets (un
    /// ensemble n'en enregistre pas).
    pub fn edit_templates(&mut self, id: &str) -> Result<()> {
        if matches!(self.mode, Mode::History { .. }) {
            // L'historique ouvert se ferme d'abord (`handle` refuse avant d'arriver ici).
            return InHistorySnafu.fail()?;
        }
        let (set, builtin) = self.get_set(id)?;
        let set = set.into_owned();
        let slides = set
            .templates
            .into_iter()
            .map(|t| {
                let mut s = t.slide;
                s.id = t.id;
                s.name = Some(t.name);
                s.description = (!t.description.is_empty()).then_some(t.description);
                s
            })
            .collect();
        let mut deck = styled_like(self.user_deck(), slides);
        deck.params.title = Some(set.name.clone());
        deck.templates = Some(set.id.clone());
        let (widgets, notice) = self.resolve_widgets(Some(&set.id))?;
        let sources = SourceBundle::new(deck, widgets.registry.clone()).context(SourcesSnafu)?;
        let templates = Document::new(sources, None, &self.doc_defaults);
        let edit = TemplateMode { set: set.id, name: set.name, builtin };
        // Les widgets du pack s'installent avant de changer de document : en
        // cas d'échec, rien ne change.
        self.install_widgets(widgets)?;
        self.mode = match std::mem::replace(&mut self.mode, Mode::Deck) {
            // Déjà en édition : on change d'ensemble, le deck reste de côté.
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

    /// L'ensemble tel que le montre `deck`, le deck des modèles en cours d'édition.
    fn edited_set(&self, deck: &Deck, id: String) -> TemplateSet {
        let name = deck
            .params
            .title
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| self.localizer.tr("templates-default-name"));
        let description = self
            .editing()
            .and_then(|m| self.library.get(&m.set))
            .map(|(s, _)| s.into_owned().description)
            .unwrap_or_default();
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
        TemplateSet { id, name, description, templates }
    }

    /// Enregistre l'ensemble édité dans la bibliothèque de l'utilisateur.
    pub fn save_templates(&mut self, as_new: bool) -> Result<()> {
        let Mode::Templates { edit: mode, .. } = &self.mode else {
            return NotEditingTemplatesSnafu.fail()?;
        };
        let copy = as_new || mode.builtin;
        // Le deck modifié (titre de la copie, ensemble) remplace la vue d'un
        // coup, sans étape d'annulation : ce n'est pas une édition.
        let mut deck = self.doc.sources.deck().clone();
        let mut name = deck.params.title.clone().unwrap_or_default();
        if copy && name == mode.name {
            name = self.localizer.tr_with("templates-copy-name", &[("name", name.as_str().into())]);
            deck.params.title = Some(name.clone());
        }
        let id = if copy { self.library.free_id(&name) } else { mode.set.clone() };
        let set = self.edited_set(&deck, id.clone());
        self.library.save(&set)?;
        // Une copie emporte les widgets du pack copié.
        if copy && let Some(source) = self.library.widgets(&mode.set) {
            self.library.save_widgets(&id, &source)?;
        }
        deck.templates = Some(id);
        self.doc.sources.apply_silent(deck).context(SourcesSnafu)?;
        self.sync_widgets()?;
        self.doc.dirty = false;
        if let Mode::Templates { edit, .. } = &mut self.mode {
            *edit = TemplateMode { set: set.id, name: set.name, builtin: false };
        }
        self.template_cache = None;
        self.rerender()?;
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

    /// Aperçus d'un ensemble de modèles, rendus avec le thème, les jetons, les
    /// paramètres et les variables du deck. En cache tant que rien ne change.
    pub fn templates_view(&mut self, set: Option<String>) -> Result<TemplatesView> {
        let id = set.unwrap_or_else(|| self.deck_set().to_owned());
        // Un ensemble disparu (supprimé, ou d'une autre machine) : celui par défaut.
        let (set, builtin) = self.get_set(&id).or_else(|_| self.get_set(DEFAULT_SET))?;
        let base = self.user_deck();
        let views = match &self.template_cache {
            Some(p) if p.set == *set && same_style(&p.deck, base) => p.views.clone(),
            _ => {
                let deck = styled_like(base, set.templates.iter().map(|t| t.slide.clone()).collect());
                let renderer = match &mut self.template_renderer {
                    Some(r) => r,
                    None => self.template_renderer.insert(new_renderer(
                        self.render_fonts.clone(),
                        &self.localizer,
                        self.pack.as_ref(),
                        &self.registry,
                    )?),
                };
                let compiled = renderer.render(&deck, &self.doc.sources.assets).context(RenderSnafu)?;
                let layout = compiled.layout();
                let views = set
                    .templates
                    .iter()
                    .map(|t| {
                        let page = layout.page_of(&t.slide.id).unwrap_or(1);
                        let svg = compiled.svg(page - 1).context(RenderSnafu)?;
                        Ok(TemplateView { template: t.clone(), svg: svg.into() })
                    })
                    .collect::<Result<Arc<[TemplateView]>>>()?;
                self.template_cache = Some(Previews { set: set.clone().into_owned(), deck, views: views.clone() });
                views
            }
        };
        Ok(TemplatesView { set: set.info(builtin), sets: self.library.list(), templates: views })
    }
}
