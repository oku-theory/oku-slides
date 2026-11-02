//! Sélecteur de thème de l'hôte : cartes, surface et choix.
//!
//! Ouvert par le raccourci « thème » de l'agent, sans tour. Chaque carte porte la
//! slide montrée rendue dans son thème (le deck entier rendu, sa page prise :
//! voir `theme_job`), ses pastilles et son nom. Les rendus sont bornés
//! (`Bounds`) : l'actif d'abord, puis au plus `THEMES_RENDERED` miniatures dans
//! `RENDER_BUDGET`, chacune sous `THUMB_BYTES` ; au-delà, en échec ou sur un deck
//! vide, une carte reste sans miniature. Ils se font hors du verrou de session
//! ([`crate::ThemeJob`]). Le hub garde les cartes : un choix ne rend rien.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use oku_slides_a2ui::{Action, ButtonIcon, Component, ComponentKind, TextVariant};
use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Namespace, is_hex_color};
use oku_slides_i18n::Localizer;
use serde_json::{Map, Value, json};
use snafu::OptionExt;

use crate::agent::clip;
use crate::agent::forms::{Place, Surface, lit, surface};
use crate::theme::default_namespace;
use crate::theme_closure::theme_closure;
use crate::theme_schema::{default_tokens, lookup};
use crate::{Mode, Result, Session};

/// Surface du sélecteur de thème de l'hôte.
pub const HOST_THEME_PICKER: &str = "host-theme-picker";
/// Évènement d'un choix de thème.
pub const THEME_EVENT: &str = "theme";
/// Clé du contexte d'un choix : l'espace de noms du thème.
pub(crate) const THEME_KEY: &str = "theme";
/// Miniatures rendues au plus, l'actif compris.
pub(crate) const THEMES_RENDERED: usize = 12;
/// Temps de rendu des miniatures autres que l'actif.
pub(crate) const RENDER_BUDGET: Duration = Duration::from_secs(4);
/// Taille maximale d'une miniature (SVG) ; miroir de `SVG_MAX` dans l'interface.
pub(crate) const THUMB_BYTES: usize = 512 * 1024;
/// Longueur maximale d'un nom de thème montré.
pub(crate) const NAME_CHARS: usize = 80;

/// Jetons montrés en pastilles, dans l'ordre ; puis la première couleur de `chart.palette`.
const SWATCH_NAMES: [&str; 4] = ["bg", "surface", "ink", "accent"];

/// Les bornes des miniatures : temps total et taille de chacune.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bounds {
    pub(crate) budget: Duration,
    pub(crate) thumb_bytes: usize,
}

impl Bounds {
    /// Les bornes de l'application.
    pub(crate) const DEFAULT: Self = Self { budget: RENDER_BUDGET, thumb_bytes: THUMB_BYTES };
}

/// Un thème offert par le sélecteur.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeCard {
    /// L'espace de noms du thème.
    pub namespace: Namespace,
    /// Son nom, coupé à 80 caractères.
    pub name: String,
    /// La slide que montre la miniature.
    pub slide: String,
    /// Couleurs hexadécimales de ses jetons.
    pub swatches: Vec<String>,
    /// La slide rendue dans ce thème ; absente au-delà des bornes ou en échec.
    pub svg: Option<Arc<str>>,
}

/// Les cartes du sélecteur et le thème actif du deck.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeChoices {
    /// Les thèmes de la bibliothèque, dans son ordre.
    pub cards: Vec<ThemeCard>,
    /// Le thème du deck, ou celui par défaut.
    pub active: Namespace,
}

/// La couleur hexadécimale de `value`, ou celle de la couleur qu'elle nomme.
fn colour(all: &Map<String, Value>, value: &Value) -> Option<String> {
    let s = value.as_str()?;
    let hex = if s.starts_with('#') { Some(s) } else { lookup(all, &["color".to_owned(), s.to_owned()])?.as_str() };
    hex.filter(|h| is_hex_color(h)).map(str::to_owned)
}

