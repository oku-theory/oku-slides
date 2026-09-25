//! Validation sémantique d'un deck (au-delà de la désérialisation).

use std::collections::HashSet;

use crate::model::{Deck, ElementBody, SlideKind, Zone};

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
        let grid = s.effective_grid();
        let mut seen = HashSet::new();
        for l in s.walk() {
            let e = l.element;
            let issue = |m: String| Issue { slide: Some(s.id.clone()), element: Some(e.id.clone()), message: m };
            if !seen.insert(e.id.as_str()) {
                out.push(issue("identifiant d'élément en double".into()));
            }
            if e.id.is_empty() {
                out.push(issue("identifiant vide".into()));
            }
            // Seuls les éléments libres de premier niveau s'accrochent à la grille.
            if let Some(c) = e.cell
                && l.zone == Zone::Elements
                && l.parent.is_none()
                && !c.fits(grid)
            {
                out.push(issue("cellule hors de la grille".into()));
            }
            match e.frame {
                Some(f) if !(f.w > 0.0 && f.h > 0.0) => out.push(issue("cadre de taille nulle".into())),
                None if l.in_free_layout && e.cell.is_none() => out.push(issue("élément libre sans cadre".into())),
                _ => {}
            }
            if let ElementBody::Image { asset, .. } = &e.body
                && let Some(assets) = assets
                && !assets.contains(asset)
            {
                out.push(issue(format!("asset manquant : {asset}")));
            }
        }
    }
    out
}
