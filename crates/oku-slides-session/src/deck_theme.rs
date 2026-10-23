//! Le thème du deck : ses versions embarquées dans ses sources font foi.
//!
//! Un deck épingle un thème par son espace de noms (`Deck.theme`) et toute la
//! fermeture de ses `extends` (`Deck.themes`, voir `theme_closure`). Un deck
//! sans thème (deck neuf, deck JSON de développement) épingle la fermeture de
//! Papier dès son chargement. Changer de thème (`set_theme`, op `SetTheme`)
//! embarque d'abord la fermeture voulue, prise dans la bibliothèque.
//!
//! Le rendu reçoit un calque (`/theme.json`) : les défauts de composant de
//! chaque pack épinglé, puis chaque thème de la chaîne linéarisée, le thème
//! du deck en dernier ([`oku_slides_core::tokens::theme_layer`]). Il se lit
//! dans les versions embarquées, jamais dans les courantes de la
//! bibliothèque ; une chaîne illisible (version absente, déjà signalée à la
//! lecture des sources) est journalisée une fois et rendue au thème par défaut.

use std::collections::{BTreeMap, HashSet};
use std::sync::{LazyLock, Mutex};

use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt, ensure};

use oku_slides_core::tokens::theme_layer;
use oku_slides_core::{
    Deck, Namespace, Op, PackFiles, PackManifest, PackVersion, SourceBundle, ThemePin, check_closure, linearize,
};
use oku_slides_render::{PackSet, RenderError};
use oku_slides_render_worker::{RemoteCompiled, RenderSlot};

use crate::error::{
    RenderSnafu, SourcesSnafu, ThemeClosureSnafu, ThemeIdMismatchSnafu, ThemeJsonSnafu, ThemeVersionMissingSnafu,
    theme_chain_error,
};
use crate::theme::{ThemeLibrary, ThemeManifest, default_namespace};
use crate::theme_closure::{Closure, repaired_closure, theme_closure};
use crate::{Mode, Result, Session, new_renderer};

/// `deck` épingle la fermeture `closure` du thème `ns`, dont `sources` embarque chaque version.
pub(crate) fn pin_closure(sources: &mut SourceBundle, deck: &mut Deck, ns: Namespace, closure: Closure) {
    let (pins, files) = closure;
    for f in files {
        sources.add_theme(f);
    }
    deck.theme = Some(ns);
    deck.themes = pins;
}

/// Un deck sans thème épingle la fermeture de Papier, dans sa version courante.
///
/// Elle est embarquée dans `sources`, sans étape d'annulation. Un deck qui a
/// déjà un thème le garde ; une version épinglée qui manque aux sources mais
/// que la bibliothèque a (même empreinte, même contenu) est embarquée. La
/// session et la CLI passent par ici.
pub fn adopt_default_theme(themes: &ThemeLibrary, sources: &mut SourceBundle) -> Result<()> {
    if sources.deck().theme.is_some() {
        let missing: Vec<ThemePin> =
            sources.deck().themes.values().filter(|p| !sources.themes.contains_key(&p.version)).cloned().collect();
        for pin in missing {
            if let Some(files) = themes.version(pin.id, &pin.version) {
                sources.add_theme(files);
            }
        }
        return Ok(());
    }
    let ns = default_namespace()?;
    let closure = theme_closure(themes, &ns)?;
    let mut deck = sources.deck().clone();
    pin_closure(sources, &mut deck, ns, closure);
    sources.apply_silent(deck).context(SourcesSnafu)?;
    Ok(())
}

