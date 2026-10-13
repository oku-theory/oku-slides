//! Le thème du deck : la version embarquée dans ses sources fait foi.
//!
//! Un deck sans thème (deck neuf, deck JSON de développement) prend la
//! version courante du thème par défaut, embarquée et référencée dès son
//! chargement. Changer de thème (`set_theme`, op `SetTheme`) embarque d'abord
//! la version voulue, prise dans la bibliothèque. Le rendu lit les jetons de
//! la version embarquée, jamais la courante de la bibliothèque ; une version
//! absente des sources (déjà signalée à la lecture) : le thème par défaut.

use std::collections::{BTreeMap, HashSet};
use std::sync::{LazyLock, Mutex};

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};
use uuid::Uuid;

use oku_slides_core::{Deck, Op, PackFiles, PackVersion, SourceBundle, ThemeRef};
use oku_slides_render::RenderError;
use oku_slides_render_worker::{RemoteCompiled, RenderSlot};

use crate::error::{SourcesSnafu, ThemeIdMismatchSnafu, ThemeJsonSnafu, ThemeVersionMissingSnafu, UnknownThemeSnafu};
use crate::theme::{DEFAULT_THEME, ThemeLibrary, ThemeManifest};
use crate::{Mode, Result, Session};

/// Un deck sans thème adopte la version courante du thème par défaut.
///
/// Elle est embarquée dans `sources` et référencée par le deck, sans étape
/// d'annulation. Un deck qui a déjà un thème le garde ; si sa version manque
/// aux sources mais que la bibliothèque l'a (même empreinte, même contenu),
/// elle est embarquée. La session et la CLI passent par ici.
pub fn adopt_default_theme(themes: &ThemeLibrary, sources: &mut SourceBundle) -> Result<()> {
    if let Some(r) = sources.deck().theme.clone() {
        if !sources.themes.contains_key(&r.version)
            && let Some(files) = themes.version(r.id, &r.version)
        {
            sources.add_theme(files);
        }
        return Ok(());
    }
    let current = themes.current_version(DEFAULT_THEME).context(UnknownThemeSnafu { id: DEFAULT_THEME })?;
    let version = sources.add_theme(current.files);
    let mut deck = sources.deck().clone();
    deck.theme = Some(ThemeRef { id: DEFAULT_THEME, version });
    sources.apply_silent(deck).context(SourcesSnafu)?;
    Ok(())
}

/// Versions embarquées déjà signalées, pour le processus.
static BAD_EMBEDDED_SEEN: LazyLock<Mutex<HashSet<(Uuid, String)>>> = LazyLock::new(Mutex::default);

/// Vrai la première fois que la version `r` est signalée dans le processus.
fn first_bad_embedded_sighting(r: &ThemeRef) -> bool {
    BAD_EMBEDDED_SEEN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert((r.id, r.version.hex().to_string()))
}

/// Le manifeste embarqué de `files` ; illisible ou d'un autre thème, il est
/// journalisé une fois par version et ignoré (le rendu retombe sur le thème par défaut).
fn embedded_manifest(r: &ThemeRef, files: &PackFiles) -> Option<ThemeManifest> {
    manifest_of(r, files)
        .inspect_err(|e| {
            if first_bad_embedded_sighting(r) {
                tracing::warn!(theme = %r.id, version = %r.version.hex(), error = %e, "thème embarqué ignoré");
            }
        })
        .ok()
}

/// Le manifeste de `files`, s'il est bien une version du thème `r.id`.
fn manifest_of(r: &ThemeRef, files: &PackFiles) -> Result<ThemeManifest> {
    let manifest = ThemeManifest::from_files(files).context(ThemeJsonSnafu { id: r.id })?;
    ensure!(manifest.id == r.id, ThemeIdMismatchSnafu { id: r.id, found: manifest.id });
    Ok(manifest)
}

/// La version courante du thème par défaut, en dernier recours.
fn default_manifest(themes: &ThemeLibrary) -> Option<ThemeManifest> {
    themes.current_version(DEFAULT_THEME).map(|s| s.manifest)
}

/// Les jetons du thème de `deck` : sa version embarquée dans `embedded`,
/// sinon la version courante du thème par défaut.
pub fn theme_tokens(
    themes: &ThemeLibrary,
    embedded: &BTreeMap<PackVersion, PackFiles>,
    deck: &Deck,
) -> Option<Map<String, Value>> {
    let own = deck.theme.as_ref().and_then(|r| embedded_manifest(r, embedded.get(&r.version)?));
    own.or_else(|| default_manifest(themes)).map(|m| m.tokens)
}

/// Rend `deck` dans `slot` avec les jetons `tokens` (posés seulement s'ils changent).
pub(crate) fn render_themed(
    slot: &mut RenderSlot,
    deck: &Deck,
    tokens: Option<&Map<String, Value>>,
    assets: &BTreeMap<String, Vec<u8>>,
) -> std::result::Result<RemoteCompiled, RenderError> {
    slot.set_theme(tokens)?;
    slot.render(deck, assets)
}

