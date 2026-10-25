//! Graphiques du deck : chaque élément dont le widget a un paramètre
//! `host("chart")` est rendu en SVG par `oku-slides-chart` et servi à Typst
//! sous `/charts/<empreinte>.svg`.
//!
//! L'empreinte couvre les options, la taille, le thème (police, palette,
//! tailles de texte), les libellés et l'écriture des nombres (langue du deck) : un graphique inchangé n'est pas
//! redessiné d'un rendu à l'autre.
//!
//! Un graphique est dessiné à la taille de sa boîte, jamais mis à l'échelle :
//! ses textes gardent la taille du thème. Hors cadre (dans le flux, sur une
//! cellule de la grille), seule la compilation donne cette taille : elle est
//! relevée dans les boîtes ([`Charts::set_measured`]) et le graphique redessiné.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::panic::AssertUnwindSafe;
use std::sync::{Mutex, PoisonError};

use oku_slides_chart::{ChartLabels, ChartOption, ChartRenderer, ChartTheme, ChartsRsRenderer, NumberFormat, Size};
use oku_slides_core::{HostType, WidgetRegistry};
use serde_json::{Map, Value as Json};
use typst::foundations::Bytes;
use typst::syntax::FileId;

use crate::error::RenderError;
use crate::fonts::Fonts;
use crate::world::file_id;

/// Champ ajouté à l'élément dans `/deck.json` : l'empreinte de son SVG.
pub(crate) const SVG_FIELD: &str = "chart-svg";

/// Taille d'un graphique sans cadre (placé par le layout), en pt : 16:9.
pub(crate) const DEFAULT_SIZE: Size = Size { width: 800.0, height: 450.0 };

/// Côtés du dessin, en pt. Sous ce minimum, charts-rs panique ; Typst met
/// alors le dessin à l'échelle de sa boîte.
const MIN_SIDE: f32 = 160.0;
const MAX_SIDE: f32 = 4000.0;

/// Écart (pt) entre la boîte et le dessin au-delà duquel il est redessiné.
const SIZE_TOLERANCE: f32 = 0.5;

/// Au-delà, le graphique n'est pas dessiné : un cadre de repli le remplace.
const MAX_VALUES: usize = 20_000;
const MAX_SVG_BYTES: usize = 4 << 20;

/// Familles déjà données à charts-rs : son registre est global au processus
/// et chaque ajout y garde une copie de la police.
static REGISTERED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Police des graphiques quand les jetons n'en donnent pas.
const DEFAULT_FONT: &str = "Libertinus Serif";

/// Tailles des textes des graphiques, en pt, tirées des jetons `size`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TextSizes {
    /// Axes, légende, étiquettes et sous-titre : le texte courant (`size.body`).
    pub(crate) body: f32,
    /// Titre du graphique (`size.lead`).
    pub(crate) title: f32,
}

/// Tailles de texte des jetons résolus ; aucune s'il manque `body` ou `lead`.
pub(crate) fn text_sizes(tokens: &Json) -> Option<TextSizes> {
    let size = |name: &str| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "f64 to f32 has no fallible conversion; out of range gives infinity, discarded right after"
        )]
        let v = tokens.get("size")?.get(name)?.as_f64()? as f32;
        (v.is_finite() && v > 0.0).then_some(v)
    };
    Some(TextSizes { body: size("body")?, title: size("lead")? })
}

/// SVG des graphiques du deck courant.
#[derive(Debug, Default)]
pub(crate) struct Charts {
    files: HashMap<FileId, Bytes>,
    labels: ChartLabels,
    /// Palette des séries, tirée des jetons du thème ; vide = celle par défaut.
    palette: Vec<String>,
    /// Première famille de `font.body` des jetons effectifs ; `None` = `DEFAULT_FONT`.
    font: Option<String>,
    /// Tailles de texte du thème ; aucune = celles par défaut du moteur.
    text: Option<TextSizes>,
    /// Taille de la boîte des graphiques sans cadre, relevée à la compilation.
    measured: HashMap<String, Size>,
    /// Taille du dessin de chaque graphique du deck courant, par élément.
    drawn: HashMap<String, Size>,
}

impl Charts {
    pub(crate) fn file(&self, id: FileId) -> Option<&Bytes> {
        self.files.get(&id)
    }

    /// Libellés des séries sans nom. Ils entrent dans l'empreinte : le
    /// prochain `attach` redessine les graphiques concernés.
    pub(crate) fn set_labels(&mut self, labels: ChartLabels) {
        self.labels = labels;
    }

    /// Palette des séries (jetons `chart.palette`). Elle entre dans
    /// l'empreinte ; `true` si elle a changé.
    pub(crate) fn set_palette(&mut self, palette: Vec<String>) -> bool {
        let changed = palette != self.palette;
        self.palette = palette;
        changed
    }