/// Répare des épinglages qui ne sont plus exactement la fermeture du thème du deck.
///
/// Après une fusion de deux `set_theme` concurrents, le deck garde un thème
/// mais l'union des épinglages (`issue-theme-closure`). La fermeture de son
/// thème est refaite depuis les versions embarquées, sinon la bibliothèque, et
/// posée par une op `SetTheme` vérifiée par le cœur, sans étape d'annulation.
/// Vrai si le deck a changé. Une fermeture impossible à refaire (thème
/// introuvable, boucle) remonte en erreur : le deck reste tel quel.
pub fn repair_theme_closure(themes: &ThemeLibrary, sources: &mut SourceBundle) -> Result<bool> {
    let deck = sources.deck();
    if check_closure(deck.theme.as_ref(), &deck.themes).is_ok() {
        return Ok(false);
    }
    let (theme, closure) = match deck.theme.clone() {
        Some(ns) => {
            let closure = repaired_closure(themes, sources, &ns)?;
            (ns, closure)
        }
        None => {
            let ns = default_namespace()?;
            let closure = theme_closure(themes, &ns)?;
            (ns, closure)
        }
    };
    let (pins, files) = closure;
    for f in files {
        sources.add_theme(f);
    }
    let mut deck = sources.deck().clone();
    let op = Op::SetTheme { theme: Some(theme), themes: pins };
    oku_slides_core::apply_all(&mut deck, vec![op], sources.registry()).context(crate::error::OpSnafu)?;
    tracing::info!("épinglages des thèmes réparés après fusion");
    sources.apply_silent(deck).context(SourcesSnafu)?;
    Ok(true)
}

/// Résout les `SetTheme` de `ops` ; rend les fichiers des versions épinglées absentes de `embedded`.
///
/// `null` devient le thème par défaut ; sans épinglages, la fermeture du
/// thème est prise dans la bibliothèque `themes` (`theme_closure`). Chaque
/// version épinglée se lit dans `embedded`, sinon dans la bibliothèque
/// (absente : `ThemeVersionMissing`) ; une version d'un autre thème que son
/// épinglage est refusée (`ThemeIdMismatch`), de même qu'un épinglage dont l'espace de noms ou les
/// parents diffèrent de ceux de la version (`ThemeClosure`). Le cœur vérifie ensuite que les
/// épinglages sont exactement la fermeture. Les variantes de l'agent, qui
/// n'embarquent rien, passent par ici.
pub(crate) fn fill_op_closures(
    themes: &ThemeLibrary,
    embedded: &BTreeMap<PackVersion, PackFiles>,
    ops: &mut [Op],
) -> Result<Vec<PackFiles>> {
    let mut missing: BTreeMap<PackVersion, PackFiles> = BTreeMap::new();
    for op in ops {
        let Op::SetTheme { theme, themes: pins } = op else { continue };
        let ns = match theme {
            Some(ns) => ns.clone(),
            None => theme.insert(default_namespace()?).clone(),
        };
        let mut found: BTreeMap<PackVersion, PackFiles> = BTreeMap::new();
        if pins.is_empty() {
            let (closure, files) = theme_closure(themes, &ns)?;
            found.extend(files.into_iter().map(|f| (f.version(), f)));
            *pins = closure;
        }
        for (key, pin) in pins.iter() {
            let held = embedded.get(&pin.version).or_else(|| found.get(&pin.version)).cloned();
            let files = held
                .or_else(|| themes.version(pin.id, &pin.version))
                .context(ThemeVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
            let manifest = manifest_of(pin, &files)?;
            // L'épinglage recopie l'espace de noms et les parents de la version : un épinglage fourni par l'appelant s'y tient.
            ensure!(
                manifest.namespace == *key && manifest.extends == pin.extends,
                ThemeClosureSnafu { namespace: key.as_str() }
            );
            if !embedded.contains_key(&pin.version) {
                missing.insert(pin.version.clone(), files);
            }
        }
    }
    Ok(missing.into_values().collect())
}

/// Résout les `SetTheme` de `ops` ([`fill_op_closures`]) et embarque dans `sources` chaque version épinglée.
///
/// La session et l'import (le plan hi-fi pose Neutre sans épinglages) passent par ici.
pub(crate) fn embed_op_closures(themes: &ThemeLibrary, sources: &mut SourceBundle, ops: &mut [Op]) -> Result<()> {
    for f in fill_op_closures(themes, &sources.themes, ops)? {
        sources.add_theme(f);
    }
    Ok(())
}

/// Versions embarquées déjà signalées, pour le processus.
static BAD_EMBEDDED_SEEN: LazyLock<Mutex<HashSet<(String, String)>>> = LazyLock::new(Mutex::default);

/// Vrai la première fois que la version `version` du thème `ns` est signalée dans le processus.
fn first_bad_embedded_sighting(ns: &Namespace, version: &PackVersion) -> bool {
    BAD_EMBEDDED_SEEN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert((ns.as_str().to_owned(), version.hex().to_string()))
}

/// Le manifeste de `files`, s'il est bien une version du thème `pin.id`.
pub(crate) fn manifest_of(pin: &ThemePin, files: &PackFiles) -> Result<ThemeManifest> {
    let manifest = ThemeManifest::from_files(files).context(ThemeJsonSnafu { id: pin.id })?;
    ensure!(manifest.id == pin.id, ThemeIdMismatchSnafu { id: pin.id, found: manifest.id });
    Ok(manifest)
}

/// Comment le calque de thème traite une partie illisible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerReading {
    /// Rendu : une chaîne illisible (déjà signalée par les sources) est journalisée une fois et
    /// remplacée par celle du thème par défaut ; un pack illisible, déjà écarté du rendu, n'apporte rien.
    Lenient,
    /// Retrofit : un deck sans thème, une chaîne ou un pack illisible est une erreur, jamais les défauts.
    Strict,
}

