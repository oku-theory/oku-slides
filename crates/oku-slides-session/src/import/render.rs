//! Le rendu d'un deck importé, toujours par le processus de rendu.

use oku_slides_core::SourceBundle;
use oku_slides_platform::Platform;
use oku_slides_render::Fonts;
use oku_slides_render_worker::{RemoteCompiled, RenderSlot};
use snafu::ResultExt;

use crate::error::RenderSnafu;
use crate::{ProbedWidgets, ThemeLibrary, sources_theme_layer};

/// Un deck rendu ; son emplacement vit autant que lui.
///
/// Ses pages s'exportent à la demande.
#[derive(Debug)]
pub struct RenderedDeck {
    compiled: RemoteCompiled,
    _slot: RenderSlot,
}

impl RenderedDeck {
    /// Le rendu : pages, mise en page, PNG et PDF.
    #[must_use]
    pub fn compiled(&self) -> &RemoteCompiled {
        &self.compiled
    }
}

/// Renders the deck of `sources` with `widgets` and fonts `fonts`.
///
/// It renders with its theme layer ([`sources_theme_layer`]): its packs'
/// defaults, then its embedded theme chain, or else the default theme.
///
/// # Errors
///
/// Fails if the theme layer cannot be composed, or if the render slot or the rendering fails
/// (`Render`).
pub fn render(
    platform: &Platform,
    fonts: &Fonts,
    widgets: &ProbedWidgets,
    sources: &SourceBundle,
) -> crate::Result<RenderedDeck> {
    let layer = sources_theme_layer(&ThemeLibrary::user(platform), sources)?;
    let mut slot = widgets.slot(fonts, &platform.localizer).context(RenderSnafu)?;
    slot.set_theme(Some(&layer)).context(RenderSnafu)?;
    let compiled = slot.render(sources.deck(), &sources.assets).context(RenderSnafu)?;
    Ok(RenderedDeck { compiled, _slot: slot })
}
