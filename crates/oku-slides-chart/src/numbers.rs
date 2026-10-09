//! Format des nombres selon la langue du deck, et non celle de l'interface :
//! le rendu est le même pour chaque collaborateur.
//!
//! Séparateur décimal et séparateur des milliers, posés seulement à partir
//! de 10 000 (règle typographique française, appliquée à toutes les langues).
//! Négatifs : signe moins typographique « − » (U+2212) dans toutes les
//! langues, comme Typst. Langue inconnue ou absente : point décimal, pas de groupes.
//! Miroir Typst : `typst/oku-slides/src/numbers.typ` ; interface :
//! `ui/src/table/numbers.ts`.

use std::collections::HashSet;

use serde_json::Value as Json;

use crate::option::{ChartOption, format_number};

/// Espace fine insécable (français).
const NARROW_NBSP: char = '\u{202F}';
/// Espace insécable (polonais, ukrainien, russe).
const NBSP: char = '\u{00A0}';

/// Partie entière à partir de laquelle les milliers sont groupés.
const GROUP_FROM_DIGITS: usize = 5;

/// Signe moins typographique, celui que Typst écrit.
const MINUS: char = '\u{2212}';

/// Séparateur entre le nom et la valeur d'une étiquette (« {a}: {d} » des secteurs).
const NAME_VALUE: &str = ": ";

/// Unités qu'un moteur accole à un nombre (graduations « 22.2k », « 40% »).
const UNITS: [char; 5] = ['k', 'M', 'G', 'T', '%'];

/// Séparateurs d'une langue : décimal et milliers (`None` : pas de groupes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumberFormat {
    decimal: char,
    group: Option<char>,
}

impl Default for NumberFormat {
    /// Point décimal, pas de groupes : l'écriture des nombres sans langue.
    fn default() -> Self {
        Self { decimal: '.', group: None }
    }
}

impl NumberFormat {
    /// Format de la langue `lang` (étiquette BCP 47 : « fr », « de-CH », « pt_BR »).
    pub fn for_lang(lang: Option<&str>) -> Self {
        let subtag = lang.and_then(|l| l.split(['-', '_']).next()).unwrap_or_default().to_ascii_lowercase();
        let (decimal, group) = match subtag.as_str() {
            "fr" => (',', NARROW_NBSP),
            "en" | "ja" | "ko" | "zh" | "th" | "hi" | "bn" | "ar" => ('.', ','),
            "de" | "nl" | "it" | "es" | "pt" | "tr" | "vi" => (',', '.'),
            "pl" | "uk" | "ru" => (',', NBSP),
            _ => return Self::default(),
        };
        Self { decimal, group: Some(group) }
    }

    /// Un nombre : entier sans décimales, sinon l'écriture la plus courte.
    pub fn format(&self, n: f64) -> String {
        let plain = format_number(n);
        self.localize(&plain).unwrap_or(plain)
    }

    /// Réécrit un nombre écrit sans langue (`-12345.5`, `22.2k`) ; `None` si
    /// `plain` n'en est pas un.
    pub fn localize(&self, plain: &str) -> Option<String> {
        let (sign, rest) = plain.strip_prefix('-').map_or((None, plain), |r| (Some(MINUS), r));
        let (number, unit) = match rest.strip_suffix(UNITS) {
            Some(n) => (n, &rest[n.len()..]),
            None => (rest, ""),
        };
        let (int, frac) = number.split_once('.').map_or((number, None), |(i, f)| (i, Some(f)));
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        if !digits(int) || frac.is_some_and(|f| !digits(f)) {
            return None;
        }
        let mut out = String::with_capacity(plain.len() + int.len() / 3 * 3);
        out.extend(sign);
        match self.group {
            Some(g) if int.len() >= GROUP_FROM_DIGITS => {
                for (i, c) in int.chars().enumerate() {
                    if i > 0 && (int.len() - i) % 3 == 0 {
                        out.push(g);
                    }
                    out.push(c);
                }
            }
            _ => out.push_str(int),
        }
        if let Some(f) = frac {
            out.push(self.decimal);
            out.push_str(f);
        }
        out.push_str(unit);
        Some(out)
    }

    /// Un texte qui est un nombre, ou une étiquette « nom: nombre » dont seule
    /// la valeur est réécrite ; `None` sinon.
    fn localize_text(&self, text: &str) -> Option<String> {
        if let Some(local) = self.localize(text) {
            return Some(local);
        }
        let (name, value) = text.rsplit_once(NAME_VALUE)?;
        self.localize(value).map(|local| format!("{name}{NAME_VALUE}{local}"))
    }

    /// Réécrit les textes d'un SVG qui ne sont qu'un nombre (étiquettes de
    /// valeurs, graduations, catégories numériques) ou finissent par « : nombre »
    /// (secteurs) ; le reste est inchangé, comme les textes de `keep` (saisis
    /// en texte : « 75001 » reste tel quel).
    /// Sans langue, seul le signe des négatifs change.
    pub fn localize_svg(&self, svg: &str, keep: &HashSet<String>) -> String {
        let mut out = String::with_capacity(svg.len() + svg.len() / 64);
        let mut rest = svg;
        while let Some(start) = find_text_open(rest) {
            let Some(open_end) = rest[start..].find('>').map(|i| start + i + 1) else { break };
            if rest[..open_end].ends_with("/>") {
                out.push_str(&rest[..open_end]);
                rest = &rest[open_end..];
                continue;
            }
            let Some(close) = rest[open_end..].find("</text>").map(|i| open_end + i) else { break };
            out.push_str(&rest[..open_end]);
            let content = &rest[open_end..close];
            let trimmed = content.trim();
            match self.localize_text(trimmed).filter(|_| !keep.contains(trimmed)) {
                Some(local) => {
                    let lead = content.len() - content.trim_start().len();
                    out.push_str(&content[..lead]);
                    out.push_str(&local);
                    out.push_str(&content[lead + trimmed.len()..]);
                }
                None => out.push_str(content),
            }
            rest = &rest[close..];
        }
        out.push_str(rest);
        out
    }
}

/// Textes des options qui ressemblent à un nombre (catégories, noms,
/// indicateurs saisis en texte) : [`NumberFormat::localize_svg`] les laisse tels quels.
/// Les identifiants de lignes et de colonnes (`dataset.ids`, `dataset.columns`)
/// n'en sont pas : ils ne s'affichent pas.
pub(crate) fn numeric_texts(option: &ChartOption) -> HashSet<String> {
    fn visit(v: &Json, out: &mut HashSet<String>) {
        match v {
            Json::String(s) if NumberFormat::default().localize_text(s.trim()).is_some() => {
                out.insert(s.trim().to_owned());
            }
            Json::Array(items) => items.iter().for_each(|i| visit(i, out)),
            Json::Object(map) => map.values().for_each(|i| visit(i, out)),
            _ => {}
        }
    }
    let mut out = HashSet::new();
    if let Ok(mut v) = serde_json::to_value(option) {
        if let Some(dataset) = v.get_mut("dataset").and_then(Json::as_object_mut) {
            dataset.remove("ids");
            dataset.remove("columns");
        }
        visit(&v, &mut out);
    }
    out
}

/// Début de la prochaine balise `<text` (pas `<textPath`).
fn find_text_open(s: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(i) = s[from..].find("<text").map(|i| from + i) {
        if matches!(s.as_bytes().get(i + 5), Some(b' ' | b'>' | b'\n' | b'\t')) {
            return Some(i);
        }
        from = i + 5;
    }
    None
}

#[cfg(test)]
#[path = "numbers_tests.rs"]
mod numbers_tests;