/// Les jetons de chaque thème de la chaîne de `ns` épinglée par `pins`, le thème `ns` en dernier.
///
/// Chaque version se lit dans `embedded`, sinon la même dans la bibliothèque.
fn chain_of<'a>(
    themes: &ThemeLibrary,
    embedded: &impl Fn(&PackVersion) -> Option<&'a PackFiles>,
    pins: &BTreeMap<Namespace, ThemePin>,
    ns: &Namespace,
) -> Result<Vec<Map<String, Value>>> {
    let order = linearize(ns, pins).map_err(theme_chain_error)?;
    let mut chain = Vec::new();
    for n in &order {
        let pin = pins.get(n).context(crate::error::UnknownThemeSnafu { theme: n.as_str() })?;
        let files = embedded(&pin.version).cloned().or_else(|| themes.version(pin.id, &pin.version));
        let files = files.context(ThemeVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
        chain.push(manifest_of(pin, &files)?.tokens);
    }
    Ok(chain)
}

/// Les jetons de chaque thème de la chaîne de `ns` dans la bibliothèque, thème `ns` en dernier.
fn library_chain(themes: &ThemeLibrary, ns: &Namespace) -> Result<Vec<Map<String, Value>>> {
    let (pins, files) = theme_closure(themes, ns)?;
    let by_version: BTreeMap<PackVersion, PackFiles> = files.into_iter().map(|f| (f.version(), f)).collect();
    chain_of(themes, &|v: &PackVersion| by_version.get(v), &pins, ns)
}

/// Les défauts de composant de chaque pack épinglé par `deck`, par espace de noms.
///
/// Un pack dont la version manque ou dont le manifeste ne se lit pas : une
/// erreur en lecture stricte ; sinon il n'en donne pas (déjà écarté du rendu par `resolve_widgets`).
pub(crate) fn pack_defaults<'a>(
    packs: &impl Fn(&PackVersion) -> Option<&'a PackFiles>,
    deck: &Deck,
    reading: LayerReading,
) -> Result<Vec<(Namespace, Map<String, Value>)>> {
    let mut out = Vec::new();
    for (_, ns, pin) in deck.all_pins() {
        let read = || -> Result<Map<String, Value>> {
            let files = packs(&pin.version)
                .context(crate::error::PackVersionMissingSnafu { id: pin.id, version: pin.version.clone() })?;
            let manifest = PackManifest::from_files(files).context(crate::error::PackManifestSnafu { id: pin.id })?;
            Ok(manifest.component_defaults().context(crate::error::PackTokensSnafu { id: pin.id })?)
        };
        match (read(), reading) {
            (Ok(defaults), _) => out.push((ns.clone(), defaults)),
            (Err(e), LayerReading::Strict) => return Err(e),
            (Err(e), LayerReading::Lenient) => {
                tracing::debug!(pack = %ns, error = %e.one_line(), "pack sans défauts de composant lisibles : écarté du calque");
            }
        }
    }
    Ok(out)
}