/// Les pastilles du calque de thème `tokens`, posé sur les défauts.
pub(crate) fn swatches(tokens: &Map<String, Value>) -> Vec<String> {
    let all = effective_tokens(default_tokens(), tokens);
    let named = SWATCH_NAMES.iter().filter_map(|n| lookup(&all, &["color".to_owned(), (*n).to_owned()]));
    let chart = lookup(&all, &["chart".to_owned(), "palette".to_owned()]).and_then(|v| v.get(0));
    named.chain(chart).filter_map(|v| colour(&all, v)).collect()
}

/// La surface : un titre, un `ThemeChoice` par carte, une croix pour fermer.
pub fn theme_picker(cards: &[ThemeCard], active: &Namespace, l: &Localizer) -> Surface {
    let ids: Vec<String> = (0..cards.len()).map(|i| format!("c{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut components = vec![
        Component::column("root", &["title", "row", "close"]),
        Component::text("title", l.tr("agent-surface-themes"), Some(TextVariant::H4)),
        Component::row("row", &refs),
        Component::button_builder("close", l.tr("agent-surface-close"), Action::named("dismiss"))
            .icon(ButtonIcon::Close)
            .build(),
    ];
    for (cid, card) in ids.iter().zip(cards) {
        components.push(Component::new(
            cid,
            ComponentKind::ThemeChoice {
                svg: card.svg.as_deref().map(str::to_owned),
                name: card.name.clone(),
                slide: card.slide.clone(),
                swatches: card.swatches.clone(),
                active: card.namespace == *active,
                action: Some(Action::event(THEME_EVENT, [(THEME_KEY, lit(json!(card.namespace.as_str())))])),
            },
        ));
    }
    surface(HOST_THEME_PICKER, Place::Popover, None, components, json!({}))
}

/// L'espace de noms du thème d'un choix ; `None` s'il manque ou n'en est pas un.
pub fn picked_theme(context: &Map<String, Value>) -> Option<Namespace> {
    context.get(THEME_KEY).and_then(Value::as_str).and_then(Namespace::parse)
}

impl Session {
    /// Vrai en mode deck : ni modèles, ni historique, ni aide.
    pub fn in_deck_mode(&self) -> bool {
        matches!(self.mode, Mode::Deck)
    }

    /// The theme the deck renders with.
    ///
    /// # Errors
    ///
    /// Fails if the deck has no theme and Papier, the default theme, is missing from the built-ins.
    pub fn active_theme(&self) -> Result<Namespace> {
        self.deck_theme().cloned().map_or_else(default_namespace, Ok)
    }

    /// Applies the chosen theme to the deck and notes it for the agent.
    ///
    /// Rejected outside deck mode or for an unknown theme; one undo step.
    ///
    /// # Errors
    ///
    /// Fails outside deck mode, for an unknown theme (`UnknownTheme`), or if the transaction is rejected.
    pub fn pick_theme(&mut self, ns: &Namespace) -> Result<()> {
        self.ensure_deck_mode()?;
        let name = self.themes.by_namespace(ns).map(|s| s.manifest.name);
        let name = name.context(crate::error::UnknownThemeSnafu { theme: ns.as_str() })?;
        self.set_theme(ns)?;
        self.note(format!("theme picked: {:?} ({ns})", clip(&name, NAME_CHARS)));
        Ok(())
    }

    /// Le calque du deck sous le thème `ns`, dans la fermeture que prendrait `set_theme`.
    ///
    /// `None` (journalisé) si le thème ou un parent a disparu entre la liste et la lecture.
    pub(crate) fn offered_layer(&self, ns: &Namespace) -> Option<Map<String, Value>> {
        let layer = || -> Result<Map<String, Value>> {
            let (pins, files) = theme_closure(&self.themes, ns)?;
            let files: HashMap<_, _> = files.into_iter().map(|f| (f.version(), f)).collect();
            let mut deck = self.deck().clone();
            deck.theme = Some(ns.clone());
            deck.themes = pins;
            let packs = |v: &oku_slides_core::PackVersion| self.doc.sources.packs.get(v);
            crate::deck_theme::compose_layer(
                &self.themes,
                packs,
                |v| files.get(v),
                &deck,
                crate::deck_theme::LayerReading::Lenient,
            )
        };
        layer().inspect_err(|e| tracing::warn!(theme = %ns, error = %e.one_line(), "calque du thème illisible")).ok()
    }
}

#[cfg(test)]
#[path = "theme_picker_tests.rs"]
mod theme_picker_tests;
