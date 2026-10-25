//! Écritures et langue du texte : la police d'un caractère dépend de son écriture.

use serde::Serialize;

use super::{Item, ItemKind, SourceDeck, TextBox};

/// Écriture d'un caractère, au sens des trois polices d'un run OOXML (`latin`, `ea`, `cs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Script {
    /// Latin, grec, cyrillique, symboles : la police `latin`.
    Latin,
    /// Chinois, japonais, coréen : la police `ea`.
    EastAsian,
    /// Arabe, hébreu, écritures indiennes, thaï… : la police `cs`.
    Complex,
}

impl Script {
    /// L'écriture du caractère `c`, par plages Unicode.
    #[must_use]
    pub fn of(c: char) -> Self {
        match u32::from(c) {
            0x1100..=0x11FF
            | 0x2E80..=0x2FDF
            | 0x3000..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7FF
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFFEF
            | 0x20000..=0x3FFFF => Self::EastAsian,
            0x0590..=0x0EFF | 0x1000..=0x109F | 0x1780..=0x17FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF => Self::Complex,
            _ => Self::Latin,
        }
    }

    /// `text` contient-il un caractère de cette écriture ?
    #[must_use]
    pub fn occurs_in(self, text: &str) -> bool {
        text.chars().any(|c| Self::of(c) == self)
    }
}

/// L'étiquette de langue `raw` (BCP 47, `lang` OOXML) ramenée à ce que Typst
/// accepte : langue de 2 ou 3 lettres en minuscules, puis l'écriture (4 lettres,
/// `Hant`) et la région (2 lettres, `TW`) si elles suivent ; `None` pour une
/// étiquette inutilisable (`x-none`, vide).
#[must_use]
pub fn lang_tag(raw: &str) -> Option<String> {
    let alpha = |p: &str, n: &[usize]| n.contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphabetic());
    let mut parts = raw.trim().split(['-', '_']);
    let lang = parts.next().filter(|l| alpha(l, &[2, 3]))?;
    let mut out = lang.to_ascii_lowercase();
    let mut script = true;
    for p in parts {
        if script && alpha(p, &[4]) {
            let (head, tail) = p.split_at(1);
            out.push('-');
            out.push_str(&head.to_ascii_uppercase());
            out.push_str(&tail.to_ascii_lowercase());
            script = false;
        } else if alpha(p, &[2]) {
            out.push('-');
            out.push_str(&p.to_ascii_uppercase());
            break;
        } else {
            break;
        }
    }
    Some(out)
}

impl SourceDeck {
    /// La langue du texte des slides ([`lang_tag`]) qui couvre le plus de
    /// caractères, la première rencontrée en cas d'égalité ; `None` sans texte
    /// à la langue lisible.
    #[must_use]
    pub fn language(&self) -> Option<String> {
        let mut tally: Vec<(String, usize)> = Vec::new();
        for s in &self.slides {
            count_items(&s.items, &mut tally);
        }
        let mut best: Option<(String, usize)> = None;
        for (lang, n) in tally {
            if best.as_ref().is_none_or(|(_, b)| n > *b) {
                best = Some((lang, n));
            }
        }
        best.map(|(lang, _)| lang)
    }
}

fn count_items(items: &[Item], tally: &mut Vec<(String, usize)>) {
    for it in items {
        match &it.kind {
            ItemKind::Text(tb) => count_text(tb, tally),
            ItemKind::Shape(s) => s.text.iter().for_each(|tb| count_text(tb, tally)),
            ItemKind::Table(t) => t.rows.iter().flatten().for_each(|c| count_text(&c.text, tally)),
            ItemKind::Group { items: kids, .. } => count_items(kids, tally),
            ItemKind::SmartArt(sa) => sa.nodes.iter().for_each(|n| count_text(&n.text, tally)),
            ItemKind::Picture(_) | ItemKind::Chart(_) | ItemKind::Opaque(_) => {}
        }
    }
}

fn count_text(tb: &TextBox, tally: &mut Vec<(String, usize)>) {
    for r in tb.paragraphs.iter().flat_map(|p| &p.runs) {
        let n = r.text.chars().filter(|c| !c.is_whitespace()).count();
        let Some(lang) = r.style.lang.as_deref().and_then(lang_tag).filter(|_| n > 0) else { continue };
        match tally.iter_mut().find(|(l, _)| *l == lang) {
            Some((_, total)) => *total += n,
            None => tally.push((lang, n)),
        }
    }
}

#[cfg(test)]
#[path = "lang_tests.rs"]
mod lang_tests;
