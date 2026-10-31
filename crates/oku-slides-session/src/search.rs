//! Recherche d'un texte dans les slides du document (bandeau de l'aide).
//!
//! Chaque champ texte de chaque slide (titres, paramètres, paragraphes des
//! éléments) est comparé sans casse ni accents. Les slides dont le titre
//! correspond viennent d'abord ; chacune rend la ligne trouvée, raccourcie
//! autour de la correspondance.

use oku_slides_i18n::fold;
use serde::Serialize;

use oku_slides_core::{FieldOwner, ParamType};

use crate::Session;
use crate::fields::slide_text;
use crate::text::interp;

/// Longueur maximale d'un extrait, en caractères (points de suspension non compris).
pub(crate) const SEARCH_EXCERPT: usize = 90;

/// Résultats rendus au plus.
const SEARCH_LIMIT: usize = 30;

/// Une slide où le texte cherché apparaît.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchHit<'a> {
    /// Id de la slide.
    pub slide: &'a str,
    /// Son titre, variables remplacées ; `None` : sans titre.
    pub title: Option<String>,
    /// La ligne du corps trouvée, raccourcie autour de la correspondance ;
    /// `None` : seul le titre correspond.
    pub excerpt: Option<String>,
}

impl Session {
    /// Les slides où `query` apparaît, titres d'abord ; vide pour une requête blanche.
    pub fn search(&self, query: &str) -> Vec<SearchHit<'_>> {
        let needle = fold(query.trim());
        if needle.is_empty() {
            return Vec::new();
        }
        let deck = self.deck();
        let mut hits: Vec<(bool, SearchHit<'_>)> = Vec::new();
        for slide in &deck.slides {
            let title = slide_text(deck, slide, &self.registry, "title").map(|t| interp(t, &deck.vars));
            let in_title = title.as_deref().is_some_and(|t| fold(t).contains(&needle));
            let lines = slide.text_fields(&self.registry).into_iter().flat_map(|tf| {
                let params = match tf.owner {
                    FieldOwner::Slide => &slide.params,
                    FieldOwner::Element(e) => &e.params,
                };
                let name = tf.param.name.as_str();
                let lines: Vec<String> = match (tf.param.bind.as_deref(), &tf.param.ty) {
                    (Some(key), _) => deck.params.field(key).map(str::to_owned).into_iter().collect(),
                    (None, ParamType::Rich) => params
                        .rich(name)
                        .unwrap_or_default()
                        .iter()
                        .map(|p| p.runs.iter().map(|r| r.t.as_str()).collect())
                        .collect(),
                    _ => params.text(name).map(str::to_owned).into_iter().collect(),
                };
                lines.into_iter().map(|l| interp(&l, &deck.vars))
            });
            // Le titre est aussi un champ de la slide : l'extrait n'en répète pas la ligne.
            let excerpt = lines.filter(|l| Some(l) != title.as_ref()).find_map(|line| excerpt(&line, &needle));
            if in_title || excerpt.is_some() {
                hits.push((in_title, SearchHit { slide: &slide.id, title, excerpt }));
            }
        }
        // Stable : à rang égal, l'ordre des slides.
        hits.sort_by_key(|(in_title, _)| !in_title);
        hits.into_iter().take(SEARCH_LIMIT).map(|(_, h)| h).collect()
    }
}

/// `line` raccourcie autour de `needle` (déjà replié), s'il y apparaît.
///
/// The line matches as a title does (`fold` of the whole line); the excerpt's position folds
/// char by char, so a context-dependent fold (Greek final sigma) may start it at the line's head.
fn excerpt(line: &str, needle: &str) -> Option<String> {
    if !fold(line).contains(needle) {
        return None;
    }
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= SEARCH_EXCERPT {
        return Some(line.to_owned());
    }
    let folded: Vec<String> = chars.iter().map(|c| fold(&c.to_string())).collect();
    let char_needle: String = needle.chars().map(|c| fold(&c.to_string())).collect();
    // Position en caractères de `line` : le repli peut changer la longueur d'un caractère (œ).
    let start = (0..chars.len()).find(|&i| folded[i..].concat().starts_with(&char_needle)).unwrap_or(0);
    let from = start.saturating_sub(SEARCH_EXCERPT / 3).min(chars.len() - SEARCH_EXCERPT);
    let to = from + SEARCH_EXCERPT;
    let body: String = chars[from..to].iter().collect();
    Some(format!("{}{}{}", if from > 0 { "…" } else { "" }, body.trim(), if to < chars.len() { "…" } else { "" }))
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod search_tests;