/// Le calque de thème de `deck` : défauts de ses packs, puis sa chaîne de thèmes.
///
/// `packs` et `embedded` trouvent les fichiers d'une version embarquée. Une
/// partie illisible se traite selon `reading` ; sans thème, en lecture
/// souple : la chaîne du thème par défaut, dans la bibliothèque.
pub(crate) fn compose_layer<'a>(
    themes: &ThemeLibrary,
    packs: impl Fn(&PackVersion) -> Option<&'a PackFiles>,
    embedded: impl Fn(&PackVersion) -> Option<&'a PackFiles>,
    deck: &Deck,
    reading: LayerReading,
) -> Result<Map<String, Value>> {
    let pinned = deck.theme.as_ref().map(|ns| (ns, chain_of(themes, &embedded, &deck.themes, ns)));
    let chain = match (pinned, reading) {
        (Some((_, Ok(chain))), _) => chain,
        (Some((_, Err(e))), LayerReading::Strict) => return Err(e),
        (None, LayerReading::Strict) => {
            return crate::error::UnknownThemeSnafu { theme: crate::theme::PAPIER_NAMESPACE }.fail()?;
        }
        (Some((ns, Err(e))), LayerReading::Lenient) => {
            let version = deck.themes.get(ns).map(|p| &p.version);
            if version.is_none_or(|v| first_bad_embedded_sighting(ns, v)) {
                tracing::warn!(theme = %ns, error = %e.one_line(), "chaîne de thèmes du deck illisible : thème par défaut");
            }
            library_chain(themes, &default_namespace()?)?
        }
        (None, LayerReading::Lenient) => library_chain(themes, &default_namespace()?)?,
    };
    let defaults = pack_defaults(&packs, deck, reading)?;
    Ok(theme_layer(defaults.iter().map(|(ns, d)| (ns, d.clone())), chain.iter()))
}

/// Le calque de thème du deck de `sources` (voir [`Session::theme_layer`]), pour la CLI et l'import.
pub fn sources_theme_layer(themes: &ThemeLibrary, sources: &SourceBundle) -> Result<Map<String, Value>> {
    compose_layer(themes, |v| sources.packs.get(v), |v| sources.themes.get(v), sources.deck(), LayerReading::Lenient)
}

/// Comme [`sources_theme_layer`], toute partie illisible étant une erreur : le calque que mesure le retrofit.
pub(crate) fn strict_sources_layer(themes: &ThemeLibrary, sources: &SourceBundle) -> Result<Map<String, Value>> {
    compose_layer(themes, |v| sources.packs.get(v), |v| sources.themes.get(v), sources.deck(), LayerReading::Strict)
}

/// Rend `deck` dans `slot` avec le calque `layer` (posé seulement s'il change).
pub(crate) fn render_themed(
    slot: &mut RenderSlot,
    deck: &Deck,
    layer: &Map<String, Value>,
    assets: &BTreeMap<String, Vec<u8>>,
) -> std::result::Result<RemoteCompiled, RenderError> {
    slot.set_theme(Some(layer))?;
    slot.render(deck, assets)
}

impl Session {
    /// Rend `deck` avec le calque `layer` dans l'emplacement des aperçus.
    ///
    /// Variantes et thèmes ; l'emplacement est ouvert au premier appel ; `set` : les packs utilisables.
    pub(crate) fn render_deck_preview(
        &mut self,
        set: &PackSet,
        deck: &Deck,
        layer: &Map<String, Value>,
    ) -> Result<RemoteCompiled> {
        let renderer = match &mut self.variant_renderer {
            Some(r) => r,
            None => self.variant_renderer.insert(new_renderer(
                &self.worker,
                self.render_fonts.clone(),
                &self.localizer,
                set,
                &self.registry,
            )?),
        };
        Ok(render_themed(renderer, deck, layer, &self.doc.sources.assets).context(RenderSnafu)?)
    }

    /// Le thème du deck de l'utilisateur, même pendant l'édition d'un pack.
    pub(crate) fn deck_theme(&self) -> Option<&Namespace> {
        self.user_deck().theme.as_ref()
    }

