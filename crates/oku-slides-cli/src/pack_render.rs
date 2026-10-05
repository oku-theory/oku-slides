//! Rendu d'un deck pour la CLI : en mémoire, ou par le processus de rendu
//! quand le pack qu'il embarque a des widgets : chaque fichier y est sondé à
//! part, les widgets cassés écartés (diagnostics au journal). Le code d'un
//! pack reçu y est isolé et borné (délai, mémoire : spec t-327).

use color_eyre::eyre::Result;

use oku_slides_core::{Deck, SourceBundle};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_platform::errors::one_line;
use oku_slides_render::{Compiled, Diagnostic, Fonts, Layout, PackWidgets, RenderError, Renderer};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot, RenderWorker};

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

/// Rend `deck` avec les widgets du pack embarqué dans `bundle`.
///
/// `deck` est la vue de `bundle`, thème éventuellement changé. Un pack à
/// widgets passe par le processus de rendu ; sans widgets, le rendu se fait
/// en mémoire. Version absente des sources ou pack cassé : journalisés, la
/// base, comme dans l'app.
pub(crate) fn render(
    platform: &Platform,
    l: &Localizer,
    fonts: Fonts,
    bundle: &SourceBundle,
    deck: &Deck,
) -> Result<Rendered> {
    let Some(r) = &deck.templates else { return local(l, fonts, bundle, deck) };
    let Some(files) = bundle.packs.get(&r.version) else {
        tracing::warn!(pack = %r.id, version = %r.version, "{}", l.tr("session-pack-missing"));
        return local(l, fonts, bundle, deck);
    };
    if files.widgets().is_empty() {
        return local(l, fonts, bundle, deck);
    }
    let worker = RenderWorker::new(platform.render_launch.clone(), platform.settings.render.clone());
    let probed = PackWidgets::new(r.version.hex(), files.widgets().clone()).and_then(|pack| worker.probe_pack(&pack));
    let probe = match probed {
        Ok(probe) => probe,
        Err(e) => {
            tracing::warn!(pack = %r.id, error = %one_line(&e), "{}", e.localize(l));
            return local(l, fonts, bundle, deck);
        }
    };
    for (name, file) in probe.files.iter().filter(|(_, p)| !p.is_valid()) {
        for d in &file.diagnostics {
            tracing::warn!(pack = %r.id, widget = %name, "{}", d.localize(l));
        }
    }
    let Some(pack) = probe.pack else { return local(l, fonts, bundle, deck) };
    let mut slot = worker.slot(&fonts, l, probe.registry.clone())?;
    slot.set_widgets(Some(&pack), probe.registry)?;
    let compiled = slot.render(deck, &bundle.assets)?;
    Ok(Rendered::Remote { compiled, _slot: slot })
}

/// Rendu en mémoire, avec les widgets de base.
fn local(l: &Localizer, fonts: Fonts, bundle: &SourceBundle, deck: &Deck) -> Result<Rendered> {
    let mut renderer = Renderer::new(fonts)?;
    renderer.set_language(l)?;
    Ok(Rendered::Local(renderer.render(deck, &bundle.assets)?))
}
