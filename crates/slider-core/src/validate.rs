//! Validation sémantique d'un deck (au-delà de la désérialisation).

use std::collections::HashSet;

use crate::model::{Deck, Element, ElementBody, SlideKind};
use crate::ops::element_ids;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub slide: Option<String>,
    pub element: Option<String>,
    pub message: String,
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.slide, &self.element) {
            (Some(s), Some(e)) => write!(f, "[{s}/{e}] {}", self.message),
            (Some(s), None) => write!(f, "[{s}] {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

fn check_elements(
    slide: &str,
    list: &[Element],
    assets: Option<&HashSet<String>>,
    out: &mut Vec<Issue>,
    free: bool,
    grid: Option<&crate::model::Grid>,
) {
    for e in list {
        let issue = |m: String| Issue { slide: Some(slide.to_owned()), element: Some(e.id.clone()), message: m };
        if e.id.is_empty() {
            out.push(issue("identifiant vide".into()));
        }
        if let (Some(c), Some(g)) = (e.cell, grid)
            && !c.fits(g)
        {
            out.push(issue("cellule hors de la grille".into()));
        }
        match e.frame {
            Some(f) if !(f.w > 0.0 && f.h > 0.0) => out.push(issue("cadre de taille nulle".into())),
            None if free && e.cell.is_none() => out.push(issue("élément libre sans cadre".into())),
            _ => {}
        }
        match &e.body {
            ElementBody::Image { asset, .. } => {
                if let Some(assets) = assets
                    && !assets.contains(asset)
                {
                    out.push(issue(format!("asset manquant : {asset}")));
                }
            }
            ElementBody::Group { children, layout } => {
                let free_children = layout.kind == crate::model::GroupLayoutKind::Free;
                check_elements(slide, children, assets, out, free_children, None);
            }
            _ => {}
        }
    }
}

/// Vérifie le deck. `assets` : identifiants disponibles, si on veut les contrôler.
pub fn validate(deck: &Deck, assets: Option<&HashSet<String>>) -> Vec<Issue> {
    let mut out = Vec::new();
    let mut slide_ids = HashSet::new();
    for s in &deck.slides {
        let issue = |m: &str| Issue { slide: Some(s.id.clone()), element: None, message: m.to_owned() };
        if s.id.is_empty() {
            out.push(issue("identifiant de slide vide"));
        }
        if !slide_ids.insert(s.id.as_str()) {
            out.push(issue("identifiant de slide en double"));
        }
        if s.kind == SlideKind::Chapter && s.title.as_deref().unwrap_or("").is_empty() {
            out.push(issue("un chapitre doit avoir un titre"));
        }
        if !s.blocks.is_empty() && !s.elements.is_empty() {
            out.push(issue("une slide a soit des blocs, soit des éléments libres, pas les deux"));
        }
        let ids = element_ids(s);
        let mut seen = HashSet::new();
        for id in &ids {
            if !seen.insert(id) {
                out.push(Issue {
                    slide: Some(s.id.clone()),
                    element: Some(id.clone()),
                    message: "identifiant d'élément en double".into(),
                });
            }
        }
        check_elements(&s.id, &s.blocks, assets, &mut out, false, None);
        check_elements(&s.id, &s.elements, assets, &mut out, true, Some(&s.effective_grid()));
    }
    out
}