    /// L'épinglage du thème du deck de l'utilisateur, même pendant l'édition d'un pack.
    pub(crate) fn deck_theme_pin(&self) -> Option<&ThemePin> {
        let deck = self.user_deck();
        deck.themes.get(deck.theme.as_ref()?)
    }

    /// Le deck prend le thème `ns` et sa fermeture, dans leur version courante, embarquées ; annulable.
    ///
    /// En mode pack, le brouillon du thème part de `ns` (`set_edited_theme`).
    pub fn set_theme(&mut self, ns: &Namespace) -> Result<()> {
        if matches!(self.mode, Mode::Pack { .. }) {
            return self.set_edited_theme(ns);
        }
        let label = self.localizer.tr("session-undo-theme");
        self.apply(&label, vec![Op::SetTheme { theme: Some(ns.clone()), themes: BTreeMap::new() }], None)
    }

    /// Résout les `SetTheme` de `ops` avant leur application (voir [`embed_op_closures`]).
    pub(crate) fn embed_op_themes(&mut self, ops: &mut [Op]) -> Result<()> {
        embed_op_closures(&self.themes, &mut self.doc.sources, ops)
    }

    /// Le manifeste de la version `pin`, embarquée dans le document montré ou
    /// dans le deck mis de côté ; `None` si elle n'y est pas.
    pub(crate) fn theme_manifest(&self, pin: &ThemePin) -> Option<ThemeManifest> {
        self.embedded_themes().find_map(|themes| manifest_of(pin, themes.get(&pin.version)?).ok())
    }

    /// Les fichiers de la version `pin` : embarqués (document montré, puis deck
    /// mis de côté), sinon dans la bibliothèque.
    pub(crate) fn theme_files(&self, pin: &ThemePin) -> Option<PackFiles> {
        let embedded = self
            .embedded_themes()
            .find_map(|themes| themes.get(&pin.version).filter(|files| manifest_of(pin, files).is_ok()));
        embedded.cloned().or_else(|| self.themes.version(pin.id, &pin.version))
    }

    /// Les versions de thème embarquées : document montré, puis deck mis de côté.
    pub(crate) fn embedded_themes(&self) -> impl Iterator<Item = &BTreeMap<PackVersion, PackFiles>> {
        [Some(&self.doc.sources.themes), self.stash_sources().map(|s| &s.themes)].into_iter().flatten()
    }

    /// Les sources du deck mis de côté (mode pack, historique).
    fn stash_sources(&self) -> Option<&SourceBundle> {
        match &self.mode {
            Mode::Deck | Mode::Help { .. } => None,
            Mode::Pack { stash, .. } | Mode::History { stash, .. } => Some(&stash.sources),
        }
    }

    /// Le calque de thème avec lequel se rend `deck`.
    ///
    /// Les défauts de composant de ses packs, puis sa chaîne de thèmes
    /// linéarisée, lue dans les versions embarquées (document montré, puis
    /// deck mis de côté). En mode pack, un deck sans thème (le deck de
    /// travail, ses aperçus) se rend sur la chaîne sous le brouillon
    /// (`Session::draft_chain`) : celle des parents de sa base, ou de
    /// l'intégré qu'étendra un enfant ; le brouillon est dans ses jetons.
    pub fn theme_layer(&self, deck: &Deck) -> Result<Map<String, Value>> {
        let stash = self.stash_sources();
        let packs = |v: &PackVersion| self.doc.sources.packs.get(v).or_else(|| stash?.packs.get(v));
        if let Mode::Pack { theme, .. } = &self.mode
            && deck.theme.is_none()
        {
            let chain = self.draft_chain(theme)?;
            let defaults = pack_defaults(&packs, deck, LayerReading::Lenient)?;
            return Ok(theme_layer(defaults.iter().map(|(ns, d)| (ns, d.clone())), chain.iter()));
        }
        let embedded = |v: &PackVersion| self.doc.sources.themes.get(v).or_else(|| stash?.themes.get(v));
        compose_layer(&self.themes, packs, embedded, deck, LayerReading::Lenient)
    }
}

#[cfg(test)]
#[path = "deck_theme_tests.rs"]
mod deck_theme_tests;
