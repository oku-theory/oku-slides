//! Rendu d'un deck pour la CLI, avec les widgets sondés de son pack
//! ([`ProbedWidgets`]). Le rendu se fait en mémoire, ou par le processus de
//! rendu quand le pack a des widgets valides : le code d'un pack reçu y est
//! isolé et borné (délai, mémoire : spec t-327).

use color_eyre::eyre::Result;

use oku_slides_core::{Deck, SourceBundle};
use oku_slides_i18n::Localizer;
use oku_slides_platform::Platform;
use oku_slides_render::{Compiled, Diagnostic, Fonts, Layout, RenderError, Renderer};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot};
use oku_slides_session::ProbedWidgets;
use oku_slides_session::{ThemeLibrary, theme_tokens};

/// Jetons d'un thème, par groupe.
type Tokens = serde_json::Map<String, serde_json::Value>;

/// Un rendu, fait en mémoire ou par le processus de rendu.
pub(crate) enum Rendered {
    Local(Compiled),
    /// L'emplacement vit autant que le rendu : ses pages s'exportent à la demande.
    Remote {
        compiled: RemoteCompiled,
        _slot: RenderSlot,
    },
}

impl Rendered {
    pub(crate) fn page_count(&self) -> usize {
        match self {
            Self::Local(c) => c.page_count(),
            Self::Remote { compiled, .. } => compiled.page_count(),
        }
    }

    pub(crate) fn warnings(&self) -> &[Diagnostic] {
        match self {
            Self::Local(c) => &c.warnings,
            Self::Remote { compiled, .. } => &compiled.warnings,
        }
    }

    pub(crate) fn layout(&self) -> &Layout {
        match self {
            Self::Local(c) => c.layout(),
            Self::Remote { compiled, .. } => compiled.layout(),
        }
    }

    pub(crate) fn svg(&self, index: usize) -> Result<String, RenderError> {
        match self {
            Self::Local(c) => c.svg(index),
            Self::Remote { compiled, .. } => compiled.svg(index),
        }
    }

    pub(crate) fn pdf(&self) -> Result<Vec<u8>, RenderError> {
        match self {
            Self::Local(c) => c.pdf(),
            Self::Remote { compiled, .. } => compiled.pdf(),
        }
    }
}

/// Rend `deck` avec `widgets`, ceux du pack embarqué dans `bundle` (voir [`ProbedWidgets::probe`]).
///
/// `deck` est la vue de `bundle`, thème éventuellement changé (sa version
/// embarquée dans `bundle`). Un pack à widgets valides passe par le processus
/// de rendu ; sinon, le rendu se fait en mémoire avec la base. Le thème se
/// rend dans sa version embarquée, sinon le thème par défaut.
pub(crate) fn render(
    platform: &Platform,
    l: &Localizer,
    fonts: Fonts,
    widgets: &ProbedWidgets,
    bundle: &SourceBundle,
    deck: &Deck,
) -> Result<Rendered> {
    let tokens = theme_tokens(&ThemeLibrary::user(platform), &bundle.themes, deck);
    if !widgets.has_packs() {
        return local(l, fonts, bundle, deck, tokens);
    }
    let mut slot = widgets.slot(&fonts, l)?;
    slot.set_theme(tokens.as_ref())?;
    let compiled = slot.render(deck, &bundle.assets)?;
    Ok(Rendered::Remote { compiled, _slot: slot })
}

/// Rendu en mémoire, avec les widgets de base et les jetons de thème `tokens`.
fn local(l: &Localizer, fonts: Fonts, bundle: &SourceBundle, deck: &Deck, tokens: Option<Tokens>) -> Result<Rendered> {
    let mut renderer = Renderer::new(fonts)?;
    renderer.set_language(l)?;
    renderer.set_theme(tokens)?;
    Ok(Rendered::Local(renderer.render(deck, &bundle.assets)?))
}