    /// Police des graphiques (jeton `font.body`). Elle entre dans
    /// l'empreinte ; `true` si elle a changé.
    pub(crate) fn set_font(&mut self, font: Option<String>) -> bool {
        let changed = font != self.font;
        self.font = font;
        changed
    }

    /// Tailles de texte (jetons `size`). Elles entrent dans l'empreinte ;
    /// `true` si elles ont changé.
    pub(crate) fn set_text_sizes(&mut self, text: Option<TextSizes>) -> bool {
        let changed = text != self.text;
        self.text = text;
        changed
    }

    pub(crate) fn palette(&self) -> &[String] {
        &self.palette
    }

    /// Boîtes compilées `(élément, largeur, hauteur)` en pt. Celles des
    /// graphiques dessinés à une autre taille sont retenues pour le prochain
    /// `attach` ; `true` s'il y en a.
    pub(crate) fn set_measured<'a>(&mut self, boxes: impl IntoIterator<Item = (&'a str, f64, f64)>) -> bool {
        let mut changed = false;
        for (id, w, h) in boxes {
            let Some(drawn) = self.drawn.get(id) else { continue };
            let Some(size) = clamped(w, h) else { continue };
            let off = |a: f32, b: f32| (a - b).abs() > SIZE_TOLERANCE;
            if off(size.width, drawn.width) || off(size.height, drawn.height) {
                self.measured.insert(id.to_owned(), size);
                changed = true;
            }
        }
        changed
    }

    /// Rend les graphiques de `deck` (JSON) et note l'empreinte de chacun
    /// dans l'élément. Un élément porte un graphique si son widget (dans
    /// `registry`) a un paramètre `host("chart")` : l'option est la valeur
    /// de ce paramètre. Un graphique qui ne se dessine pas n'a pas
    /// d'empreinte : Typst affiche alors un cadre à la place.
    pub(crate) fn attach(
        &mut self,
        deck: &mut Json,
        fonts: &Fonts,
        registry: &WidgetRegistry,
    ) -> Result<(), RenderError> {
        let family = register_font(fonts, self.font.as_deref().unwrap_or(DEFAULT_FONT));
        // Les nombres s'écrivent dans la langue du deck (`params.lang`).
        let lang = deck.get("params").and_then(|p| p.get("lang")).and_then(Json::as_str);
        let renderer = ChartsRsRenderer {
            theme: self.theme(family),
            labels: self.labels.clone(),
            numbers: NumberFormat::for_lang(lang),
        };
        let mut wanted = HashSet::new();
        let mut failure = None;
        let measured = &self.measured;
        let mut drawn = HashMap::new();
        for_each_element(deck, &mut |e| {
            // Une empreinte d'un rendu précédent ne vaut plus rien.
            e.remove(SVG_FIELD);
            let Some(param) = e.get("kind").and_then(Json::as_str).and_then(|k| chart_param(registry, k)) else {
                return;
            };
            let Some(option) = e.get("params").and_then(|p| p.get(param)) else { return };
            let element = e.get("id").and_then(Json::as_str).unwrap_or_default().to_owned();
            let size = drawing_size(e, measured.get(&element).copied());
            drawn.insert(element, size);
            let key = fingerprint(option, size, &renderer);
            let id = match file_id(&format!("/charts/{key}.svg")) {
                Ok(id) => id,
                Err(err) => {
                    failure.get_or_insert(err);
                    return;
                }
            };
            if let Entry::Vacant(slot) = self.files.entry(id) {
                match draw(&renderer, option, size) {
                    Ok(svg) => {
                        slot.insert(Bytes::from_string(svg));
                    }
                    Err(reason) => {
                        let element = e.get("id").and_then(Json::as_str).unwrap_or_default();
                        tracing::warn!(element, reason, "graphique non dessiné");
                        return;
                    }
                }
            }
            wanted.insert(id);
            e.insert(SVG_FIELD.to_owned(), Json::String(key));
        });
        self.files.retain(|id, _| wanted.contains(id));
        self.measured.retain(|id, _| drawn.contains_key(id));
        self.drawn = drawn;
        failure.map_or(Ok(()), Err)
    }
}

impl Charts {
    /// Thème du moteur : police `family`, palette et tailles du thème du deck.
    fn theme(&self, family: String) -> ChartTheme {
        let mut theme = ChartTheme { font_family: family, ..ChartTheme::default() };
        if !self.palette.is_empty() {
            theme.palette.clone_from(&self.palette);
        }
        if let Some(t) = self.text {
            theme.title_size = t.title;
            theme.subtitle_size = t.body;
            theme.legend_size = t.body;
            theme.axis_size = t.body;
            theme.label_size = t.body;
        }
        theme
    }
}

