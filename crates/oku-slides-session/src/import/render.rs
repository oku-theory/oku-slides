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
    pub fn compiled(&self) -> &RemoteCompiled {
        &self.compiled
    }
}

/// Rend le deck de `sources` avec `widgets` et les polices `fonts`.
///
/// Il se rend avec son calque de thème ([`sources_theme_layer`]) : défauts de
/// ses packs, puis sa chaîne de thèmes embarquée, sinon le thème par défaut.
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
