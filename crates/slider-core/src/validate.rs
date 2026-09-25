//! Validation sémantique d'un deck (au-delà de la désérialisation).

use std::collections::HashSet;

use slider_i18n::{Localize, Localizer, Message};

use crate::model::{Deck, ElementBody, SlideKind, Zone};

/// Un problème du deck : où, et quoi.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub slide: Option<String>,
    pub element: Option<String>,
    pub message: Message,
}

impl Localize for Issue {
    fn localize(&self, l: &Localizer) -> String {
        let message = self.message.localize(l);
        match (&self.slide, &self.element) {
            (Some(s), Some(e)) => format!("[{s}/{e}] {message}"),
            (Some(s), None) => format!("[{s}] {message}"),
            _ => message,
        }
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.localize(slider_i18n::english()))
    }
}

/// Vérifie le deck. `assets` : identifiants disponibles, si on veut les contrôler.
pub fn validate(deck: &Deck, assets: Option<&HashSet<String>>) -> Vec<Issue> {
    let mut out = Vec::new();
    let mut slide_ids = HashSet::new();
    for s in &deck.slides {
        let issue = |id| Issue { slide: Some(s.id.clone()), element: None, message: Message::new(id) };
        if s.id.is_empty() {
            out.push(issue("issue-empty-slide-id"));
        }
        if !slide_ids.insert(s.id.as_str()) {
            out.push(issue("issue-duplicate-slide-id"));
        }
        if s.kind == SlideKind::Chapter && s.title.as_deref().unwrap_or("").is_empty() {
            out.push(issue("issue-chapter-needs-title"));
        }
        if !s.blocks.is_empty() && !s.elements.is_empty() {
            out.push(issue("issue-blocks-and-elements"));
        }
        let grid = s.effective_grid();
        let mut seen = HashSet::new();
        for l in s.walk() {
            let e = l.element;
            let issue = |message| Issue { slide: Some(s.id.clone()), element: Some(e.id.clone()), message };
            if !seen.insert(e.id.as_str()) {
                out.push(issue(Message::new("issue-duplicate-element-id")));
            }
            if e.id.is_empty() {
                out.push(issue(Message::new("issue-empty-element-id")));
            }
            // Seuls les éléments libres de premier niveau s'accrochent à la grille.
            if let Some(c) = e.cell
                && l.zone == Zone::Elements
                && l.parent.is_none()
                && !c.fits(grid)
            {
                out.push(issue(Message::new("issue-cell-outside-grid")));
            }
            match e.frame {
                Some(f) if !(f.w > 0.0 && f.h > 0.0) => out.push(issue(Message::new("issue-empty-frame"))),
                None if l.in_free_layout && e.cell.is_none() => {
                    out.push(issue(Message::new("issue-free-without-frame")))
                }
                _ => {}
            }
            if let ElementBody::Image { asset, .. } = &e.body
                && let Some(assets) = assets
                && !assets.contains(asset)
            {
                out.push(issue(Message::new("issue-missing-asset").arg("asset", asset.as_str())));
            }
        }
    }
    out
}