/// Donne à charts-rs la face normale de `family` (pour mesurer le texte),
/// une fois par processus. Sans cette police, celle de Typst par défaut.
fn register_font(fonts: &Fonts, family: &str) -> String {
    let fallback = DEFAULT_FONT;
    let Some(face) = fonts.regular_face(family).or_else(|| fonts.regular_face(fallback)) else {
        return family.to_owned();
    };
    let (name, data) = face;
    let mut registered = REGISTERED.lock().unwrap_or_else(PoisonError::into_inner);
    let registered = registered.get_or_insert_with(HashSet::new);
    if !registered.contains(&name) {
        match oku_slides_chart::register_fonts(&[&data[..]]) {
            Ok(()) => {
                registered.insert(name.clone());
            }
            Err(err) => tracing::warn!(family = name, error = %err, "police refusée par le moteur de graphiques"),
        }
    }
    name
}

/// Le SVG d'un graphique, ou pourquoi il n'est pas dessiné. Le moteur ne
/// doit pas pouvoir arrêter le rendu du deck : une panique devient une erreur.
fn draw(renderer: &ChartsRsRenderer, option: &Json, size: Size) -> Result<String, String> {
    let values = count_values(option);
    if values > MAX_VALUES {
        return Err(format!("{values} values (max {MAX_VALUES})"));
    }
    let option: ChartOption = serde_json::from_value(option.clone()).map_err(|e| e.to_string())?;
    let svg = std::panic::catch_unwind(AssertUnwindSafe(|| renderer.render(&option, size)))
        .map_err(|_| "chart engine panicked".to_owned())?
        .map_err(|e| e.to_string())?;
    if svg.len() > MAX_SVG_BYTES {
        return Err(format!("SVG of {} bytes (max {MAX_SVG_BYTES})", svg.len()));
    }
    Ok(svg)
}

/// Nombre de valeurs (feuilles JSON) des options.
fn count_values(v: &Json) -> usize {
    match v {
        Json::Array(items) => items.iter().map(count_values).sum(),
        Json::Object(map) => map.values().map(count_values).sum(),
        _ => 1,
    }
}

/// Taille du dessin : celle du cadre (sauf sur une cellule de la grille, qui
/// le remplace), sinon celle relevée à la compilation, sinon 16:9.
fn drawing_size(e: &Map<String, Json>, measured: Option<Size>) -> Size {
    let on_cell = e.get("cell").is_some_and(|c| !c.is_null());
    let framed = if on_cell { None } else { frame_size(e) };
    framed.or(measured).unwrap_or(DEFAULT_SIZE)
}

fn frame_size(e: &Map<String, Json>) -> Option<Size> {
    let frame = e.get("frame")?;
    clamped(frame.get("w")?.as_f64()?, frame.get("h")?.as_f64()?)
}

/// A size clamped to [`MIN_SIDE`, `MAX_SIDE`]; none if a side is not finite.
fn clamped(w: f64, h: f64) -> Option<Size> {
    let side = |v: f64| {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "f64 to f32 has no fallible conversion; out of range gives infinity, discarded right after"
        )]
        let v = v as f32;
        v.is_finite().then(|| v.clamp(MIN_SIDE, MAX_SIDE))
    };
    Some(Size { width: side(w)?, height: side(h)? })
}

fn fingerprint(option: &Json, size: Size, renderer: &ChartsRsRenderer) -> String {
    let mut h = DefaultHasher::new();
    option.to_string().hash(&mut h);
    size.width.to_bits().hash(&mut h);
    size.height.to_bits().hash(&mut h);
    serde_json::to_string(&renderer.theme).unwrap_or_default().hash(&mut h);
    renderer.labels.hash(&mut h);
    renderer.numbers.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Nom du paramètre `host("chart")` du widget `kind`, s'il en a un.
fn chart_param<'r>(registry: &'r WidgetRegistry, kind: &str) -> Option<&'r str> {
    registry.get(kind)?.host_param(HostType::Chart).map(|p| p.name.as_str())
}

/// Chaque élément du deck : tout objet d'un tableau des `params` d'une
/// slide ou d'un élément qui a un `id` et un `kind`, à toute profondeur de
/// slot.
pub(crate) fn for_each_element(deck: &mut Json, f: &mut impl FnMut(&mut Map<String, Json>)) {
    fn visit(params: &mut Json, f: &mut impl FnMut(&mut Map<String, Json>)) {
        let Json::Object(params) = params else { return };
        for value in params.values_mut() {
            let Json::Array(items) = value else { continue };
            for item in items {
                let Json::Object(e) = item else { continue };
                let element = e.get("id").is_some_and(Json::is_string) && e.get("kind").is_some_and(Json::is_string);
                if element {
                    f(e);
                    if let Some(params) = e.get_mut("params") {
                        visit(params, f);
                    }
                }
            }
        }
    }
    let Some(Json::Array(slides)) = deck.get_mut("slides") else { return };
    for params in slides.iter_mut().filter_map(|s| s.get_mut("params")) {
        visit(params, f);
    }
}

#[cfg(test)]
#[path = "charts_tests.rs"]
mod charts_tests;
