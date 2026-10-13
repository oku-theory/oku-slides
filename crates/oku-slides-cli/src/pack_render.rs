//! Widgets et rendu d'un deck pour la CLI. Le pack qu'il embarque est sondé
//! par le processus de rendu ([`DeckWidgets::probe`]) : chaque fichier à part,
//! les widgets cassés écartés (diagnostics au journal) ; le registre sondé
//! valide le deck (t-355). Une commande n'a pas de « rendu suivant » : les
//! fichiers laissés non sondés (budget épuisé) sont sondés de nouveau, tant
//! qu'un appel en sonde au moins un, en [`PROBE_ROUNDS`] appels au plus. Le rendu se fait en mémoire, ou par ce processus
//! quand le pack a des widgets valides. Le code d'un pack reçu y est isolé et
//! borné (délai, mémoire : spec t-327).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use color_eyre::eyre::Result;

use oku_slides_core::{Deck, PackFiles, PackVersion, SourceBundle, WidgetFile, WidgetRegistry};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_platform::Platform;
use oku_slides_platform::errors::one_line;
use oku_slides_render::{Compiled, Diagnostic, Fonts, Layout, PackWidgets, RenderError, Renderer};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot, RenderWorker};
use oku_slides_session::{ThemeLibrary, theme_tokens};

/// Appels de sonde d'un pack au plus. Les sondes faites sont gardées par le
/// processus de rendu : un appel qui ne fait pas baisser le nombre de fichiers
/// non sondés (processus indisponible) arrête la relance avant cette borne.
const PROBE_ROUNDS: usize = 8;

/// Appelle `probe` tant que des fichiers restent non sondés (`unprobed`) et
/// que chaque appel en fait baisser le nombre, en [`PROBE_ROUNDS`] appels au
/// plus ; la première erreur arrête tout.
fn probe_rounds<T, E>(mut probe: impl FnMut() -> Result<T, E>, unprobed: impl Fn(&T) -> usize) -> Result<T, E> {
    let mut last = probe()?;
    for _ in 1..PROBE_ROUNDS {
        let left = unprobed(&last);
        if left == 0 {
            break;
        }
        let next = probe()?;
        let progressed = unprobed(&next) < left;
        last = next;
        if !progressed {
            break;
        }
    }
    Ok(last)
}

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

/// Les widgets d'un deck : le registre sondé (la base surchargée par les
/// widgets valides de son pack) et, pour un pack à widgets valides, de quoi
/// le rendre par le processus de rendu.
pub(crate) struct DeckWidgets {
    /// Le registre qui valide le deck et ses sources.
    pub(crate) registry: Arc<WidgetRegistry>,
    /// Fichiers du pack toujours non sondés après la relance (budget épuisé, lien indisponible).
    pub(crate) unprobed: BTreeSet<WidgetFile>,
    remote: Option<(RenderWorker, PackWidgets)>,
}

impl DeckWidgets {
    /// Sonde le pack que `deck` référence, pris dans `packs`.
    ///
    /// Sans référence, ou sans widgets : la base. Version absente de `packs`
    /// ou pack cassé : journalisés, la base, comme dans l'app.
    pub(crate) fn probe(
        platform: &Platform,
        l: &Localizer,
        base: &Arc<WidgetRegistry>,
        deck: &Deck,
        packs: &BTreeMap<PackVersion, PackFiles>,
    ) -> Self {
        let base_only = || Self { registry: base.clone(), unprobed: BTreeSet::new(), remote: None };
        let Some(r) = &deck.templates else { return base_only() };
        let Some(files) = packs.get(&r.version) else {
            tracing::warn!(pack = %r.id, version = %r.version, "{}", l.tr("session-pack-missing"));
            return base_only();
        };
        if files.widgets().is_empty() {
            return base_only();
        }
        let worker = RenderWorker::new(platform.render_launch.clone(), platform.settings.render.clone());
        let probed = PackWidgets::new(r.version.hex(), files.widgets().clone())
            .and_then(|pack| probe_rounds(|| worker.probe_pack(&pack), |p| p.unprobed.len()));
        let probe = match probed {
            Ok(probe) => probe,
            Err(e) => {
                tracing::warn!(pack = %r.id, error = %one_line(&e), "{}", e.localize(l));
                return base_only();
            }
        };
        for (name, file) in probe.files.iter().filter(|(_, p)| !p.is_valid()) {
            for d in &file.diagnostics {
                tracing::warn!(pack = %r.id, widget = %name, "{}", d.localize(l));
            }
        }
        match probe.pack {
            Some(pack) => Self { registry: probe.registry, unprobed: probe.unprobed, remote: Some((worker, pack)) },
            None => Self { unprobed: probe.unprobed, ..base_only() },
        }
    }
}

/// Rend `deck` avec `widgets`, ceux du pack embarqué dans `bundle` (voir [`DeckWidgets::probe`]).
///
/// `deck` est la vue de `bundle`, thème éventuellement changé (sa version
/// embarquée dans `bundle`). Un pack à widgets valides passe par le processus
/// de rendu ; sinon, le rendu se fait en mémoire avec la base. Le thème se
/// rend dans sa version embarquée, sinon le thème par défaut.
pub(crate) fn render(
    platform: &Platform,
    l: &Localizer,
    fonts: Fonts,
    widgets: &DeckWidgets,
    bundle: &SourceBundle,
    deck: &Deck,
) -> Result<Rendered> {
    let themes = ThemeLibrary::new(Some(platform.settings.paths.themes()), platform.storage.clone());
    let tokens = theme_tokens(&themes, &bundle.themes, deck);
    let Some((worker, pack)) = &widgets.remote else { return local(l, fonts, bundle, deck, tokens) };
    let mut slot = worker.slot(&fonts, l, widgets.registry.clone())?;
    slot.set_widgets(Some(pack), widgets.registry.clone())?;
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

#[cfg(test)]
#[path = "pack_render_tests.rs"]
mod pack_render_tests;
