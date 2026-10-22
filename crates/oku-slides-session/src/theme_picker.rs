//! Sélecteur de thème de l'hôte : cartes, surface et choix.
//!
//! Ouvert par le raccourci « thème » de l'agent, sans tour. Chaque carte porte la
//! slide montrée rendue dans son thème (clone d'une seule slide : numéro de page
//! et sommaire peuvent différer du deck entier), ses pastilles et son nom. Les
//! rendus sont bornés (`Bounds`) : l'actif d'abord, puis au plus
//! `THEMES_RENDERED` miniatures dans `RENDER_BUDGET`, chacune sous `THUMB_BYTES` ;
//! au-delà, en échec ou sur un deck vide, une carte reste sans miniature. Le
//! verrou de session (celui de l'espace de travail dans l'app) est tenu pendant
//! les rendus. Le hub garde les cartes : un choix ne rend rien.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use oku_slides_a2ui::{Action, ButtonIcon, Component, ComponentKind, TextVariant};
use oku_slides_core::tokens::effective_tokens;
use oku_slides_core::{Deck, is_hex_color};
use oku_slides_i18n::Localizer;
use oku_slides_render::PackSet;
use serde_json::{Map, Value, json};
use snafu::ResultExt;
use uuid::Uuid;

use crate::agent::clip;
use crate::agent::forms::{Place, Surface, lit, surface};
use crate::error::RenderSnafu;
use crate::theme::DEFAULT_THEME;
use crate::theme_schema::{default_tokens, lookup};
use crate::{Mode, Result, Session};

/// Surface du sélecteur de thème de l'hôte.
pub const HOST_THEME_PICKER: &str = "host-theme-picker";
/// Évènement d'un choix de thème.
pub const THEME_EVENT: &str = "theme";
/// Clé du contexte d'un choix : l'id du thème.
pub(crate) const THEME_KEY: &str = "theme";
/// Miniatures rendues au plus, l'actif compris.
pub(crate) const THEMES_RENDERED: usize = 12;
/// Temps de rendu des miniatures autres que l'actif.
pub(crate) const RENDER_BUDGET: Duration = Duration::from_secs(4);
/// Taille maximale d'une miniature (SVG) ; miroir de `SVG_MAX` dans l'interface.
pub(crate) const THUMB_BYTES: usize = 512 * 1024;
/// Longueur maximale d'un nom de thème montré.
const NAME_CHARS: usize = 80;

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
    /// L'id du thème.
    pub id: Uuid,
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
    /// Bibliothèque d'abord, puis thèmes du deck seul.
    pub cards: Vec<ThemeCard>,
    /// Le thème du deck, ou celui par défaut.
    pub active: Uuid,
}

/// La couleur hexadécimale de `value`, ou celle de la couleur qu'elle nomme.
fn colour(all: &Map<String, Value>, value: &Value) -> Option<String> {
    let s = value.as_str()?;
    let hex = if s.starts_with('#') { Some(s) } else { lookup(all, &["color".to_owned(), s.to_owned()])?.as_str() };
    hex.filter(|h| is_hex_color(h)).map(str::to_owned)
}

/// Les pastilles des jetons propres `tokens`, posés sur les défauts.
pub(crate) fn swatches(tokens: &Map<String, Value>) -> Vec<String> {
    let all = effective_tokens(default_tokens(), tokens);
    let named = SWATCH_NAMES.iter().filter_map(|n| lookup(&all, &["color".to_owned(), (*n).to_owned()]));
    let chart = lookup(&all, &["chart".to_owned(), "palette".to_owned()]).and_then(|v| v.get(0));
    named.chain(chart).filter_map(|v| colour(&all, v)).collect()
}

