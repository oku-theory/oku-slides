//! Le rendu d'un deck importé, toujours par le processus de rendu.

use oku_slides_core::SourceBundle;
use oku_slides_platform::Platform;
use oku_slides_render::{Fonts, RenderError};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot};

use crate::{ProbedWidgets, ThemeLibrary, theme_tokens};

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
/// Le thème se rend dans sa version embarquée, sinon le thème par défaut.
pub fn render(
    platform: &Platform,
    fonts: &Fonts,
    widgets: &ProbedWidgets,
    sources: &SourceBundle,
) -> Result<RenderedDeck, RenderError> {
    let tokens = theme_tokens(&ThemeLibrary::user(platform), &sources.themes, sources.deck());
    let mut slot = widgets.slot(fonts, &platform.localizer)?;
    slot.set_theme(tokens.as_ref())?;
    let compiled = slot.render(sources.deck(), &sources.assets)?;
    Ok(RenderedDeck { compiled, _slot: slot })
}
