//! Rendu d'un deck pour la CLI, avec les widgets sondés de son pack
//! ([`ProbedWidgets`]). Le rendu se fait en mémoire, ou par le processus de
//! rendu quand le pack a des widgets valides : le code d'un pack reçu y est
//! isolé et borné (délai, mémoire : spec t-327).

use color_eyre::eyre::{Result, eyre};

use oku_slides_core::{Deck, SourceBundle};
use oku_slides_i18n::Localize;
use oku_slides_i18n::Localizer;
use oku_slides_platform::Platform;
use oku_slides_render::{Compiled, Diagnostic, Fonts, Layout, RenderError, Renderer};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot};
use oku_slides_session::ProbedWidgets;
use oku_slides_session::{ThemeLibrary, sources_theme_layer};

/// Calque de thème, par groupe.
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

/// Rend le deck de `bundle` avec `widgets`, ceux de son pack embarqué (voir [`ProbedWidgets::probe`]).
///
/// Un deck sans pack se rend avec le registre du noyau, comme l'ensemble vide de la session
/// (`adopt_default_packs` rend la branche locale inatteignable pour un deck ouvert par
/// `sources_with_packs`). Le thème se rend par son calque, composé des versions
/// embarquées (voir [`sources_theme_layer`]).
pub(crate) fn render(
    platform: &Platform,
    l: &Localizer,
    fonts: Fonts,
    widgets: &ProbedWidgets,
    bundle: &SourceBundle,
) -> Result<Rendered> {
    let deck = bundle.deck();
    let layer = sources_theme_layer(&ThemeLibrary::user(platform), bundle).map_err(|e| eyre!(e.localize(l)))?;
    if !widgets.has_packs() {
        return local(l, fonts, bundle, deck, layer);
    }
    let mut slot = widgets.slot(&fonts, l)?;
    slot.set_theme(Some(&layer))?;
    let compiled = slot.render(deck, &bundle.assets)?;
    Ok(Rendered::Remote { compiled, _slot: slot })
}

/// Rendu en mémoire, avec les widgets de base et le calque de thème `layer`.
fn local(l: &Localizer, fonts: Fonts, bundle: &SourceBundle, deck: &Deck, layer: Tokens) -> Result<Rendered> {
    let mut renderer = Renderer::new(fonts)?;
    renderer.set_language(l)?;
    renderer.set_theme(Some(layer))?;
    Ok(Rendered::Local(renderer.render(deck, &bundle.assets)?))
}