/// La surface : un titre, un `ThemeChoice` par carte, une croix pour fermer.
pub fn theme_picker(cards: &[ThemeCard], active: Uuid, l: &Localizer) -> Surface {
    let ids: Vec<String> = (0..cards.len()).map(|i| format!("c{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut components = vec![
        Component::column("root", &["title", "row", "close"]),
        Component::text("title", l.tr("agent-surface-themes"), Some(TextVariant::H4)),
        Component::row("row", &refs),
        Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss"))
            .icon(ButtonIcon::Close),
    ];
    for (cid, card) in ids.iter().zip(cards) {
        components.push(Component::new(
            cid,
            ComponentKind::ThemeChoice {
                svg: card.svg.as_deref().map(str::to_owned),
                name: card.name.clone(),
                slide: card.slide.clone(),
                swatches: card.swatches.clone(),
                active: card.id == active,
                action: Some(Action::event(THEME_EVENT, [(THEME_KEY, lit(json!(card.id.to_string())))])),
            },
        ));
    }
    surface(HOST_THEME_PICKER, Place::Thread, None, components, json!({}))
}

/// L'id du thème d'un choix ; `None` s'il manque ou n'est pas un uuid.
pub fn picked_theme_id(context: &Map<String, Value>) -> Option<Uuid> {
    context.get(THEME_KEY).and_then(Value::as_str).and_then(|s| Uuid::parse_str(s).ok())
}

impl Session {
    /// Vrai en mode deck : ni modèles, ni historique, ni aide.
    pub fn in_deck_mode(&self) -> bool {
        matches!(self.mode, Mode::Deck)
    }

    /// Le thème avec lequel le deck se rend.
    pub fn active_theme(&self) -> Uuid {
        self.deck_theme().map_or(DEFAULT_THEME, |r| r.id)
    }

    /// Les cartes du sélecteur sur `slide` ; `None` hors du mode deck.
    ///
    /// `slide` absente ou inconnue : la première. Rend dans le worker, verrou de session tenu.
    pub fn theme_choices(&mut self, slide: Option<&str>) -> Result<Option<ThemeChoices>> {
        self.theme_choices_within(slide, Bounds::DEFAULT)
    }

    /// [`theme_choices`](Self::theme_choices) avec ces bornes.
    pub(crate) fn theme_choices_within(&mut self, slide: Option<&str>, bounds: Bounds) -> Result<Option<ThemeChoices>> {
        if !self.in_deck_mode() {
            return Ok(None);
        }
        let active = self.active_theme();
        let shown = self.one_slide_deck(slide);
        let shown_id = shown.as_ref().and_then(|d| d.slides.first()).map(|s| s.id.clone()).unwrap_or_default();
        let offered: Vec<(Uuid, String)> = self
            .theme_list
            .iter()
            .map(|t| (t.id, t.name.clone()))
            .chain(self.deck_only_themes().into_iter().map(|t| (t.id, t.name)))
            .collect();
        let tokens: HashMap<Uuid, Option<Map<String, Value>>> =
            offered.iter().map(|(id, _)| (*id, self.offered_tokens(*id))).collect();
        let svgs = match &shown {
            Some(deck) => self.thumbnails(deck, active, &offered, &tokens, bounds)?,
            None => HashMap::new(),
        };
        let cards = offered
            .into_iter()
            .map(|(id, name)| ThemeCard {
                id,
                name: clip(&name, NAME_CHARS),
                slide: shown_id.clone(),
                swatches: tokens.get(&id).and_then(Option::as_ref).map(swatches).unwrap_or_default(),
                svg: svgs.get(&id).cloned(),
            })
            .collect();
        Ok(Some(ThemeChoices { cards, active }))
    }

    /// Applique le thème choisi au deck et le note pour l'agent.
    ///
    /// Refusé hors du mode deck ou pour un thème inconnu ; une étape d'annulation.
    pub fn pick_theme(&mut self, id: Uuid) -> Result<()> {
        self.ensure_deck_mode()?;
        let name = self.theme_info(id)?.name;
        self.set_theme(id)?;
        self.note(format!("theme picked: {:?} ({id})", clip(&name, NAME_CHARS)));
        Ok(())
    }

    /// Les miniatures : l'actif d'abord, puis les autres dans l'ordre, dans les bornes.
    fn thumbnails(
        &mut self,
        deck: &Deck,
        active: Uuid,
        offered: &[(Uuid, String)],
        tokens: &HashMap<Uuid, Option<Map<String, Value>>>,
        bounds: Bounds,
    ) -> Result<HashMap<Uuid, Arc<str>>> {
        let set = self.usable_packs()?;
        let mut order: Vec<Uuid> = vec![active];
        order.extend(offered.iter().map(|(id, _)| *id).filter(|id| *id != active));
        order.truncate(THEMES_RENDERED);
        let deadline = Instant::now() + bounds.budget;
        let mut svgs = HashMap::new();
        for id in order {
            if id != active && Instant::now() >= deadline {
                break;
            }
            let Some(Some(theme)) = tokens.get(&id) else { continue };
            if let Some(svg) = self.thumbnail(&set, deck, theme, id, bounds.thumb_bytes) {
                svgs.insert(id, svg);
            }
        }
        Ok(svgs)
    }

    /// Un clone du deck réduit à `slide` (sinon à la première) ; `None` si le deck est vide.
    fn one_slide_deck(&self, slide: Option<&str>) -> Option<Deck> {
        let deck = self.deck();
        let keep = slide.and_then(|id| deck.slide(id)).or_else(|| deck.slides.first())?.clone();
        Some(Deck {
            version: deck.version,
            theme: deck.theme.clone(),
            params: deck.params.clone(),
            vars: deck.vars.clone(),
            tokens: deck.tokens.clone(),
            widgets: deck.widgets.clone(),
            layouts: deck.layouts.clone(),
            slides: vec![keep],
        })
    }

    /// Les jetons propres du thème `id` dans la version que prendrait `set_theme`.
    ///
    /// `None` (journalisé) si sa version a disparu entre la liste et la lecture.
    fn offered_tokens(&self, id: Uuid) -> Option<Map<String, Value>> {
        self.theme_tokens_of(id)
            .inspect_err(|e| tracing::warn!(theme = %id, error = %e.one_line(), "jetons du thème illisibles"))
            .ok()
    }

    /// La miniature du thème `id`, sous `max` octets ; `None` (journalisé) sinon.
    fn thumbnail(
        &mut self,
        set: &PackSet,
        deck: &Deck,
        tokens: &Map<String, Value>,
        id: Uuid,
        max: usize,
    ) -> Option<Arc<str>> {
        match self.thumbnail_svg(set, deck, tokens) {
            Ok(Some(svg)) if svg.len() <= max => Some(svg.into()),
            Ok(Some(svg)) => {
                tracing::warn!(theme = %id, bytes = svg.len(), "miniature du thème trop lourde : écartée");
                None
            }
            Ok(None) => {
                tracing::warn!(theme = %id, "miniature du thème : slide absente du rendu");
                None
            }
            Err(e) => {
                tracing::warn!(theme = %id, error = %e.one_line(), "miniature du thème impossible");
                None
            }
        }
    }

    /// Le SVG de l'unique slide de `deck` rendue avec `tokens` ; `None` si elle n'a pas de page.
    fn thumbnail_svg(&mut self, set: &PackSet, deck: &Deck, tokens: &Map<String, Value>) -> Result<Option<String>> {
        let Some(slide) = deck.slides.first().map(|s| s.id.clone()) else { return Ok(None) };
        let compiled = self.render_deck_preview(set, deck, Some(tokens))?;
        let Some(page) = compiled.layout().page_of(&slide) else { return Ok(None) };
        Ok(Some(compiled.svg(page - 1).context(RenderSnafu)?))
    }
}

#[cfg(test)]
#[path = "theme_picker_tests.rs"]
mod theme_picker_tests;