impl Session {
    /// Le thème du deck de l'utilisateur, même pendant l'édition des modèles.
    pub(crate) fn deck_theme(&self) -> Option<&ThemeRef> {
        self.user_deck().theme.as_ref()
    }

    /// Embarque la version `r` dans les sources du document, si elle n'y est pas.
    ///
    /// Elle vient de la bibliothèque ; absente : `ThemeVersionMissing`. Une
    /// version d'un autre thème que `r.id` est refusée (`ThemeIdMismatch`).
    pub(crate) fn embed_theme_from_library(&mut self, r: &ThemeRef) -> Result<()> {
        if let Some(files) = self.doc.sources.themes.get(&r.version) {
            return manifest_of(r, files).map(drop);
        }
        let files = self
            .themes
            .version(r.id, &r.version)
            .context(ThemeVersionMissingSnafu { id: r.id, version: r.version.clone() })?;
        manifest_of(r, &files)?;
        self.doc.sources.add_theme(files);
        Ok(())
    }

    /// Le deck prend le thème `id` dans sa version courante, embarquée ; annulable.
    ///
    /// En mode modèles, le brouillon du thème part de `id` (`set_edited_theme`).
    pub fn set_theme(&mut self, id: Uuid) -> Result<()> {
        if matches!(self.mode, Mode::Templates { .. }) {
            return self.set_edited_theme(id);
        }
        let version = self.themes.current(id).context(UnknownThemeSnafu { id })?;
        let label = self.localizer.tr("session-undo-theme");
        self.apply(&label, vec![Op::SetTheme { theme: Some(ThemeRef { id, version }) }], None)
    }

    /// Résout les `SetTheme` de `ops` avant leur application : `null` devient
    /// la version courante du thème par défaut, et chaque version visée est
    /// embarquée (en ajout seul).
    pub(crate) fn embed_op_themes(&mut self, ops: &mut [Op]) -> Result<()> {
        for op in ops {
            let Op::SetTheme { theme } = op else { continue };
            if theme.is_none() {
                let version = self.themes.current(DEFAULT_THEME).context(UnknownThemeSnafu { id: DEFAULT_THEME })?;
                *theme = Some(ThemeRef { id: DEFAULT_THEME, version });
            }
            if let Some(r) = theme.clone() {
                self.embed_theme_from_library(&r)?;
            }
        }
        Ok(())
    }

    /// Le manifeste de la version `r`, embarquée dans le document montré ou
    /// dans le deck mis de côté ; `None` si elle n'y est pas.
    pub(crate) fn theme_manifest(&self, r: &ThemeRef) -> Option<ThemeManifest> {
        self.embedded_themes().find_map(|themes| embedded_manifest(r, themes.get(&r.version)?))
    }

    /// Les fichiers de la version `r` : embarqués (document montré, puis deck
    /// mis de côté), sinon dans la bibliothèque — l'ordre de `theme_version`.
    pub(crate) fn theme_files(&self, r: &ThemeRef) -> Option<PackFiles> {
        let embedded = self
            .embedded_themes()
            .find_map(|themes| themes.get(&r.version).filter(|files| manifest_of(r, files).is_ok()));
        embedded.cloned().or_else(|| self.themes.version(r.id, &r.version))
    }

    /// Les versions de thème embarquées : document montré, puis deck mis de côté.
    fn embedded_themes(&self) -> impl Iterator<Item = &BTreeMap<PackVersion, PackFiles>> {
        let stash = match &self.mode {
            Mode::Deck | Mode::Help { .. } => None,
            Mode::Templates { stash, .. } | Mode::History { stash, .. } => Some(&stash.sources.themes),
        };
        [Some(&self.doc.sources.themes), stash].into_iter().flatten()
    }

    /// Les jetons avec lesquels se rend `deck` : ceux de sa version embarquée,
    /// sinon ceux du thème par défaut.
    ///
    /// En mode modèles, un deck sans thème est le brouillon du thème édité :
    /// il porte tous ses jetons et se rend sur les seuls défauts, comme le
    /// thème édité (un thème est partiel : les clés qu'il omet viennent des
    /// défauts, pas du thème par défaut).
    pub(crate) fn theme_tokens(&self, deck: &Deck) -> Option<Map<String, Value>> {
        if deck.theme.is_none() && matches!(self.mode, Mode::Templates { .. }) {
            return None;
        }
        let own = deck.theme.as_ref().and_then(|r| self.theme_manifest(r));
        own.or_else(|| default_manifest(&self.themes)).map(|m| m.tokens)
    }
}

#[cfg(test)]
#[path = "deck_theme_tests.rs"]
mod deck_theme_tests;
