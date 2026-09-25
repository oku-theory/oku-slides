//! Graphiques du deck : chaque élément `chart` est rendu en SVG par
//! `slider-chart` et servi à Typst sous `/charts/<empreinte>.svg`.
//!
//! L'empreinte couvre les options, la taille, la police et les libellés : un
//! graphique inchangé n'est pas redessiné d'un rendu à l'autre.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::panic::AssertUnwindSafe;
use std::sync::{Mutex, PoisonError};

use serde_json::{Map, Value as Json};
use slider_chart::{ChartLabels, ChartOption, ChartRenderer, ChartTheme, ChartsRsRenderer, Size};
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
/// ensuite le dessin à l'échelle de son cadre.
const MIN_SIDE: f32 = 160.0;
const MAX_SIDE: f32 = 4000.0;

/// Au-delà, le graphique n'est pas dessiné : un cadre de repli le remplace.
const MAX_VALUES: usize = 20_000;
const MAX_SVG_BYTES: usize = 4 << 20;

/// Familles déjà données à charts-rs : son registre est global au processus
/// et chaque ajout y garde une copie de la police.
static REGISTERED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Police du texte courant de chaque thème (voir `typst/slider/themes`).
fn body_font(theme: Option<&str>) -> &'static str {
    match theme {
        Some("papier") => "Instrument Sans",
        Some("pro") => "Inter",
        _ => "Libertinus Serif",
    }
}

/// SVG des graphiques du deck courant.
#[derive(Debug, Default)]
pub(crate) struct Charts {
    files: HashMap<FileId, Bytes>,
    labels: ChartLabels,
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

    /// Rend les graphiques de `deck` (JSON) et note l'empreinte de chacun
    /// dans l'élément. Un graphique qui ne se dessine pas n'en a pas : Typst
    /// affiche alors un cadre à la place.
    pub(crate) fn attach(&mut self, deck: &mut Json, theme: Option<&str>, fonts: &Fonts) -> Result<(), RenderError> {
        let family = register_font(fonts, body_font(theme));
        let renderer = ChartsRsRenderer {
            theme: ChartTheme { font_family: family, ..ChartTheme::default() },
            labels: self.labels.clone(),
        };
        let mut wanted = HashSet::new();
        let mut failure = None;
        for_each_element(deck, &mut |e| {
            if e.get("kind").and_then(Json::as_str) != Some("chart") {
                return;
            }
            // Une empreinte d'un rendu précédent ne vaut plus rien.
            e.remove(SVG_FIELD);
            let Some(option) = e.get("option") else { return };
            let size = frame_size(e).unwrap_or(DEFAULT_SIZE);
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
        failure.map_or(Ok(()), Err)
    }
}

/// Donne à charts-rs la face normale de `family` (pour mesurer le texte),
/// une fois par processus. Sans cette police, celle de Typst par défaut.
fn register_font(fonts: &Fonts, family: &str) -> String {
    let fallback = body_font(None);
    let Some(face) = fonts.regular_face(family).or_else(|| fonts.regular_face(fallback)) else {
        return family.to_owned();
    };
    let (name, data) = face;
    let mut registered = REGISTERED.lock().unwrap_or_else(PoisonError::into_inner);
    let registered = registered.get_or_insert_with(HashSet::new);
    if !registered.contains(&name) {
        match slider_chart::register_fonts(&[&data[..]]) {
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

/// Taille du dessin : celle du cadre, ramenée dans [MIN_SIDE, MAX_SIDE].
fn frame_size(e: &Map<String, Json>) -> Option<Size> {
    let frame = e.get("frame")?;
    let side = |key: &str| {
        let v = frame.get(key)?.as_f64()? as f32;
        v.is_finite().then(|| v.clamp(MIN_SIDE, MAX_SIDE))
    };
    Some(Size { width: side("w")?, height: side("h")? })
}

fn fingerprint(option: &Json, size: Size, renderer: &ChartsRsRenderer) -> String {
    let mut h = DefaultHasher::new();
    option.to_string().hash(&mut h);
    size.width.to_bits().hash(&mut h);
    size.height.to_bits().hash(&mut h);
    renderer.theme.font_family.hash(&mut h);
    renderer.labels.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Chaque élément du deck (blocs, éléments libres, enfants de groupes).
fn for_each_element(deck: &mut Json, f: &mut impl FnMut(&mut Map<String, Json>)) {
    fn visit(list: Option<&mut Json>, f: &mut impl FnMut(&mut Map<String, Json>)) {
        let Some(Json::Array(items)) = list else { return };
        for item in items {
            if let Json::Object(e) = item {
                f(e);
                visit(e.get_mut("children"), f);
            }
        }
    }
    let Some(Json::Array(slides)) = deck.get_mut("slides") else { return };
    for slide in slides {
        visit(slide.get_mut("blocks"), f);
        visit(slide.get_mut("elements"), f);
    }
}

#[cfg(test)]
#[path = "charts_tests.rs"]
mod charts_tests;
